
use std::thread;
use std::thread::JoinHandle;
use crate::blocks::*;
use crate::work_storage::WorkItem;

pub struct Pipeline<TInput, TOutput, TCollected> {
    signaled_end: bool,
    initial_block: Option<InOutBlock<TInput, TOutput, TCollected>>,
    monitors: Vec<MonitorLoop>,
    threads: Vec<JoinHandle<()>>
}

impl<TInput: 'static, TOutput: 'static, TCollected: 'static> Pipeline<TInput, TOutput, TCollected> 
where
    TInput: Send,
    TInput: Sync {
   
    pub fn new(
        initial_block: InOutBlock<TInput, TOutput, TCollected>,
        monitors: Vec<MonitorLoop>
    ) -> Pipeline<TInput, TOutput, TCollected> {
        Pipeline {
            initial_block: Some(initial_block),
            monitors: monitors,
            threads: vec![],
            signaled_end: false
        }
    }

    fn end(&mut self) {
        self.signaled_end = true;
        match &self.initial_block {
            Some(block) => block.send_stop(),
            None => {}
        }
    }

    pub fn end_and_wait(&mut self) {
        self.end();
        let all_threads = std::mem::replace(&mut self.threads, vec![]);
        for thread in all_threads {
            thread.join().unwrap();
        }
    }

    pub fn post(&self, item: TInput) -> Result<(), ItemPostError> {
        if self.signaled_end {
            return Err(ItemPostError::StreamEnded);
        }
        match &self.initial_block {
            Some(block) => {
                block.process(WorkItem::Value(item));
                Ok(())
            }
            None => Err(ItemPostError::UnknownError)
        }
        
    }

    pub fn collect(mut self) -> Vec<TCollected> {
        self.end_and_wait();

        let current_block = std::mem::replace(&mut self.initial_block, None);
        match current_block {
            Some(block) => {
                Box::new(block).collect()     
            }
            None => vec![]
        }
    }

    pub fn start(&mut self) {
        let monitors = std::mem::replace(&mut self.monitors, vec![]);
        
        for monitor in monitors {
            self.threads.push(thread::spawn(move || {
                monitor.run();
            }))
        }
    }
}

impl<TInput, TOutput, TCollected> Drop for Pipeline<TInput, TOutput, TCollected> {
    fn drop(&mut self) {

        let block = std::mem::replace(&mut self.initial_block, None);

        if !self.signaled_end {
            block.unwrap().send_stop();
        }

        let all_threads = std::mem::replace(&mut self.threads, vec![]);
        for thread in all_threads {
            thread.join().unwrap();
        }
    }   
}


#[derive(Debug)]
pub enum ItemPostError {
    StreamEnded,
    UnknownError
}

#[macro_export]
macro_rules! pipeline_propagate {
    ($threads:expr, $s1:expr) => {
        {
            let (mode, factory) = $s1;
            let mut block = InBlock::new(mode, factory);
            $threads.push(block.monitor_posts());
            block
        }
    };

    ($threads:expr, $s1:expr $(, $tail:expr)*) => {
        {
            let (mode, factory) = $s1;
            let mut block = InOutBlock::new(
                Box::new(pipeline_propagate!($threads, $($tail),*)),
                mode, factory);
            $threads.extend(block.monitor_posts());
            block
        }
    };
}


#[macro_export]
macro_rules! pipeline {
    ($s1:expr $(, $tail:expr)*) => {
        {
            let mut monitors = Vec::<MonitorLoop>::new();
            let (mode, factory) = $s1;
            let mut block = InOutBlock::new(
                Box::new(pipeline_propagate!(monitors, $($tail),*)),
                mode, factory);
            monitors.extend(block.monitor_posts());

            let mut pipeline = Pipeline::new(block, monitors);
            pipeline.start();
            pipeline
        }
    };
}


#[macro_export]
macro_rules! parallel {
    ($block:expr, $threads:expr) => {
        {
            let mode = BlockMode::Parallel($threads);
            let factory: Box<FnMut() -> Box<InOut<_,_>>> = Box::new(move || Box::new($block));
            (mode, factory)
        }
    };
}


#[macro_export]
macro_rules! sequential {
    ($block:expr) => {
        {
            let mode = BlockMode::Sequential(OrderingMode::Unordered);
            let factory: Box<FnMut() -> Box<In<_, _>>> = Box::new(move || Box::new($block));
            (mode, factory)
        }
    };
}

#[macro_export]
macro_rules! sequential_ordered {
    ($block:expr) => {
        {
            let mode = BlockMode::Sequential(OrderingMode::Ordered);
            let factory: Box<FnMut() -> Box<In<_, _>>> = Box::new(move || Box::new($block));
            (mode, factory)
        }
    };
}


#[macro_export]
macro_rules! collect {
    () => {
        {
            sequential!(move |item: _| {item})
        }
    };
}


#[macro_export]
macro_rules! collect_ordered {
    () => {
        {
            sequential_ordered!(move |item: _| {item})
        }
    };
}