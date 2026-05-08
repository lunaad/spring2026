use std::time::Instant;

// A Snapshot is one reading taken by the monitor thread. Every 10ms we
// record the current time, how much CPU is being used, how many workers
// are active, and how long the queue is. All of these get saved to a Vec
// so we can compute averages at the end and write everything to a CSV file.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub elapsed_ms: f64,
    pub cpu_pct: f64,
    pub active_workers: usize,
    pub queue_len: usize,
}

// MonitorLog holds all the snapshots collected during the run.
// The start Instant is used to calculate elapsed_ms for each snapshot.
pub struct MonitorLog {
    pub snapshots: Vec<Snapshot>,
    pub start: Instant,
}

impl MonitorLog {
    pub fn new() -> Self {
        MonitorLog {
            snapshots: Vec::new(),
            start: Instant::now(),
        }
    }

    // Records one snapshot. Called by the monitor thread every 10ms.
    pub fn record(&mut self, cpu_pct: f64, active_workers: usize, queue_len: usize) {
        let elapsed_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        self.snapshots.push(Snapshot {
            elapsed_ms,
            cpu_pct,
            active_workers,
            queue_len,
        });
    }

    // Average CPU usage across all snapshots — this is what gets printed
    // in the results as "avg CPU usage".
    pub fn avg_cpu(&self) -> f64 {
        if self.snapshots.is_empty() {
            return 0.0;
        }
        self.snapshots.iter().map(|s| s.cpu_pct).sum::<f64>() / self.snapshots.len() as f64
    }

    // Average number of active workers across all snapshots — shows how
    // well the worker pool was being utilized during the run.
    pub fn avg_active_workers(&self) -> f64 {
        if self.snapshots.is_empty() {
            return 0.0;
        }
        self.snapshots
            .iter()
            .map(|s| s.active_workers as f64)
            .sum::<f64>()
            / self.snapshots.len() as f64
    }

    // The highest CPU reading recorded during the run.
    #[allow(dead_code)]
    pub fn peak_cpu(&self) -> f64 {
        self.snapshots
            .iter()
            .map(|s| s.cpu_pct)
            .fold(0.0_f64, f64::max)
    }

    // The elapsed time of the last snapshot — basically how long the
    // monitor was running.
    #[allow(dead_code)]
    pub fn total_time_ms(&self) -> f64 {
        self.snapshots
            .last()
            .map(|s| s.elapsed_ms)
            .unwrap_or(0.0)
    }

    // Prints a formatted summary — kept here for potential future use.
    #[allow(dead_code)]
    pub fn print_summary(&self, label: &str) {
        println!();
        println!("┌──────────────────────────────────────────────────────┐");
        println!("│  MONITOR SUMMARY — {:<33}│", label);
        println!("├──────────────────────────────────────────────────────┤");
        println!("│  Snapshots recorded   : {:>26}   │", self.snapshots.len());
        println!(
            "│  Total time working   : {:>22.2} ms   │",
            self.total_time_ms()
        );
        println!(
            "│  Avg CPU consumption  : {:>22.2} %    │",
            self.avg_cpu()
        );
        println!(
            "│  Peak CPU consumption : {:>22.2} %    │",
            self.peak_cpu()
        );
        println!(
            "│  Avg active workers   : {:>22.2}      │",
            self.avg_active_workers()
        );
        println!("└──────────────────────────────────────────────────────┘");
    }

    // Writes every snapshot to a CSV file so the data can be graphed or
    // analyzed externally. The file gets created in whatever directory
    // you run the program from.
    pub fn save_csv(&self, path: &str) {
        use std::fmt::Write as _;
        let mut out = String::from("elapsed_ms,cpu_pct,active_workers,queue_len\n");
        for s in &self.snapshots {
            writeln!(
                out,
                "{:.1},{:.1},{},{}",
                s.elapsed_ms, s.cpu_pct, s.active_workers, s.queue_len
            )
            .unwrap();
        }
        std::fs::write(path, out).unwrap_or_else(|e| eprintln!("CSV write error: {}", e));
    }

    // Prints a sample of the timeline to the terminal — every Nth snapshot
    // so you get a readable overview without printing all 3000+ rows.
    #[allow(dead_code)]
    pub fn print_timeline(&self, sample_every: usize) {
        println!();
        println!("  Timeline sample (every {}th snapshot):", sample_every);
        println!(
            "  {:>10}  {:>10}  {:>10}  {:>10}",
            "time(ms)", "cpu%", "workers", "queue"
        );
        println!(
            "  {:-<10}  {:-<10}  {:-<10}  {:-<10}",
            "", "", "", ""
        );
        for (i, s) in self.snapshots.iter().enumerate() {
            if i % sample_every == 0 {
                println!(
                    "  {:>10.1}  {:>10.1}  {:>10}  {:>10}",
                    s.elapsed_ms, s.cpu_pct, s.active_workers, s.queue_len
                );
            }
        }
        println!();
    }
}
