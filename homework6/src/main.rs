// use std::sync::{mpsc, Arc, Mutex};
// use std::thread;

// // Message enum
// enum Message {
//     NewJob(Job),
//     Terminate,
// }

// // Job type
// type Job = Box<dyn FnOnce() + Send + 'static>;

// // ThreadPool struct
// struct ThreadPool {
//     workers: Vec<Worker>,
//     sender: mpsc::Sender<Message>,
// }

// impl ThreadPool {
//     // Create a new ThreadPool
//     fn new(size: usize) -> ThreadPool {
//         assert!(size > 0);

//         // Create channel
//         let (sender, receiver) = mpsc::channel();

//         // Share receiver among workers
//         let receiver = Arc::new(Mutex::new(receiver));

//         let mut workers = Vec::with_capacity(size);

//         // Create workers
//         for id in 0..size {
//             workers.push(Worker::new(id, Arc::clone(&receiver)));
//         }

//         ThreadPool { workers, sender }
//     }

//     // Execute a job
//     fn execute<F>(&self, f: F)
//     where
//         F: FnOnce() + Send + 'static,
//     {
//         let job = Box::new(f);

//         self.sender
//             .send(Message::NewJob(job))
//             .expect("Failed to send job to worker");
//     }
// }

// // Clean shutdown
// impl Drop for ThreadPool {
//     fn drop(&mut self) {
//         println!("Sending terminate message to all workers...");

//         // Send terminate message to each worker
//         for _ in &self.workers {
//             self.sender.send(Message::Terminate).unwrap();
//         }

//         println!("Shutting down all workers...");

//         // Join all worker threads
//         for worker in &mut self.workers {
//             println!("Shutting down worker {}", worker.id);

//             if let Some(thread) = worker.thread.take() {
//                 thread.join().unwrap();
//             }
//         }
//     }
// }

// // Worker struct
// struct Worker {
//     id: usize,
//     thread: Option<thread::JoinHandle<()>>,
// }

// impl Worker {
//     fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
//         let thread = thread::spawn(move || {
//             loop {
//                 let message = receiver
//                     .lock()
//                     .unwrap()
//                     .recv()
//                     .unwrap();

//                 match message {
//                     Message::NewJob(job) => {
//                         println!("Worker {} got a job; executing.", id);
//                         job();
//                     }
//                     Message::Terminate => {
//                         println!("Worker {} received terminate signal.", id);
//                         break;
//                     }
//                 }
//             }
//         });

//         Worker {
//             id,
//             thread: Some(thread),
//         }
//     }
// }

// fn main() {
//     let pool = ThreadPool::new(4);

//     // Submit 10 tasks
//     for i in 1..=10 {
//         pool.execute(move || {
//             println!("Processing task {}", i);
//             thread::sleep(std::time::Duration::from_millis(500));
//             println!("Completed task {}", i);
//         });
//     }

//     println!("Main thread waiting for tasks to complete...");
// }

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::Rng;
use rand::thread_rng;


// Special value signaling termination
const TERMINATION_SIGNAL: i32 = -1;

fn main() {
    const ITEM_COUNT: usize = 20;

    // Create channel
    let (tx, rx) = mpsc::channel::<i32>();
    let rx = Arc::new(Mutex::new(rx));

    // Spawn 2 producers
    let mut producers = Vec::new();
    for id in 0..2 {
        let tx_clone = tx.clone();
        producers.push(thread::spawn(move || {
            producer(id, tx_clone, ITEM_COUNT / 2);
        }));
    }

    // Spawn 3 consumers
    let mut consumers = Vec::new();
    for id in 0..3 {
        let rx_clone = Arc::clone(&rx);
        consumers.push(thread::spawn(move || {
            consumer(id, rx_clone);
        }));
    }

    // Wait for producers to finish
    for p in producers {
        p.join().unwrap();
    }

    // Send termination signals (one per consumer)
    for _ in 0..3 {
        tx.send(TERMINATION_SIGNAL).unwrap();
    }

    // Wait for consumers to finish
    for c in consumers {
        c.join().unwrap();
    }

    println!("All items have been produced and consumed!");
}

// Producer function
fn producer(id: usize, tx: mpsc::Sender<i32>, item_count: usize) {
    let mut rng = thread_rng();

    for _ in 0..item_count {
        let value = rng.gen_range(1..=100);
        println!("Producer {id} produced {value}");
        tx.send(value).unwrap();
        thread::sleep(Duration::from_millis(100));
    }

    println!("Producer {id} finished producing.");
}

// Consumer function
fn consumer(id: usize, rx: Arc<Mutex<mpsc::Receiver<i32>>>) {
    loop {
        let value = rx.lock().unwrap().recv().unwrap();

        if value == TERMINATION_SIGNAL {
            println!("Consumer {id} received termination signal. Exiting.");
            break;
        }

        println!("Consumer {id} consumed {value}");
        thread::sleep(Duration::from_millis(150));
    }
}
