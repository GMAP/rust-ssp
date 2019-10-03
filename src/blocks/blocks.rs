use crate::work_storage::{WorkItem, TimestampedWorkItem};


//Base trait for all blocks in the pipeline
//Used by the internals. Should be able to detal with
//timestamped items and also perform some automatic timestamping on its own
pub trait PipelineBlock<TInput, TCollected> {
    fn process(&self, input: WorkItem<TInput>);
    fn process_timestamped(&self, input: TimestampedWorkItem<TInput>);
    fn collect(self: Box<Self>) -> Vec<TCollected>;
}

#[derive(Clone, Copy)]
pub enum OrderingMode {
    Unordered,
    Ordered,
}

pub enum BlockMode {
    Sequential(OrderingMode),
    Parallel(i32)
}



pub struct MonitorLoop {
    loop_function: Box<FnOnce() -> () + Send>
}

impl MonitorLoop {

    pub fn new<F>(function: F) -> MonitorLoop 
        where  F: FnOnce() -> (), F: Send + 'static {
        MonitorLoop {
            loop_function: Box::new(function)
        }
    }

    pub fn run(self) {
        (self.loop_function)()
    }

}