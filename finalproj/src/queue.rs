use crate::task::{Task, TaskKind, CPU_CAP};
use std::collections::VecDeque;

// The queue supports two scheduling modes. Right now the simulation runs
// FIFO, which is Experiment 1. The Optimized mode is already written and
// ready to go for Experiment 2, it scans the queue smarter instead of
// just taking the front task every time.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleMode {
    Fifo,
    Optimized,
}

// This is the task queue, it sits between the generator and the workers.
// It's not just a simple list though. Before it hands any task to a worker,
// it checks two things: is there a free worker slot, and would this task
// push the CPU over 100%? If either check fails, the task stays in the
// queue and the worker goes back to sleep.
//
// current_cpu tracks the total CPU being used right now by all running tasks.
// active_workers tracks how many of the 8 workers are currently busy.
pub struct TaskQueue {
    inner: VecDeque<Task>,
    mode: ScheduleMode,
    pub current_cpu: f64,
    pub active_workers: usize,
    pub max_workers: usize,
}

impl TaskQueue {
    pub fn new(max_workers: usize, mode: ScheduleMode) -> Self {
        TaskQueue {
            inner: VecDeque::new(),
            mode,
            current_cpu: 0.0,
            active_workers: 0,
            max_workers,
        }
    }

    // Just adds the task to the back of the queue — arrival order.
    pub fn push(&mut self, task: Task) {
        self.inner.push_back(task);
    }

    // This is the main dispatch function. A worker calls this when it wants
    // the next task. It returns Some(task) if one is available and safe to
    // run, or None if the worker should go back to sleep and wait.
    pub fn try_next(&mut self) -> Option<Task> {
        // First check: are all 8 workers already busy?
        if self.active_workers >= self.max_workers {
            return None;
        }

        match self.mode {
            ScheduleMode::Fifo => self.try_next_fifo(),
            ScheduleMode::Optimized => self.try_next_optimized(),
        }
    }

    // FIFO dispatch — takes the task at the front of the queue, but only if
    // it fits within the remaining CPU budget. If the front task would push
    // current_cpu over 100%, we return None and wait for a running task to
    // finish and free up some headroom. This is the limitation of FIFO —
    // tasks behind the front have to wait even if they'd fit fine.
    fn try_next_fifo(&mut self) -> Option<Task> {
        if self.inner.is_empty() {
            return None;
        }
        let cost = self.inner[0].kind.cpu_cost();
        if self.current_cpu + cost <= CPU_CAP {
            let task = self.inner.pop_front().unwrap();
            self.current_cpu += cost;
            self.active_workers += 1;
            Some(task)
        } else {
            None
        }
    }

    // Optimized dispatch — instead of being stuck on the front task, this
    // scans the whole queue to find the best task that actually fits in the
    // remaining CPU budget. It scores tasks by priority first, then prefers
    // IO tasks over CPU tasks within the same priority level, since IO tasks
    // are cheaper (10% vs 35%) and leave more room for other workers to run.
    fn try_next_optimized(&mut self) -> Option<Task> {
        if self.inner.is_empty() {
            return None;
        }

        let remaining_cpu = CPU_CAP - self.current_cpu;

        // Lower score = picked first. IO gets kind_order 0, CPU gets 1,
        // so IO is preferred when priority is equal.
        let best_idx = self
            .inner
            .iter()
            .enumerate()
            .filter(|(_, t)| t.kind.cpu_cost() <= remaining_cpu)
            .min_by_key(|(_, t)| {
                let kind_order: u8 = if t.kind == TaskKind::Io { 0 } else { 1 };
                (t.priority, kind_order)
            })
            .map(|(i, _)| i);

        if let Some(idx) = best_idx {
            let task = self.inner.remove(idx).unwrap();
            self.current_cpu += task.kind.cpu_cost();
            self.active_workers += 1;
            Some(task)
        } else {
            None
        }
    }

    // Called when a task finishes. We subtract its CPU cost back out of the
    // budget and decrement the active worker count. After this, a sleeping
    // worker will be woken up to check if something can now be dispatched.
    pub fn task_finished(&mut self, kind: &TaskKind) {
        self.current_cpu = (self.current_cpu - kind.cpu_cost()).max(0.0);
        if self.active_workers > 0 {
            self.active_workers -= 1;
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
