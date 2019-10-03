use crate::blocks::*;
use crate::work_storage::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::thread;

// Public API: A Input-Output node; transforms some value into another
pub trait InOut<TInput, TOutput> {
    fn process(&mut self, input: TInput) -> Option<TOutput>;
}


impl <TInput, TOutput, F> InOut<TInput, TOutput> for F where F: FnMut(TInput) -> Option<TOutput> {
    fn process(&mut self, input: TInput) -> Option<TOutput> {
        (*self)(input)
    }
}


// Internals: This is a thread-local object for inout blocks
struct InOutBlockInfo<TInput, TOutput, TCollected> {
    next_step: Arc<Box<dyn PipelineBlock<TOutput, TCollected>>>,
    transformer: Box<dyn InOut<TInput, TOutput>>
}

//Internals: Processing queue for inout blocks in the pipeline
pub struct InOutBlock<TInput, TOutput, TCollected> {
    work_queue: Arc<BlockingQueue<TInput>>,
    next_step: Arc<Box<dyn PipelineBlock<TOutput, TCollected>>>,
    transformer_factory: Box<FnMut() -> Box<dyn InOut<TInput, TOutput>>>,
    replicas: i32,
}

impl<TInput, TOutput, TCollected> InOutBlock<TInput, TOutput, TCollected> {
    pub fn send_stop(&self) {
        (*self.work_queue).enqueue(WorkItem::Stop);
    }
}

impl<TInput: 'static, TCollected: 'static, TOutput: 'static> PipelineBlock<TInput, TCollected> 
for InOutBlock<TInput, TOutput, TCollected>
where
    TInput: Send,
    TInput: Sync,
{
    //used by the public API. Always unordered
    fn process(&self, input: WorkItem<TInput>) {
        (*self.work_queue).enqueue(input);
    }

    //Used internally
    fn process_timestamped(&self, input: TimestampedWorkItem<TInput>) {
        (*self.work_queue).enqueue_timestamped(input)
    }

    fn collect(self: Box<Self>) -> Vec<TCollected> {
        match Arc::try_unwrap(self.next_step) {
            Ok(result) => result.collect(),
            Err(_) => {
                panic!("Could not unwrap Arc in call to collect");
            }
        }
    }

}

impl<TInput: 'static, TOutput: 'static, TCollected: 'static> InOutBlock<TInput, TOutput, TCollected>
where
    TInput: Send,
    TInput: Sync,
{
    pub fn new(
        next_step: Box<dyn PipelineBlock<TOutput, TCollected>>,
        transformer: BlockMode,
        transformer_factory: Box<FnMut() -> Box<dyn InOut<TInput, TOutput>>>
    ) -> InOutBlock<TInput, TOutput, TCollected> {
        match transformer {
            BlockMode::Parallel(replicas) => {
                InOutBlock::new_block(next_step, transformer_factory, replicas)
            }
            BlockMode::Sequential(_) => InOutBlock::new_block(next_step, transformer_factory, 1),
        }
    }
   
    pub fn new_block(
        next_step: Box<dyn PipelineBlock<TOutput, TCollected>>,
        transformer: Box<FnMut() -> Box<dyn InOut<TInput, TOutput>>>,
        replicas: i32,
    ) -> InOutBlock<TInput, TOutput, TCollected> {
        InOutBlock {
            work_queue: BlockingQueue::new(),
            next_step: Arc::new(next_step),
            transformer_factory: transformer,
            replicas: replicas,
        }
    }


    pub fn monitor_posts(&mut self) -> Vec<MonitorLoop> {
        let mut monitors: Vec<MonitorLoop> = vec![];
        let alive_threads = Arc::new(AtomicUsize::new(self.replicas as usize));

        for _ in 0..self.replicas {
            let queue = self.work_queue.clone();
            let alive_threads = alive_threads.clone();
            
            let mut info = InOutBlockInfo {
                next_step: self.next_step.clone(),
                transformer: (self.transformer_factory)(),
            };
            
            let monitor_loop = MonitorLoop::new(move || {
               
                loop {
                    let dequeued = queue.wait_and_dequeue();

                    match dequeued {
                        TimestampedWorkItem(WorkItem::Value(val), order) => {
                            let output = info.transformer.process(val);

                            if let Some(val) = output {
                                info.next_step.process_timestamped(TimestampedWorkItem(
                                    WorkItem::Value(val),
                                    order,
                                ));
                            } else {
                                info.next_step.process_timestamped(TimestampedWorkItem(
                                    WorkItem::Dropped,
                                    order,
                                ));
                            }
                        },
                        TimestampedWorkItem(WorkItem::Dropped, order) => {
                            info.next_step.process_timestamped(TimestampedWorkItem(
                                WorkItem::Dropped,
                                order,
                            ));
                        },
                        TimestampedWorkItem(WorkItem::Stop, order) => {
                            let mut threads = alive_threads.load(Ordering::SeqCst);
                        
                            threads -= 1;

                            if threads == 0 {
                                info.next_step.process_timestamped(TimestampedWorkItem(
                                    WorkItem::Stop,
                                    order,
                                ));
                            }

                            alive_threads.store(threads, Ordering::SeqCst);

                            //reenqueue the same item
                            queue.enqueue_timestamped(dequeued);

                            break;
                        }
                    }
                }
            });
            monitors.push(monitor_loop);
        }

        return monitors;
    }

}

/* Assume a MapBlock can be passed to threads, and assume we'll implement parallelism correctly */
unsafe impl<TInput, TOutput, TCollected> Send for InOutBlockInfo<TInput, TOutput, TCollected> {}
unsafe impl<TInput, TOutput, TCollected> Sync for InOutBlockInfo<TInput, TOutput, TCollected> {}
