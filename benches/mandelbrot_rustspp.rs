
#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::ParameterizedBenchmark;
use criterion::Benchmark;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use rust_spp::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use raster::Color;
use futures::Map;
use tokio::*;
use tokio_core::reactor::Core;
use tokio::prelude::*;
use std::thread;
use std::time::*;
use futures::sync::*;

use futures::{stream, Stream, Future};
use futures::future::lazy;
struct ImageLine {
    line_index: usize,
    line_buffer: Vec<u8>
}


fn render_line(size: usize, line: usize) -> Option<ImageLine> {

    let init_a = -2.125 as f64;
    let init_b = -1.5 as f64;
    let range = 3.0 as f64;
    let step = range / (size as f64);
    
    let mut m: Vec<u8> = vec![0; size];

    let i = line;

    let im = init_b + (step * (i as f64));
    let iterations = 10000;

    for j in 0 .. size {

        let mut a = init_a + step * j as f64;
        let cr = a;

        let mut b = im;
        let mut k = 0;

        for ii in 0.. iterations {
            let a2 = a * a;
            let b2 = b * b;
            if (a2 + b2) > 4.0 {break;}
            b = 2.0 * a * b + im;
            a = a2 - b2 + cr;
            k = ii;
        }
        m[j] = (255 as f64 - (((k as f64) * 255 as f64 / (iterations as f64)))) as u8;
    }
    return Some(ImageLine {
        line_index: line as usize,
        line_buffer: m
    });
}

struct ComputeLine {
    size: usize
}
impl ComputeLine {
    fn new(size: usize) -> ComputeLine {
        ComputeLine {size: size}
    }
}
impl InOut<usize, ImageLine> for ComputeLine {
    fn process(&mut self, image_line: usize) -> Option<ImageLine> { 
        render_line(self.size, image_line)
    }
}


struct RenderLine {
}
impl In<ImageLine, ImageLine> for RenderLine {
    fn process(&mut self, image_line: ImageLine, _order: u64) -> ImageLine {
        image_line
    }
}

fn mandelbrot_sequential(size: usize) -> Vec<Option<ImageLine>> {
    (0 .. size).into_iter()
        .map(|image_line| render_line(size, image_line))
        .collect()
}

fn mandelbrot_rustspp(size: usize, threads: usize) {
    
    let pipeline = pipeline![
        parallel!(move |line_index| render_line(size, line_index), threads as i32),
        collect!()];
    
    for i in 0 .. size {
        pipeline.post(i as usize).unwrap();
    }
    let rendered_image = pipeline.collect();
    for item in rendered_image {
        println!("Collected line {:?}", item.line_index)
    }
}



fn mandelbrot_rustspp_ordered(size: usize, threads: usize) {
    
    let mut pipeline = pipeline![
        parallel!(ComputeLine::new(size), threads as i32),
        collect_ordered!()];
    
    for i in 0 .. size {
        pipeline.post(i as usize).unwrap();
    }
    let lines = pipeline.collect();
}


fn mandelbrot_tokio(size: usize, threads: usize) {

   let mandelbrot_stream = stream::iter_ok(0..size)
    .map(move |index| {
        let (sender, receiver) = oneshot::channel::<ImageLine>();
        tokio::spawn(lazy(move || {
            let result = render_line(size, index );
            sender.send(result.unwrap()).ok();
            Ok(())
        }));
        receiver
    })
    .buffered(threads)
    .for_each(|_rendered_line| { Ok(())})
    .map_err(|e| println!("listener error = {:?}", e));

    tokio::run(mandelbrot_stream);
}


fn mandelbrot_rayon(size: usize, thread_pool: Rc<rayon::ThreadPool>) -> Vec<ImageLine>{
    let mut b = vec![];
    thread_pool.install(|| {
        (0 .. size).into_par_iter()
            .map(|image_line| { render_line(size, image_line).unwrap() })
            .collect_into_vec(&mut b);
    });
    return b;
}


fn mandelbrot_tokio_unordered(size: usize, threads: usize) {

   let line_stream = stream::iter_ok(0..size);
  
   let mandelbrot_stream = line_stream
    .map(move |index| {
        let (sender, receiver) = oneshot::channel::<ImageLine>();
        tokio::spawn(lazy(move ||{
            let result = render_line(size, index);
            sender.send(result.unwrap()).ok();
            Ok(())
        }));
        receiver
    })
    .buffer_unordered(threads)
    .for_each(|_rendered_line| { Ok(())})
    .map_err(|e| println!("listener error = {:?}", e));

    tokio::run(mandelbrot_stream);

}

fn mandelbrot_benches(c: &mut Criterion) {
    
    c.bench("mandelbrot comparison",
        ParameterizedBenchmark::new(
            "mandelbrot rustspp ordered 1000x1000", 
            |b, &threads| { b.iter(|| mandelbrot_rustspp_ordered(1000, threads)); },
            4 ..= num_cpus::get() * 2)
        .sample_size(10));
    
    /*c.bench("mandelbrot comparison",
        ParameterizedBenchmark::new(
            "mandelbrot rustspp unordered 1000x1000", 
            |b, &threads| { b.iter(|| mandelbrot_rustspp(1000, threads)); },
            1 ..= num_cpus::get() * 2)
        .sample_size(10));

    c.bench("mandelbrot comparison",
        ParameterizedBenchmark::new(
            "mandelbrot rayon 1000x1000", 
            |b, &threads| { 
                let pool = Rc::new(ThreadPoolBuilder::new().num_threads(threads as usize).build().unwrap());
                b.iter(|| mandelbrot_rayon(1000, pool.clone())); },
            1 ..= num_cpus::get() * 2)
        .sample_size(10));

    c.bench("mandelbrot comparison",
        ParameterizedBenchmark::new(
            "mandelbrot tokio ordered 1000x1000", 
            |b, &threads| { b.iter(|| mandelbrot_tokio(1000, threads)); },
            1 ..= num_cpus::get() * 2)
        .sample_size(10));

    c.bench("mandelbrot comparison",
        ParameterizedBenchmark::new(
            "mandelbrot tokio unordered 1000x1000", 
            |b, &threads| { b.iter(|| mandelbrot_tokio_unordered(1000, threads)); },
            1 ..= num_cpus::get() * 2)
        .sample_size(10));*/
}
//criterion_group!(benches, criterion_benchmark);
criterion_group!(benches, mandelbrot_benches);
criterion_main!(benches);