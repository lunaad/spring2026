use rand::rngs::StdRng;
use rand::Rng;
use std::time::{Duration, Instant};

// These are the CPU costs for each task type. IO tasks are lighter on the
// CPU at 10%, while CPU tasks are heavier at 35%. Both types run for the
// same 200ms duration though the difference is just how much CPU load
// they put on the system while they're running.
pub const IO_CPU_COST: f64 = 10.0;
pub const CPU_CPU_COST: f64 = 35.0;
pub const TASK_DURATION_MS: u64 = 200;

// This is the system-wide CPU cap. The queue will never let running tasks
// go over 100% combined. If adding a new task would push it over, that
// task has to wait.
pub const CPU_CAP: f64 = 100.0;

// A task is either IO-bound or CPU-bound. That distinction matters because
// it determines how much CPU budget the task uses while it's running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {  
    Io,
    Cpu,
}

impl TaskKind {
    // Returns how much CPU percentage this task type costs while running.
    pub fn cpu_cost(&self) -> f64 {
        match self {
            TaskKind::Io => IO_CPU_COST,
            TaskKind::Cpu => CPU_CPU_COST,
        }
    }
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskKind::Io => write!(f, "IO"),
            TaskKind::Cpu => write!(f, "CPU"),
        }
    }
}

// This is the Task struct — it's the main unit of work in the whole system.
// Every task that gets created, queued, dispatched, and completed is one of
// these. The three "at" fields at the bottom are timestamps we fill in as
// the task moves through the system — they're how we calculate wait time
// and turnaround time at the end.
#[derive(Debug, Clone)]
pub struct Task {
    #[allow(dead_code)]
    pub id: u64,
    pub arrival_time_ms: u64,
    pub kind: TaskKind,
    pub duration: Duration,
    // Priority goes from 1 to 5, where 1 is the highest priority.
    pub priority: u8,
    // Stamped when the task enters the queue.
    pub enqueued_at: Option<Instant>,
    // Stamped when a worker picks it up and starts running it.
    pub started_at: Option<Instant>,
    // Stamped when the worker finishes.
    pub finished_at: Option<Instant>,
}

impl Task {
    // Wait time = how long the task sat in the queue before a worker picked it up.
    pub fn wait_time(&self) -> Option<Duration> {
        match (self.enqueued_at, self.started_at) {
            (Some(e), Some(s)) => Some(s.duration_since(e)),
            _ => None,
        }
    }

    // Turnaround time = total time from entering the queue to fully finishing.
    pub fn turnaround_time(&self) -> Option<Duration> {
        match (self.enqueued_at, self.finished_at) {
            (Some(e), Some(f)) => Some(f.duration_since(e)),
            _ => None,
        }
    }
}

// This function generates all 1000 tasks upfront before the simulation
// starts. I use a fixed random seed (set in main.rs) so the task list
// is the same every time you run it — that makes results reproducible.
// The workload is 70% IO tasks and 30% CPU tasks, with arrivals spaced
// roughly 20ms apart to simulate a real stream of incoming work.
pub fn generate_tasks(count: u64, rng: &mut StdRng) -> Vec<Task> {
    let mut tasks = Vec::with_capacity(count as usize);
    let mut arrival_ms: u64 = 0;

    for id in 0..count {
        // Space arrivals ~20ms apart with a small random jitter each time.
        let gap: u64 = rng.gen_range(15..=25);
        arrival_ms += gap;

        // 70% of tasks are IO, 30% are CPU.
        let kind = if rng.gen_bool(0.70) {
            TaskKind::Io
        } else {
            TaskKind::Cpu
        };

        let priority: u8 = rng.gen_range(1..=5);

        tasks.push(Task {
            id,
            arrival_time_ms: arrival_ms,
            kind,
            duration: Duration::from_millis(TASK_DURATION_MS),
            priority,
            enqueued_at: None,
            started_at: None,
            finished_at: None,
        });
    }

    tasks
}
