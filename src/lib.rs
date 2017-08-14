//! Simple dependency-free threadpool based on code from 
//! The Rust Programming Language Book (Http Server example)

use std::thread;
use std::sync::{Arc, Mutex,mpsc};

trait FnBox {
    fn call(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call(self: Box<F>) {
        (*self)()
    }
}

type Func = Box<FnBox + Send + 'static>;

enum Message {
    Work(Func),
    Terminate,
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool with a `count` threads
    /// running in the background
    /// 
    /// # Example
    ///
    /// ```rust norun
    /// let pool = threadpool::ThreadPool::new(4);
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if `count` equals zero
    pub fn new(count: usize) -> ThreadPool {
        assert!(count > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));


        ThreadPool {
            workers: (0..count).map(|_| Worker::new(receiver.clone())).collect(),
            sender: sender,
        }
    }

    /// Execute a task on the ThreadPool
    /// 
    /// # Example
    ///
    /// ```rust
    /// // Create a ThreadPool with 4 threads running
    /// let pool = threadpool::ThreadPool::new(4);
    /// for i in 0..16 {
    ///     pool.execute(move || println!("threadpool!"))
    /// }
    /// ```
    pub fn execute<F: FnOnce() + Send + 'static>(&self, func: F) {
        self.sender.send(Message::Work(Box::new(func))).unwrap();
    }
}

impl Drop for ThreadPool {
    /// Stop the ThreadPool
    ///
    /// All currently running tasks will be completed first
    ///
    /// Automatically called when the ThreadPool falls out of scope
    fn drop(&mut self) {
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            
            loop {
                // We use Mutex::try_lock() because it does not block
                // Blocking here will keep one thread doing most of the work
                // for short functions
                if let Ok(message) = receiver.try_lock() {
                    if let Ok(message) = message.recv() {
                        match message {
                            Message::Work(x) => x.call(),
                            Message::Terminate => break, 
                        }
                    }
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, MutexGuard};
    use super::ThreadPool;
    #[test]
    fn thread_spawning() {
        let pool = ThreadPool::new(4);
        let count = Arc::new(Mutex::new(0));

        for i in 0..20 {
            let mut data = count.clone();

            // Share a mutex to mutable data, increment the value by 1     
            pool.execute(move || { 
                let lock = data.lock();
                match lock {
                    Ok(mut data) => {
                        *data += 1;
                    },
                    Err(_) => println!("locked"),
                };
            });
        }
        // wait for all jobs to finish
        drop(pool);
        // Make sure that all threads completed
        assert_eq!(*count.lock().unwrap(), 20);
    }
}
