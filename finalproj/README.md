# Concurrent Task Dispatcher

A multi-threaded task scheduling simulation written in Rust.

---

## How to Build and Run

### Prerequisites
- Rust (stable) — install from https://rustup.rs
- Cargo (bundled with Rust)

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

### Expected output
```
== FIFO simulation ==
1000 tasks, 70% IO / 30% CPU, 8 workers, cap 100%

— results —
total runtime        : 38752 ms
makespan             : 38743 ms
tasks completed      : 1000  (IO=708, CPU=292)
avg wait time        : 9379.77 ms
avg turnaround time  : 9579.91 ms
max wait time        : 18517 ms
avg CPU usage        : 89.36 %
avg workers active   : 5.17 / 8
monitor samples      : 3778
monitor csv          : monitor_log.csv
```

A file called `monitor_log.csv` is also written to the working directory containing every 10 ms snapshot recorded during the run.

---

## Project Structure

```
src/
  main.rs         — entry point; wires all components together
  task.rs         — Task struct, TaskKind enum, workload generator
  queue.rs        — TaskQueue with CPU-budget enforcement and FIFO/Optimized modes
  dispatcher.rs   — WorkerPool, worker loop, monitor thread spawner
  monitor.rs      — MonitorLog, Snapshot struct, CSV export
  metrics.rs      — Metrics struct, result computation, output formatting
Cargo.toml
README.md
REPORT.md
```

---

## Design Summary

### Three concurrent components

| Component | Thread(s) | Role |
|---|---|---|
| Generator | 1 spawned thread | Produces 1000 tasks and submits them at ~20 ms intervals |
| Worker pool | 8 spawned threads | Pulls tasks from the queue and executes them |
| Monitor | 1 spawned thread | Records CPU %, active workers, and queue length every 10 ms |

The **completion collector** runs on the main thread. It receives finished tasks via an `mpsc` channel, returns their CPU cost to the shared budget, and counts worker exit sentinels to know when the run is done.

### Task specification

| Property | Value |
|---|---|
| Total tasks | 1000 |
| Distribution | 70 % IO, 30 % CPU |
| IO task CPU cost | 10 % |
| CPU task CPU cost | 35 % |
| Duration (both kinds) | 200 ms |
| Arrival interval | ~20 ms (15–25 ms jitter) |
| Random seed | 42 (reproducible) |

### CPU cap enforcement

The system enforces a 100 % CPU ceiling. Before dispatching any task the queue checks:

1. Is there a free worker slot? (`active_workers < 8`)
2. Would this task push `current_cpu` over 100 %?

If either check fails the task stays in the queue. When a task finishes, its CPU cost is subtracted from `current_cpu` and sleeping workers are woken via `Condvar::notify_all()`.

Example capacity combinations under the cap:
- 8 IO tasks running = 80 % CPU (fits)
- 2 CPU + 3 IO tasks running = 70 + 30 = 100 % CPU (fits exactly)
- 3 CPU tasks running = 105 % CPU (blocked — third task must wait)

### Scheduling policy — FIFO

The queue dispatches tasks in arrival order. `try_next_fifo()` checks only the front of the deque: if the front task fits within the remaining CPU budget it is dispatched; otherwise nothing is dispatched until a running task finishes and frees headroom.

The `Optimized` mode exists in `queue.rs` for future use (Experiment 2) but is not called in the current run.

### Synchronization

| Primitive | Where | Purpose |
|---|---|---|
| `Arc<(Mutex<SharedState>, Condvar)>` | `dispatcher.rs` | Protects queue, CPU budget, and flags |
| `mpsc::channel` | `dispatcher.rs` → `main.rs` | Completed tasks and exit sentinels flow worker → collector |
| `Arc<AtomicBool>` | `main.rs` → `dispatcher.rs` | Stop signal for the monitor thread |

All shared mutable state — the task queue, `current_cpu`, `active_workers`, `generation_done`, and `shutdown` — lives behind a single `Mutex`. Workers sleep on the `Condvar` and are woken on two events: a new task being enqueued, or a running task finishing and freeing CPU headroom.

### Monitor output

The monitor thread wakes every 10 ms and calls `pool.snapshot()` which briefly acquires the lock to read `current_cpu`, `active_workers`, and `queue.len()`. Each reading is stored as a `Snapshot` in a `Vec`. At the end of the run:
- Averages (CPU %, active workers) are computed from the full snapshot list and printed in the results block.
- The full snapshot list is written to `monitor_log.csv` with columns: `elapsed_ms, cpu_pct, active_workers, queue_len`.

### Clean shutdown sequence

1. Generator finishes submitting all 1000 tasks → calls `signal_generation_done()` → sets `generation_done = true` + `notify_all()`
2. Workers see `generation_done && queue.is_empty() && active_workers == 0` → send `WorkerExited` sentinel and exit
3. Collector counts 8 `WorkerExited` messages → stops the loop
4. Main sets `stop_flag = true` → monitor thread exits and returns its log
5. Main calls `signal_shutdown()` as a safety net for any late-waking workers

---

## Tool Use Disclosure

AI assistance (Claude) was used during development.

- **What it provided:** module layout, Condvar-based worker pool pattern, CPU budget accounting logic, monitor thread design, and output formatting.
- **Example accepted:** Using `Arc<AtomicBool>` as the monitor stop signal rather than a second `mpsc` channel — simpler for a one-way flag.
- **Example rejected / fixed:** An early suggestion stored CPU cost separately in the queue state; this was changed so `task_finished()` derives the cost directly from `task.kind`, ensuring the cost returned always matches the cost that was charged.

All design decisions, synchronization choices, and trade-offs are understood and owned by the author.
