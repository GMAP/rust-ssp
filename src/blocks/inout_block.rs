use crate::blocks::*;
use crate::work_storage::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{marker::PhantomData, sync::Arc};

// Public API: A Input-Output node; transforms some value into another
pub trait InOut<TInput, TOutput> {
    fn process(&mut self, input: TInput) -> Option<TOutput>;
}

impl<TInput, TOutput, F> InOut<TInput, TOutput> for F
where
    F: FnMut(TInput) -> Option<TOutput>,
{
    fn process(&mut self, input: TInput) -> Option<TOutput> {
        (*self)(input)
    }
}

//Internals: Processing queue for inout blocks in the pipeline
pub struct InOutBlock<
    TInput,
    TOutput,
    TCollected,
    TStage: InOut<TInput, TOutput> + Send,
    TFactory: FnMut() -> TStage,
    TNextStep: PipelineBlock<TOutput, TCollected> + Send,
> {
    work_queue: Arc<BlockingQueue<TInput>>,
    next_step: Arc<TNextStep>,
    transformer_factory: TFactory,
    replicas: i32,
    _params: std::marker::PhantomData<(TOutput, TCollected)>,
}

impl<
        TInput,
        TOutput,
        TCollected,
        TStage: InOut<TInput, TOutput> + Send,
        TFactory: FnMut() -> TStage,
        TNextStep: PipelineBlock<TOutput, TCollected> + Send + Sync,
    > InOutBlock<TInput, TOutput, TCollected, TStage, TFactory, TNextStep>
{
    pub fn send_stop(&self) {
        (*self.work_queue).enqueue(WorkItem::Stop);
    }
}

impl<
        TInput: 'static,
        TOutput: 'static,
        TCollected: 'static,
        TStage: InOut<TInput, TOutput> + Send + 'static,
        TFactory: FnMut() -> TStage,
        TNextStep: PipelineBlock<TOutput, TCollected> + Send + Sync,
    > PipelineBlock<TInput, TCollected>
    for InOutBlock<TInput, TOutput, TCollected, TStage, TFactory, TNextStep>
{
    //used by the public API. Always unordered
    fn process(&self, input: WorkItem<TInput>) {
        (*self.work_queue).enqueue(input);
    }

    //Used internally
    fn process_timestamped(&self, input: TimestampedWorkItem<TInput>) {
        (*self.work_queue).enqueue_timestamped(input)
    }

    fn collect(self) -> Vec<TCollected> {
        match Arc::try_unwrap(self.next_step) {
            Ok(result) => result.collect(),
            Err(_) => {
                panic!("Could not unwrap Arc in call to collect");
            }
        }
    }
}

impl<
        TInput: 'static + Send,
        TOutput: 'static,
        TCollected: 'static,
        TStage: InOut<TInput, TOutput> + Send + 'static,
        TFactory: FnMut() -> TStage,
        TNextStep: PipelineBlock<TOutput, TCollected> + Send + Sync + 'static,
    > InOutBlock<TInput, TOutput, TCollected, TStage, TFactory, TNextStep>
{
    pub fn new(
        next_step: TNextStep,
        transformer: BlockMode,
        transformer_factory: TFactory,
    ) -> InOutBlock<TInput, TOutput, TCollected, TStage, TFactory, TNextStep> {
        match transformer {
            BlockMode::Parallel(replicas) => {
                InOutBlock::new_block(next_step, transformer_factory, replicas)
            }
            BlockMode::Sequential(_) => InOutBlock::new_block(next_step, transformer_factory, 1),
        }
    }

    pub fn new_block(
        next_step: TNextStep,
        transformer: TFactory,
        replicas: i32,
    ) -> InOutBlock<TInput, TOutput, TCollected, TStage, TFactory, TNextStep> {
        InOutBlock {
            work_queue: BlockingQueue::new(),
            next_step: Arc::new(next_step),
            transformer_factory: transformer,
            replicas,
            _params: PhantomData,
        }
    }

    pub fn monitor_posts(&mut self) -> Vec<MonitorLoop> {
        let mut monitors: Vec<MonitorLoop> = vec![];
        let alive_threads = Arc::new(AtomicUsize::new(self.replicas as usize));

        for _ in 0..self.replicas {
            let queue = self.work_queue.clone();
            let alive_threads = alive_threads.clone();

            let next_step = self.next_step.clone();
            let mut transformer = (self.transformer_factory)();

            let monitor_loop = MonitorLoop::new(move || {
                loop {
                    let dequeued = queue.wait_and_dequeue();

                    match dequeued {
                        TimestampedWorkItem(WorkItem::Value(val), order) => {
                            let output = transformer.process(val);

                            if let Some(val) = output {
                                next_step.process_timestamped(TimestampedWorkItem(
                                    WorkItem::Value(val),
                                    order,
                                ));
                            } else {
                                next_step.process_timestamped(TimestampedWorkItem(
                                    WorkItem::Dropped,
                                    order,
                                ));
                            }
                        }
                        TimestampedWorkItem(WorkItem::Dropped, order) => {
                            next_step
                                .process_timestamped(TimestampedWorkItem(WorkItem::Dropped, order));
                        }
                        TimestampedWorkItem(WorkItem::Stop, order) => {
                            let mut threads = alive_threads.load(Ordering::SeqCst);

                            threads -= 1;

                            if threads == 0 {
                                next_step.process_timestamped(TimestampedWorkItem(
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
