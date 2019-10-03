use crate::*;
use crate::blocks::*;
use work_storage::{WorkItem, TimestampedWorkItem};
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicUsize};
use std::thread::JoinHandle;
use std::thread;
use work_storage::{BlockingQueue, BlockingOrderedSet};
use parking_lot::{Mutex};

//Public API: An output node, receives values and causes side effects
pub trait In<TInput, TCollected=()> {
    fn process(&mut self, input: TInput, order: u64) -> TCollected;
}


impl <TInput, TCollected, F> In<TInput, TCollected> for F where F: FnMut(TInput) -> TCollected {
    fn process(&mut self, input: TInput, _order: u64) -> TCollected {
        (*self)(input)
    }
}

//Internals: InBlock processing queue for blocks in the pipeline
pub struct InBlock<TInput, TCollected> {
    work_queue: Arc<BlockingQueue<TInput>>,
    ordered_work: Arc<BlockingOrderedSet<TInput>>,
    collected_items: Arc<Mutex<Vec<TCollected>>>,
    handler: Box<FnMut() -> Box<dyn In<TInput, TCollected>>>,
    ordering: OrderingMode,
    counter: AtomicUsize
}

// Internals: This is a thread-local object for in blocks
struct InBlockInfo<TInput, TCollected> {
    handler: Box<dyn In<TInput, TCollected>>
}


impl <TInput, TCollected> PipelineBlock<TInput, TCollected> for InBlock<TInput, TCollected> {

    //used by the public API
    fn process(&self, input: WorkItem<TInput>) {
        match self.ordering {
            //For the unordered case, just enqueue it
            OrderingMode::Unordered => {
                (*self.work_queue).enqueue(input);
            },
            //For the ordered case: All InBlocks are single threaded
            //so we keep a count. Store under an atomic counter
            //in case we implement a multithreaded outblock
            OrderingMode::Ordered => {
                let c = self.counter.load(Ordering::SeqCst);
                (*self.ordered_work).enqueue(TimestampedWorkItem(input, c as u64));
                self.counter.store(c + 1, Ordering::SeqCst);
            }
        };
        ()
    }

    //Used internally
    fn process_timestamped(&self, input: TimestampedWorkItem<TInput>) {
        match self.ordering {
            OrderingMode::Unordered => match input {
                TimestampedWorkItem(work_item, _) => {
                    (*self.work_queue).enqueue(work_item);
                }
            },
            OrderingMode::Ordered => (*self.ordered_work).enqueue(input)
        };
    }

    fn collect(self: Box<Self>) -> Vec<TCollected> {
        match Arc::try_unwrap(self.collected_items) {
            Ok(result) => result.into_inner(),
            Err(_) => {
                panic!("Could not unwrap Arc in call to collect");
            }
        }
    }
}



impl<TInput: 'static, TCollected: 'static> InBlock<TInput, TCollected>
where
    TInput: Send,
    TInput: Sync,
    TCollected: Send,
    TCollected: Sync,
{
    pub fn monitor_posts(&mut self) -> MonitorLoop {
        match self.ordering {
            OrderingMode::Ordered => self.monitor_ordered(),
            OrderingMode::Unordered => self.monitor_unordered()
        }
        
    }

    fn monitor_unordered(&mut self) -> MonitorLoop {
        let queue = self.work_queue.clone();

        let mut info = InBlockInfo {
            handler: (self.handler)()
        };

        let arc_collected = self.collected_items.clone();

        MonitorLoop::new(move || {
            let mut collected_list = arc_collected.lock();
            loop {
                let item = queue.wait_and_dequeue();
                match item {
                    TimestampedWorkItem(WorkItem::Value(val), order) => {
                        let collected: TCollected = info.handler.process(val, order);
                        (*collected_list).push(collected);
                    },
                    TimestampedWorkItem(WorkItem::Dropped, order) => {
                        ()
                    }
                    TimestampedWorkItem(WorkItem::Stop, _) => {
                        break;
                    }
                };
            }
        })
    }

    pub fn monitor_ordered(&mut self) -> MonitorLoop {
        let storage = self.ordered_work.clone();
        
        let mut info = InBlockInfo {
            handler: (self.handler)()
        };
        let arc_collected = self.collected_items.clone();

        MonitorLoop::new(move || {
            let mut next_item = 0;
            let mut collected_list = arc_collected.lock();
            loop {
                let item = storage.wait_and_remove(next_item);
                match item {
                    TimestampedWorkItem(WorkItem::Value(val), order) => {
                        debug_assert!(order == next_item);
                        next_item += 1;
                        let collected: TCollected = info.handler.process(val, order);
                        (*collected_list).push(collected);
                    }
                    TimestampedWorkItem(WorkItem::Dropped, order) => {
                        next_item += 1;
                    }
                    TimestampedWorkItem(WorkItem::Stop, _) => {
                        break;
                    }
                };
            }
        })
    }

}


impl<TInput, TCollected> InBlock<TInput, TCollected> {
    pub fn new(behavior: BlockMode, factory: Box<FnMut() -> Box<dyn In<TInput, TCollected>>>) -> InBlock<TInput, TCollected> {
        match behavior {
            BlockMode::Parallel(_) => unimplemented!("parallel inblocks not implemented"),
            BlockMode::Sequential(ordering) => InBlock {
                work_queue: BlockingQueue::new(),
                handler: factory,
                ordering: ordering,
                ordered_work: BlockingOrderedSet::new(),
                counter: AtomicUsize::new(0),
                collected_items: Arc::new(Mutex::new(vec![]))
            },
        }
    }
}

unsafe impl<TInput, TCollected> Send for InBlockInfo<TInput, TCollected> {}
unsafe impl<TInput, TCollected> Sync for InBlockInfo<TInput, TCollected> {}
