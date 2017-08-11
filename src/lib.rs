use std::thread;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};

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

pub struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(count: usize) -> ThreadPool {
        assert!(count > 0);

        let (sender, receiver) = mpsc::channel();

        //let (status_s, status_r) = mpsc::channel::<bool>();

        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(count);

        for id in 0..count {
            workers.push(Worker::new(receiver.clone()));
        }

        ThreadPool {
            workers: workers,
            sender: sender,
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, func: F) {
        self.sender.send(Message::Work(Box::new(func))).unwrap();
    }
}

impl Drop for ThreadPool {
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
            {
                let mut data = count.clone();
        
                pool.execute(move || { 
                    //::std::thread::sleep_ms(100); 
                    println!("{}", i);
                    let mut data = data.lock().unwrap();
                    println!("{:?}", data);
                    *data += 1;
                    //*c.lock().unwrap() += 1;
                    //*c += 1;
                    //*c += 1;
                });
            }

        }

        assert_eq!(*count.try_lock().unwrap(), 20);
        println!("All jobs submitted");
    }
}
