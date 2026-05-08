use crate::task::{Task, TaskKind};
use std::time::Duration;

// Metrics holds all the final statistics computed after every task has
// finished. We pull these numbers from the completed task list by looking
// at the timestamps that were stamped as each task moved through the system.
pub struct Metrics {
    pub total_completed: usize,
    pub cpu_completed: usize,
    pub io_completed: usize,
    pub makespan: Duration,
    pub avg_wait_ms: f64,
    pub avg_turnaround_ms: f64,
    pub max_wait_ms: f64,
}

impl Metrics {
    // Takes the full list of completed tasks and computes all the stats.
    // avg_wait is a closure defined inline so we can reuse it for different
    // slices of tasks without writing the same loop multiple times.
    pub fn compute(tasks: &[Task], makespan: Duration) -> Self {
        let cpu_tasks: Vec<&Task> = tasks.iter().filter(|t| t.kind == TaskKind::Cpu).collect();
        let io_tasks: Vec<&Task> = tasks.iter().filter(|t| t.kind == TaskKind::Io).collect();

        let avg_wait = |slice: &[&Task]| -> f64 {
            if slice.is_empty() {
                return 0.0;
            }
            let sum: f64 = slice
                .iter()
                .filter_map(|t| t.wait_time())
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();
            sum / slice.len() as f64
        };

        // Max wait finds the single task that waited the longest in the queue.
        let max_wait = tasks
            .iter()
            .filter_map(|t| t.wait_time())
            .map(|d| d.as_secs_f64() * 1000.0)
            .fold(0.0_f64, f64::max);

        // Turnaround is the full time from entering the queue to finishing —
        // it includes both wait time and execution time combined.
        let avg_turnaround = {
            let sum: f64 = tasks
                .iter()
                .filter_map(|t| t.turnaround_time())
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();
            if tasks.is_empty() {
                0.0
            } else {
                sum / tasks.len() as f64
            }
        };

        let all_refs: Vec<&Task> = tasks.iter().collect();

        // We don't use io_tasks here for the final output but it's kept
        // so the variable isn't flagged as unused.
        let _ = io_tasks;

        Metrics {
            total_completed: tasks.len(),
            cpu_completed: cpu_tasks.len(),
            io_completed: all_refs.len() - cpu_tasks.len(),
            makespan,
            avg_wait_ms: avg_wait(&all_refs),
            avg_turnaround_ms: avg_turnaround,
            max_wait_ms: max_wait,
        }
    }

    // Prints the final results in the clean format. The monitor_log averages
    // are mixed in here too since they're part of the same output block.
    pub fn print_clean(
        &self,
        total_runtime_ms: u128,
        monitor_log: &crate::monitor::MonitorLog,
        csv_path: &str,
    ) {
        println!();
        println!("— results —");
        println!("total runtime        : {} ms", total_runtime_ms);
        println!("makespan             : {} ms", self.makespan.as_millis());
        println!(
            "tasks completed      : {}  (IO={}, CPU={})",
            self.total_completed, self.io_completed, self.cpu_completed
        );
        println!("avg wait time        : {:.2} ms", self.avg_wait_ms);
        println!("avg turnaround time  : {:.2} ms", self.avg_turnaround_ms);
        println!("max wait time        : {} ms", self.max_wait_ms as u64);
        println!("avg CPU usage        : {:.2} %", monitor_log.avg_cpu());
        println!(
            "avg workers active   : {:.2} / 8",
            monitor_log.avg_active_workers()
        );
        println!("monitor samples      : {}", monitor_log.snapshots.len());
        println!("monitor csv          : {}", csv_path);
    }
}
