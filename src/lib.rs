use std::io::Error;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() -> Result<(), Error> + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();
        let receiver: Arc<Mutex<mpsc::Receiver<Message>>> = Arc::new(Mutex::new(receiver));

        let mut workers: Vec<Worker> = Vec::with_capacity(size);
        (0..size).for_each(|id| workers.push(Worker::new(id, Arc::clone(&receiver))));

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() -> Result<(), Error> + Send + 'static,
    {
        self.sender
            .send(Message::NewJob(Box::new(f)))
            .unwrap_or_else(|e| eprintln!("Error {} raised sending job to worker.", e));
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.workers.iter_mut().for_each(|worker| {
            worker.thread.take().map(|thread| {
                self.sender.send(Message::Terminate).unwrap_or_else(|_| {
                    eprintln!(
                        "Thread {} got error during sending terminate message",
                        worker.id
                    )
                });

                thread.join().unwrap_or_else(|_| {
                    eprintln!("Thread {} got error during shutting down", worker.id)
                });
            });
        })
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = match receiver.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    let guard = poisoned.into_inner();
                    eprintln!("Thread {} recovered from mutex poisoning: {:?}", id, *guard);
                    guard
                }
            }
            .recv()
            .unwrap();

            match message {
                Message::NewJob(job) => {
                    job().unwrap_or_else(|e| eprintln!("Thread {} get an error: {}", id, e));
                }
                Message::Terminate => break,
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
