use crate::monitor::MonitorLog;
use crate::queue::{ScheduleMode, TaskQueue};
use crate::task::Task;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// These are the two types of messages workers send back to the main thread
// through the channel. Done carries the finished task so we can record its
// timing data. WorkerExited is a sentinel — it's how the main thread knows
// a worker has shut down. Once all 8 workers send this, the run is over.
pub enum CompletionMsg {
    Done(Task),
    WorkerExited,
}

// This is all the shared state that the workers and the generator both need
// to access. It lives behind a Mutex so only one thread can touch it at a
// time. The queue holds the pending tasks and tracks CPU usage. The two
// boolean flags are used for the shutdown sequence.
struct SharedState {
    queue: TaskQueue,
    // The generator sets this to true when it's done submitting all 1000 tasks.
    generation_done: bool,
    // This is a safety net — set to true after all workers have exited.
    shutdown: bool,
}

impl SharedState {
    fn new(max_workers: usize, mode: ScheduleMode) -> Self {
        SharedState {
            queue: TaskQueue::new(max_workers, mode),
            generation_done: false,
            shutdown: false,
        }
    }
}

// The WorkerPool owns the shared state and is the main way everything
// communicates. It wraps the Mutex and Condvar together in an Arc so
// all 8 worker threads can share a reference to it. It also holds the
// channel sender so workers can report completed tasks back.
pub struct WorkerPool {
    state: Arc<(Mutex<SharedState>, Condvar)>,
    num_workers: usize,
    tx: std::sync::mpsc::Sender<CompletionMsg>,
}

impl WorkerPool {
    pub fn new(
        num_workers: usize,
        mode: ScheduleMode,
        tx: std::sync::mpsc::Sender<CompletionMsg>,
    ) -> Self {
        WorkerPool {
            state: Arc::new((
                Mutex::new(SharedState::new(num_workers, mode)),
                Condvar::new(),
            )),
            num_workers,
            tx,
        }
    }

    // Spawns all 8 worker threads. Each one gets a clone of the Arc so they
    // all share the same underlying state, and a clone of the channel sender
    // so they can report back when they finish a task.
    pub fn spawn_workers(&self) {
        for worker_id in 0..self.num_workers {
            let state_arc = Arc::clone(&self.state);
            let tx = self.tx.clone();
            thread::spawn(move || worker_loop(worker_id, state_arc, tx));
        }
    }

    // Called by the generator thread for each task. We stamp enqueued_at
    // right here so the wait time measurement starts the moment the task
    // enters the system. Then we push it onto the queue and call notify_all
    // to wake up any sleeping workers so they can try to dispatch it.
    pub fn submit(&self, mut task: Task) {
        task.enqueued_at = Some(Instant::now());
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.queue.push(task);
        cvar.notify_all();
    }

    // Called by the generator after it submits the last task. Sets the flag
    // and wakes all workers so they know to drain the queue and exit when
    // it's empty instead of waiting for more tasks that will never come.
    pub fn signal_generation_done(&self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.generation_done = true;
        cvar.notify_all();
    }

    // Hard shutdown — safety net in case any worker is still sleeping after
    // the collector is done. Sets the shutdown flag and wakes everyone.
    pub fn signal_shutdown(&self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.shutdown = true;
        cvar.notify_all();
    }

    // Called by the main thread each time a task finishes. Subtracts the
    // task's CPU cost from the budget and wakes all workers — because freeing
    // up CPU headroom might mean a task that was blocked can now be dispatched.
    // This notify_all here was actually a bug fix — without it, workers would
    // stay asleep even after CPU headroom opened up and the whole run would stall.
    pub fn task_finished(&self, task: &Task) {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.queue.task_finished(&task.kind);
        cvar.notify_all();
    }

    // The monitor thread calls this every 10ms to get a quick snapshot of
    // the current CPU usage, how many workers are active, and how long the
    // queue is. It briefly acquires the lock, reads the values, and releases it.
    pub fn snapshot(&self) -> (f64, usize, usize) {
        let (lock, _) = &*self.state;
        let state = lock.lock().unwrap();
        (
            state.queue.current_cpu,
            state.queue.active_workers,
            state.queue.len(),
        )
    }
}

// This is the function each of the 8 worker threads runs. It's a loop that
// keeps going until there's no more work to do. Inside the loop, the worker
// tries to get a task from the queue. If there's nothing available right now,
// it calls cvar.wait() which releases the lock and puts the thread to sleep
// until something wakes it up — either a new task arriving or CPU headroom
// opening up. This is more efficient than spinning and checking repeatedly.
fn worker_loop(
    _id: usize,
    state_arc: Arc<(Mutex<SharedState>, Condvar)>,
    tx: std::sync::mpsc::Sender<CompletionMsg>,
) {
    let (lock, cvar) = &*state_arc;

    loop {
        let task_opt = {
            let mut state = lock.lock().unwrap();

            loop {
                // If shutdown is set, exit immediately.
                if state.shutdown {
                    let _ = tx.send(CompletionMsg::WorkerExited);
                    return;
                }

                // Try to get a dispatchable task from the queue.
                if let Some(task) = state.queue.try_next() {
                    break Some(task);
                }

                // If generation is done and the queue is empty and no tasks
                // are still running, all work is complete — exit cleanly.
                if state.generation_done
                    && state.queue.is_empty()
                    && state.queue.active_workers == 0
                {
                    let _ = tx.send(CompletionMsg::WorkerExited);
                    return;
                }

                // Nothing to do right now — release the lock and sleep.
                // The thread will wake up when notify_all() is called.
                state = cvar.wait(state).unwrap();
            }
        };

        // We release the lock before actually executing the task. This is
        // important — if we held the lock during the 200ms sleep, no other
        // worker could dispatch anything while this one was running.
        if let Some(mut task) = task_opt {
            task.started_at = Some(Instant::now());

            // Simulate the task running by sleeping for its duration.
            thread::sleep(task.duration);

            task.finished_at = Some(Instant::now());

            // Send the completed task back to the collector on the main thread.
            let _ = tx.send(CompletionMsg::Done(task));
        }
    }
}

// Spawns the monitor thread. It wakes up every 10ms, grabs a snapshot from
// the pool, and records it. When the stop_flag AtomicBool gets set to true
// by the main thread, the monitor exits and returns its full log.
pub fn spawn_monitor(
    pool: Arc<WorkerPool>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
) -> thread::JoinHandle<MonitorLog> {
    thread::spawn(move || {
        let mut log = MonitorLog::new();
        while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(10));
            let (cpu, workers, queue_len) = pool.snapshot();
            log.record(cpu, workers, queue_len);
        }
        log
    })
}
