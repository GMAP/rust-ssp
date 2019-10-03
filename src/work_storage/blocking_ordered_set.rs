use crate::work_storage::*;
use std::collections::BTreeMap;
use std::sync::{Arc};
use parking_lot::{Mutex, Condvar};

pub struct BlockingOrderedSet<T> {
    storage: Mutex<BTreeMap<u64, TimestampedWorkItem<T>>>,
    new_item_notifier: Condvar,
}

impl<T> BlockingOrderedSet<T> {
    pub fn new() -> Arc<BlockingOrderedSet<T>> {
        Arc::new(BlockingOrderedSet {
            storage: Mutex::new(BTreeMap::<u64, TimestampedWorkItem<T>>::new()),
            new_item_notifier: Condvar::new(),
        })
    }

    pub fn enqueue(&self, item: TimestampedWorkItem<T>) {
        let mut queue = self.storage.lock();
        match item {
            TimestampedWorkItem(_, order) => queue.insert(order, item)
        };
        self.new_item_notifier.notify_one();
    }

    pub fn wait_and_remove(&self, item: u64) -> TimestampedWorkItem<T> {
        let mut storage = self.storage.lock();
        while (*storage).is_empty() || !(*storage).contains_key(&item) {
            self.new_item_notifier.wait(&mut storage);
        }
        let removed_item = storage.remove(&item);

        match removed_item {
            Some(value) => return value,
            None => { panic!("Condition variable waited until item was found, but removal failed") }
        }
    }
}

unsafe impl<T> Send for BlockingOrderedSet<T> {}
unsafe impl<T> Sync for BlockingOrderedSet<T> {}
