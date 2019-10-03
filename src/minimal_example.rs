//#![feature(trace_macros)]
//trace_macros!(true);

#[macro_use]
extern crate lazy_static;

mod blocks;
mod blocking_queue;
mod blocking_ordered_set;
mod out_block;
mod inout_block;
mod work_item;
mod spp;

use blocking_queue::BlockingQueue;
use blocking_ordered_set::BlockingOrderedSet;
use rand::prelude::*;
use rand::Rng;
use std::cell::{Cell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize};
use std::thread;
use std::time::Duration;
use inout_block::*;
use out_block::*;
use blocks::*;
use std::fs;
use spp::Pipeline;


struct MultiplyBy2 {
    rng: ThreadRng,
}
impl InOut<i32, i32> for MultiplyBy2 {
    fn process(&mut self, input: i32) -> i32 {
        //thread::sleep(Duration::from_millis(self.rng.gen_range(1, 100)));
        input
    }
}

struct Finalize {
    counter: Arc<Mutex<i64>>
}

impl Out<i32> for Finalize {
    fn process(&mut self, input: i32, order: u64) {
        *self.counter.lock().unwrap() += 1;
    }
}

lazy_static! {
    static ref counter : Arc<Mutex<i64>> = Arc::new(Mutex::new(0));
}

fn main() {

    let mut pipeline = Pipeline::new();

    let steps = pipeline![
        pipeline,
        parallel!(
            MultiplyBy2 {
                rng: rand::thread_rng()
            },
            8
        ),
        parallel!(
            MultiplyBy2 {
                rng: rand::thread_rng()
            },
            8
        ),
        parallel!(
            MultiplyBy2 {
                rng: rand::thread_rng()
            },
            8
        ),
        sequential!(Finalize { counter: counter.clone() })
    ];


    for i in 0..1000000 {
        steps.post(i);
    }

    steps.end();

    pipeline.wait();

    println!("Result: {:?}", *counter);

}
