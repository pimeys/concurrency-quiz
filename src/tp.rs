use crossbeam_deque::{Steal, Stealer, Worker as Queue};

use std::{
    thread::{park_timeout, spawn, JoinHandle},
    time::Duration,
};

pub struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
    job_queue: Queue<Job>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let job_queue = Queue::<Job>::new_fifo();

        let workers = (0..size)
            .map(|id| {
                let stealer = job_queue.stealer();

                info!("Start worker {}", id);

                spawn(move || {
                    let worker = Worker::new(id, stealer);
                    worker.do_work();
                })
            })
            .collect();

        Self { workers, job_queue }
    }

    pub fn queue<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        debug!("Queuing work...");
        self.job_queue.push(Box::new(f));
    }

    pub fn shutdown(self) {
        for wrkr in self.workers.iter() {
            wrkr.thread().unpark();
        }

        for wrkr in self.workers.into_iter() {
            wrkr.join().unwrap();
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
}

impl Worker {
    fn new(id: usize, stealer: Stealer<Job>) -> Self {
        Self { id, stealer }
    }

    fn do_work(&self) {
        loop {
            while let Steal::Success(job) = self.stealer.steal() {
                debug!("Opening a connection in worker {}", self.id);
                job.call_box();
                debug!("Closing a connection in worker {}", self.id);
            }

            park_timeout(Duration::from_millis(100));
        }
    }
}
