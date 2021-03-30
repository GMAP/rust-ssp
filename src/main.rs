use clap::{Arg, App};

mod image_processing;

use rust_spp::*;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;


fn leak_test() {
    let pipeline = pipeline![
        parallel!(|item: i32| { 
            Some(item * 2) 
        }, 20),
        parallel!(|item: i32| { 
            if item % 5 == 0 {
                None
            } else {
                Some(item * 2) 
            }
        }, 20),
        sequential_ordered!(|item: i32| {
            println!("number: {:?}", item);
        })
    ];

    for i in 1..1000 {
        pipeline.post(i).unwrap()
    }
}

struct SyncNotNeeded {
    test: std::rc::Rc<i64>
}

unsafe impl Send for SyncNotNeeded {}

fn send_sync_notneeded() {
    let pipeline = pipeline![
        parallel!(|item: SyncNotNeeded| { 
            Some(item) 
        }, 20),
        parallel!(|item: SyncNotNeeded| { 
            Some(item)
        }, 20),
        sequential_ordered!(|item: SyncNotNeeded| {
            println!("number: {:?}", item.test);
        })
    ];

    for i in 1..1000 {
        pipeline.post(SyncNotNeeded { test: std::rc::Rc::new(1) }).unwrap()
    }
}


fn main() {

    leak_test();

    send_sync_notneeded();

/*
    let matches = App::new("Rust-SPP tests")
        .version("1")
        .author("Ricardo Pieper")
        .arg(Arg::with_name("threads")
                .short("t")
                .long("threads")
                .help("sets the number of threads, lines in parallel being calculated")
                .takes_value(true))
        .arg(Arg::with_name("executions")
            .short("e")
            .long("executions")    
            .help("How many times you want to execute")
            .takes_value(true))
        .get_matches();   

    let threads = matches.value_of("threads").unwrap().parse::<i32>().unwrap();
    let executions = matches.value_of("executions").unwrap().parse::<i32>().unwrap();

    for thread in 1 ..=threads {
        println!("Executing with {:?} threads", thread);
        for execution in 0..executions {   
            let time =/* if thread == 1 {
                println!("Unbuffered");
                image_processing::process_images_tokio_unbuffered()
            } else {
                image_processing::process_images_tokio(thread)
            };*/ image_processing::process_images_no_IO(thread);
            println!("\tExecution {:?} took {:?}", execution, time);
        }
    }
*/
}
