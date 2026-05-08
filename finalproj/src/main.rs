mod dispatcher;
mod metrics;
mod monitor;
mod queue;
mod task;

use dispatcher::{CompletionMsg, WorkerPool, spawn_monitor};
use metrics::Metrics;
use queue::ScheduleMode;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use task::generate_tasks;

// These are the core settings for the simulation: 1000 tasks, 8 workers,
// and a fixed random seed so we get the same task list every time we run.
const NUM_WORKERS: usize = 8;
const TASK_COUNT: u64 = 1000;
const RANDOM_SEED: u64 = 42;

fn main() {
    // Generate all 1000 tasks upfront using the fixed seed.
    let mut rng = StdRng::seed_from_u64(RANDOM_SEED);
    let tasks = generate_tasks(TASK_COUNT, &mut rng);
    let total_tasks = tasks.len();

    println!("== FIFO simulation ==");
    println!("1000 tasks, 70% IO / 30% CPU, 8 workers, cap 100%");

    // Set up the channel. Workers send completed tasks and exit sentinels
    // through this. The main thread receives everything on the other end.
    let (tx, rx) = mpsc::channel::<CompletionMsg>();

    // Create the worker pool and spawn all 8 worker threads.
    let pool = Arc::new(WorkerPool::new(NUM_WORKERS, ScheduleMode::Fifo, tx));
    pool.spawn_workers();

    // Spawn the monitor thread. It records a snapshot every 10ms until we
    // set stop_flag to true at the end. We use an AtomicBool here because
    // it's a simple one-way signal — no need for a full Mutex for this.
    let stop_flag = Arc::new(AtomicBool::new(false));
    let monitor_handle = spawn_monitor(Arc::clone(&pool), Arc::clone(&stop_flag));

    let start = Instant::now();

    // Spawn the generator thread. It goes through the pre-built task list
    // and submits each one to the pool, sleeping between submissions to
    // simulate tasks arriving over time. The gap is scaled down by 10x so
    // the whole simulation finishes in a reasonable amount of time.
    {
        let pool_clone = Arc::clone(&pool);
        thread::spawn(move || {
            let mut last_arrival: u64 = 0;
            for task in tasks {
                let gap_ms = task.arrival_time_ms.saturating_sub(last_arrival);
                last_arrival = task.arrival_time_ms;
                let sleep_ms = (gap_ms / 10).min(3);
                if sleep_ms > 0 {
                    thread::sleep(Duration::from_millis(sleep_ms));
                }
                pool_clone.submit(task);
            }
            // Tell the workers that no more tasks are coming so they know
            // to drain the queue and exit when it's empty.
            pool_clone.signal_generation_done();
        });
    }

    // This is the completion collector — it runs on the main thread and
    // processes everything that comes back through the channel. For each
    // finished task it frees up the CPU budget, then saves the task. For
    // each WorkerExited sentinel it counts down until all 8 workers are done.
    let mut completed: Vec<task::Task> = Vec::with_capacity(total_tasks);
    let mut workers_exited = 0;

    loop {
        match rx.recv() {
            Ok(CompletionMsg::Done(task)) => {
                pool.task_finished(&task);
                completed.push(task);
            }
            Ok(CompletionMsg::WorkerExited) => {
                workers_exited += 1;
                if workers_exited == NUM_WORKERS {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let makespan = start.elapsed();

    // All workers are done. Stop the monitor thread and wait for it to
    // return its log, then call signal_shutdown as a safety net.
    stop_flag.store(true, Ordering::Relaxed);
    pool.signal_shutdown();
    let monitor_log = monitor_handle.join().expect("monitor thread panicked");

    // Write the full snapshot log to CSV.
    let csv_path = "monitor_log.csv";
    monitor_log.save_csv(csv_path);

    // Compute and print the final metrics.
    let m = Metrics::compute(&completed, makespan);
    m.print_clean(
        start.elapsed().as_millis(),
        &monitor_log,
        csv_path,
    );
}
