
use std::collections::VecDeque;
use std::sync::{Arc};
use parking_lot::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::work_storage::*;


/*
 * Thread-safe queue for storing work items. Each enqueued item gets a timestamp
 * tag.
 */
pub struct BlockingQueue<T> {
    queue: (Mutex<VecDeque<TimestampedWorkItem<T>>>, Condvar),
    number_of_inserts: AtomicUsize
}

impl<T> BlockingQueue<T> {

    pub fn new() -> Arc<BlockingQueue<T>> {
        Arc::new(BlockingQueue {
            queue: (Mutex::new(VecDeque::<TimestampedWorkItem<T>>::new()), 
                    Condvar::new()),
            number_of_inserts: AtomicUsize::new(0)
        })
    }

    pub fn enqueue(&self, item: WorkItem<T>) -> u64 {
        let (mutex, cvar) = &self.queue;
        let mut queue = mutex.lock();
        let current = self.number_of_inserts.load(Ordering::SeqCst);
       
        queue.push_back(
            TimestampedWorkItem(item, current as u64));
        
        self.number_of_inserts.store(current + 1, Ordering::SeqCst);

        cvar.notify_one();
        return current as u64;
    }

    pub fn enqueue_timestamped(&self, item: TimestampedWorkItem<T>) {
        let (mutex, cvar) = &self.queue;
        mutex.lock().push_back(item);
        cvar.notify_one();
    }
    
    pub fn wait_and_dequeue(&self) -> TimestampedWorkItem<T> {
        let &(ref mutex, ref cvar) = &self.queue;
        let mut queue = mutex.lock();
        while queue.is_empty() {
            cvar.wait(&mut queue);
        }

        debug_assert!(queue.is_empty() == false);

        let popped = queue.pop_front();

        debug_assert!(popped.is_some());
       
        popped.unwrap()
    }
}

unsafe impl<T> Send for BlockingQueue<T> {}
unsafe impl<T> Sync for BlockingQueue<T> {}