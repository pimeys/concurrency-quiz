use crossbeam_deque::{Steal, Stealer, Worker as Queue};

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{park_timeout, spawn, JoinHandle},
    time::Duration,
};

pub struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
    job_queue: Queue<Job>,
    running: Arc<AtomicBool>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let job_queue = Queue::<Job>::new_fifo();
        let running = Arc::new(AtomicBool::new(true));

        let workers = (0..size)
            .map(|id| {
                let stealer = job_queue.stealer();
                let running = running.clone();

                spawn(move || {
                    let worker = Worker::new(id, stealer, running);
                    worker.do_work();
                })
            })
            .collect();

        Self {
            workers,
            job_queue,
            running,
        }
    }

    pub fn queue<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.job_queue.push(Box::new(f));
    }

    pub fn shutdown(self) {
        self.running.store(false, Ordering::Relaxed);

        for wrkr in self.workers.iter() {
            wrkr.thread().unpark();
        }

        for wrkr in self.workers.into_iter() {
            wrkr.join();
        }
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Job = Box<FnBox + Send + 'static>;

struct Worker {
    id: usize,
    stealer: Stealer<Job>,
    running: Arc<AtomicBool>,
}

impl Worker {
    fn new(id: usize, stealer: Stealer<Job>, running: Arc<AtomicBool>) -> Self {
        Self {
            id,
            stealer,
            running,
        }
    }

    fn do_work(&self) {
        while self.running.load(Ordering::Relaxed) {
            while let Steal::Success(job) = self.stealer.steal() {
                job.call_box();
            }

            park_timeout(Duration::from_millis(100));
        }

        println!("Exit worker {}", self.id);
    }
}
