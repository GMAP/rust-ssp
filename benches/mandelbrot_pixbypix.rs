
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

struct SquareImage {
    size: i32,
    cur_x: i32,
    cur_y: i32
}

impl Iterator for SquareImage {
    type Item = (i32, i32);
    fn next(&mut self) -> Option<(i32, i32)> {
        let ret = (self.cur_x, self.cur_y);
        
        self.cur_y = self.cur_y + 1;
        
        if (self.cur_y == self.size) {
            self.cur_x = self.cur_x + 1; 
            self.cur_y = 0;  
        }
        if (self.cur_x == self.size) {
            return None;
        } 
        else {
            return Some(ret);
        }
    }
}

struct Parameters {
    init_a: f64,
    init_b: f64,
    step: f64
}

fn calculate_pixel(x: i32, y: i32, step: f64, init_a: f64, init_b: f64) -> i32 {
    let iterations = 10000;

    let im = init_b + (step * (y as f64));
    let mut a = init_a + (step * (x as f64));
    let cr = a;

    let mut b = im;
    let mut k = 0;

    for i in 0.. iterations {
        let a2 = a * a;
        let b2 = b * b;
        if (a2 + b2) > 4.0 { break; }
        b = 2.0 * a * b + im;
        a = a2 - b2 + cr;
        k = i;
    }
    return k;
}

struct CalculatePixelIterations {
    params: Parameters
}
impl InOut<(i32, i32), (i32, i32, i32)> for CalculatePixelIterations {
    fn process(&mut self, position: (i32, i32)) -> Option<(i32, i32, i32)> {
        match position {
            (x, y) => {
                Some((x, y, calculate_pixel(x, y, self.params.step, self.params.init_a, self.params.init_b)))
            }
        }
    }
}

struct Renderer {
}
impl In<(i32, i32, i32), (usize, usize, u8)> for Renderer {
    fn process(&mut self, data: (i32, i32, i32), _order: u64) -> (usize, usize, u8) {
        let iterations = 10000;
        match data {
            (x, y, k) => {
                (x as usize, y as usize, (255 as f64 - (((k as f64) * 255 as f64 / (iterations as f64)))) as u8)
            }
        }
    }
}


fn mandelbrot_rustspp(size: usize, threads: i32) {
    {
        let mut pipeline = pipeline![
            parallel!(CalculatePixelIterations { 
                params: Parameters {
                    init_a: -2.125,
                    init_b: -1.5,
                    step: 3.0 / (size as f64),
                } 
            }, threads),
            sequential!(Renderer { })];
        
        for i in 0 .. size { 
            for j in 0 .. size {
                pipeline.post((i as i32, j as i32)).unwrap();
            }
        }
        pipeline.end_and_wait();
    }
}

fn mandelbrot_rayon(size: usize, thread_pool: Rc<rayon::ThreadPool>) {
    //x: i32, y: i32, step: f64, init_a: f64, init_b: f64
    let buf: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(vec![vec![0u8; size]; size]));
    
    let pixels = SquareImage { size: size as i32, cur_x:0, cur_y:0 };
    
    let as_vec: Vec<(i32, i32)> = pixels.into_iter().collect();
    
    let params = Parameters {
        init_a: -2.125,
        init_b: -1.5,
        step: 3.0 / (size as f64),
    };

    thread_pool.install(|| {
        as_vec.into_par_iter()
                .map(|(x, y)| (x, y, calculate_pixel(x, y, params.step, params.init_a, params.init_b)))
                .for_each(|(x,y,k)| {
                    let mut buf = buf.lock().unwrap();
                    let iterations = 10000;
                    buf[x as usize][y as usize] = (255 as f64 - (((k as f64) * 255 as f64 / (iterations as f64)))) as u8;
                });
    });
   
}

fn get_pixel_value(iterations: i32, k: i32) -> u8 {
    (255 as f64 - (((k as f64) * 255 as f64 / (iterations as f64)))) as u8
}

type PixelPositionAndValue = (i32,i32,u8);

fn mandelbrot_rayon_collect(size: usize, thread_pool: Rc<rayon::ThreadPool>) {
    //x: i32, y: i32, step: f64, init_a: f64, init_b: f64
   
    let pixels = SquareImage { size: size as i32, cur_x:0, cur_y:0 };
    
    let as_vec: Vec<(i32, i32)> = pixels.into_iter().collect();
    
    let params = Parameters {
        init_a: -2.125,
        init_b: -1.5,
        step: 3.0 / (size as f64),
    };

    thread_pool.install(|| {
        let mut buf: Vec<Vec<u8>> = vec![vec![0u8; size]; size];
        let mut all_pixels: Vec<PixelPositionAndValue> = vec![(0,0,0); size];
    
        as_vec.into_par_iter()
                .map(|(x, y)| (x, y, 
                    get_pixel_value(10000, calculate_pixel(x, y, params.step, params.init_a, params.init_b))))
                .collect_into_vec(&mut all_pixels);
        
        for (x, y, pixel) in all_pixels {
            buf[x as usize][y as usize] = pixel;
        }
    });
   
}

fn criterion_benchmark(c: &mut Criterion) {
    
    let threads_to_run = 4 ..= (num_cpus::get() as i32) * 2;
    println!("threads: {:?}", threads_to_run);
    c.bench("mandelbrot pixel by pixel comparison",
        ParameterizedBenchmark::new(
            "rustsupp mandelbrot 1000x1000", 
            |b, &threads| { b.iter(|| mandelbrot_rustspp(1000, threads)); },
            threads_to_run)
         .with_function("rayon mandelbrot 1000x1000", 
           |b, &threads| { 
                let pool = Rc::new(ThreadPoolBuilder::new().num_threads(threads as usize).build().unwrap());
                b.iter(|| mandelbrot_rayon(1000, pool.clone())); })
        .sample_size(10));

}


fn rayon_benchmark(c: &mut Criterion) {
    
    let threads_to_run = 1 ..= (num_cpus::get() as i32);
    println!("threads: {:?}", threads_to_run);
    c.bench("mandelbrot rayon pixel by pixel",
        ParameterizedBenchmark::new(
            "rayon foreach 1000x1000", 
            |b, &threads| { 
                let pool = Rc::new(ThreadPoolBuilder::new().num_threads(threads as usize).build().unwrap());
                b.iter(|| mandelbrot_rayon(1000, pool.clone())); 
            },threads_to_run)
        .with_function("rayon collect 1000x1000", 
           |b, &threads| { 
                let pool = Rc::new(ThreadPoolBuilder::new().num_threads(threads as usize).build().unwrap());
                b.iter(|| mandelbrot_rayon_collect(1000, pool.clone())); })
        .sample_size(10));

}

criterion_group!(benches, criterion_benchmark);
//criterion_group!(benches, rayon_benchmark);
criterion_main!(benches);