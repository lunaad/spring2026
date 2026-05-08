# Design Report — Concurrent Task Dispatcher

## 1. Threads and Major Components

The program has four concurrent roles:

**Generator thread** (`main.rs`, spawned with `thread::spawn`)
Iterates over the pre-generated list of 1000 `Task` structs and calls `pool.submit(task)` for each one. Before submitting, it sleeps proportionally to the gap between consecutive `arrival_time_ms` values (scaled ÷10, capped at 3 ms) to simulate tasks arriving as a stream at roughly 20 ms intervals. After the last task it calls `pool.signal_generation_done()`.

**Worker threads** (`dispatcher.rs`, 8 threads spawned by `pool.spawn_workers()`)
Each worker runs `worker_loop()`. It acquires the shared `Mutex`, calls `queue.try_next()` to retrieve a dispatchable task, releases the lock, stamps `started_at`, sleeps for `task.duration` (200 ms) to simulate execution, stamps `finished_at`, and sends `CompletionMsg::Done(task)` through the `mpsc` channel. If no task is available the worker sleeps on the `Condvar`.

**Monitor thread** (`dispatcher.rs`, `spawn_monitor()`)
Wakes every 10 ms, calls `pool.snapshot()` to briefly read `current_cpu`, `active_workers`, and `queue.len()` under the lock, and appends a `Snapshot` to a `MonitorLog`. Stops when `stop_flag` (an `Arc<AtomicBool>`) is set to `true` by the main thread. Returns the completed `MonitorLog` via `join()`.

**Completion collector** (`main.rs`, runs on the main thread)
Drains the `mpsc::Receiver` in a loop. `CompletionMsg::Done(task)` — calls `pool.task_finished(&task)` to return the CPU cost to the shared budget and wake sleeping workers, then pushes the task into the `completed` vec. `CompletionMsg::WorkerExited` — increments a counter; when it reaches 8 the loop ends and metrics are computed.

---

## 2. Shared Data and How It Is Protected

All shared mutable state lives in `SharedState` behind a single `Arc<(Mutex<SharedState>, Condvar)>`:

| Field | Type | Purpose |
|---|---|---|
| `queue` | `TaskQueue` | Holds pending tasks plus `current_cpu` and `active_workers` counts |
| `generation_done` | `bool` | Set by generator when all tasks have been submitted |
| `shutdown` | `bool` | Safety-net flag; set after all workers have exited |

`TaskQueue` itself holds:
- `inner: VecDeque<Task>` — the pending task list
- `current_cpu: f64` — aggregate CPU % of all currently running tasks
- `active_workers: usize` — count of workers currently executing a task
- `max_workers: usize` — fixed at 8; used to check for free slots

Accessing any of these fields requires holding the `Mutex` guard. No field is accessed without it (except `stop_flag` which is an `AtomicBool` and needs no lock).

---

## 3. Where Channels Are Used and Why

One `std::sync::mpsc` channel is created in `main()`:

```
workers ──Done(task)──► collector (main thread)
workers ──WorkerExited► collector (main thread)
```

Each of the 8 worker threads holds a `Sender<CompletionMsg>` clone. The main thread holds the single `Receiver`. This carries two message kinds:

- `CompletionMsg::Done(Task)` — the finished task with all timing fields stamped
- `CompletionMsg::WorkerExited` — a sentinel the worker sends just before its thread returns

**Why a channel here instead of shared state?** Moving completed tasks back to the collector through a channel avoids putting the completed-task list behind the same `Mutex` as the queue. The collector can process finished tasks without ever contending with the workers on the dispatch lock.

---

## 4. Where Shared State Is Used and Why

The `Arc<(Mutex<SharedState>, Condvar)>` pair is used for everything that workers and the generator need to coordinate on:

- **Task queue** — workers must atomically check CPU headroom and dequeue a task together; a channel cannot provide this because the decision depends on reading `current_cpu` and the queue simultaneously.
- **CPU budget** (`current_cpu`) — must be updated atomically with dispatch and completion so the total never exceeds 100 %.
- **`generation_done` flag** — workers need to know when no more tasks will arrive so they can exit instead of waiting forever.
- **`shutdown` flag** — safety-net so any worker that wakes after the collector is done still exits cleanly.

The `Condvar` is paired with the `Mutex` so workers can sleep without spinning. Two events wake sleeping workers:
1. `pool.submit()` — a new task was enqueued
2. `pool.task_finished()` — a task finished and CPU headroom was freed

Both call `cvar.notify_all()` after updating state.

---

## 5. Scheduling Policy Implemented

**FIFO with CPU-cap enforcement** (`queue.rs`, `try_next_fifo()`).

The queue holds tasks in a `VecDeque<Task>` in the order they were submitted (arrival order). When a worker asks for the next task, `try_next_fifo()` does the following:

1. If `active_workers >= max_workers` (8): return `None` — no free slot.
2. If the queue is empty: return `None`.
3. Look at the front task. If `current_cpu + task.cpu_cost > 100.0`: return `None` — adding this task would breach the CPU cap.
4. Otherwise: pop it from the front, add its CPU cost to `current_cpu`, increment `active_workers`, return the task.

This is pure first-in-first-out ordering. The CPU cap check is the only reason a task at the front of the queue might not be dispatched immediately.

---

## 6. What Improved Because of This Policy

FIFO is simple and predictable. Every task is dispatched in the order it arrived, which means:

- No task can be skipped or deprioritised by the scheduler
- The behaviour is easy to reason about and verify — task N always runs before task N+1 (unless the CPU cap blocks it)
- It is straightforward to implement correctly with no edge cases around ordering

---

## 7. What Became Worse or More Complicated

The CPU cap check creates a **front-of-queue blocking problem**. If the front task is a CPU task (35 % cost) and `current_cpu` is already at 70 %, that task cannot run. But there may be IO tasks (10 % cost) behind it that would fit in the remaining 30 %. FIFO does not look past the front, so those IO tasks also wait — even though dispatching them would be safe and would keep workers busy.

In practice this means:
- Workers can go idle even when the queue is non-empty
- Average wait time is higher than it needs to be during CPU-heavy periods
- The CPU utilisation reading can drop below what the hardware is actually capable of

This is the motivation for the `Optimized` scheduling mode (kept in `queue.rs` for Experiment 2), which scans the queue for the best-fitting task rather than blocking on the front.

---

## 8. Concurrency Bug Encountered During Development

**Problem:** An early version called `cvar.notify_one()` (wake one worker) in `pool.submit()` but `cvar.notify_all()` was never called in `pool.task_finished()`. When a task finished and freed CPU headroom, sleeping workers were not woken. If all 8 workers happened to be sleeping at the same moment (which occurred when the CPU cap was nearly full and a burst of tasks arrived just as several workers finished), the program stalled: tasks sat in the queue, no worker woke up to process them, and the run never ended.

**Fix:** Changed `pool.task_finished()` to call `cvar.notify_all()` after updating `current_cpu` and `active_workers`. Now, every time headroom is freed, all sleeping workers re-evaluate `try_next()` and one of them picks up the newly-dispatchable task.

---

## 9. Where Starvation or Unfairness Could Still Happen

**FIFO front-blocking starvation:** If the front of the queue is always a CPU task and `current_cpu` stays near 65 % (two CPU tasks running), IO tasks can never get a turn even though they would fit. In the current 70/30 IO/CPU split this is unlikely to persist for long, but under a workload with more CPU tasks it becomes a real problem.

**No priority enforcement in FIFO mode:** Each `Task` has a `priority` field (1–5), but FIFO ignores it entirely. A low-priority task that arrives early will always run before a high-priority task that arrives later. This is intentional for FIFO but means the priority field has no effect in Experiment 1.

**Monitor thread lock contention:** The monitor acquires the same `Mutex` every 10 ms to read the snapshot. Under very high dispatch rates this adds contention. In practice 10 ms is long enough that this is negligible, but it is a latent fairness issue at higher throughput.

---

## 10. Metrics Collected and How They Are Computed

**Task-level metrics** (computed in `metrics.rs` from the completed task list):

| Metric | Computation |
|---|---|
| Total / IO / CPU completed | Count of finished tasks, filtered by `task.kind` |
| Makespan | Wall-clock duration from `Instant::now()` at run start to when the last `WorkerExited` is received |
| Avg wait time | Mean of `started_at − enqueued_at` across all completed tasks |
| Avg turnaround time | Mean of `finished_at − enqueued_at` across all completed tasks |
| Max wait time | Maximum single `started_at − enqueued_at` value |

**Monitor-level metrics** (computed in `monitor.rs` from the `Vec<Snapshot>`):

| Metric | Computation |
|---|---|
| Avg CPU usage | Mean of `snapshot.cpu_pct` across all snapshots |
| Avg workers active | Mean of `snapshot.active_workers` across all snapshots |
| Monitor samples | Count of snapshots (one per 10 ms interval) |

The full snapshot list is also saved to `monitor_log.csv` with columns `elapsed_ms, cpu_pct, active_workers, queue_len` for external analysis.

---

## 11. Experiment — FIFO Simulation

**Configuration:** 1000 tasks, 70 % IO / 30 % CPU, seed 42, 8 workers, 100 % CPU cap, FIFO dispatch, 200 ms task duration.

**Expected behaviour:** Under a balanced 70/30 IO/CPU load, FIFO performs reasonably well most of the time. IO tasks (10 % CPU) allow up to 8 to run simultaneously (80 % CPU), so the queue drains quickly during IO-heavy periods. When CPU tasks cluster at the front of the queue, dispatch stalls until running CPU tasks finish and free the 35 % they each hold. This produces the observed pattern: high average CPU usage (~89 %) with occasional idle dips, and wait times that are much longer than the 200 ms task duration because tasks queue up behind stalled fronts.

**Interpretation:** The large gap between avg wait time (~9379 ms) and task duration (200 ms) shows that most time is spent waiting, not executing. With 1000 tasks arriving over ~20 seconds and workers taking 200 ms each, the theoretical minimum total time is `1000 × 200 ms / 8 workers = 25 000 ms`. The actual makespan (~38 750 ms) is longer because FIFO cannot always keep all 8 workers busy — some workers sit idle when the front task is blocked by the CPU cap even though other tasks behind it would fit.
