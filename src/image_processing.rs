//#![feature(trace_macros)]
//trace_macros!(true);

use raster::filter;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use rust_spp::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct ImageToProcess {
    path: PathBuf,
    image: raster::Image,
}

struct LoadImage;
impl InOut<PathBuf, ImageToProcess> for LoadImage {
    fn process(&mut self, input: PathBuf) -> Option<ImageToProcess> {
        Some(ImageToProcess {
            image: raster::open(input.to_str().unwrap()).unwrap(),
            path: input,
        })
    }
}

struct ApplyGrayscale;
impl InOut<ImageToProcess, ImageToProcess> for ApplyGrayscale {
    fn process(&mut self, input: ImageToProcess) -> Option<ImageToProcess> {
        let mut input = input;
        filter::grayscale(&mut input.image).unwrap();
        Some(input)
    }
}

struct ApplySharpen;
impl InOut<ImageToProcess, ImageToProcess> for ApplySharpen {
    fn process(&mut self, input: ImageToProcess) -> Option<ImageToProcess> {
        let mut input = input;
        filter::sharpen(&mut input.image).unwrap();
        Some(input)
    }
}

struct ApplyGamma;
impl InOut<ImageToProcess, ImageToProcess> for ApplyGamma {
    fn process(&mut self, input: ImageToProcess) -> Option<ImageToProcess> {
        let mut input = input;
        filter::gamma(&mut input.image, 2.0).unwrap();
        Some(input)
    }
}

struct ApplyEmboss;
impl InOut<ImageToProcess, ImageToProcess> for ApplyEmboss {
    fn process(&mut self, input: ImageToProcess) -> Option<ImageToProcess> {
        let mut input = input;
        filter::emboss(&mut input.image).unwrap();
        Some(input)
    }
}

struct ApplyMoreSaturation;
impl InOut<ImageToProcess, ImageToProcess> for ApplyMoreSaturation {
    fn process(&mut self, input: ImageToProcess) ->  Option<ImageToProcess>  {
        let mut input = input;
        filter::saturation(&mut input.image, 0.2).unwrap();
        Some(input)
    }
}

struct ResizeTo500pxWidth;
impl InOut<ImageToProcess, ImageToProcess> for ResizeTo500pxWidth {
    fn process(&mut self, input: ImageToProcess) ->  Option<ImageToProcess>  {
        let mut input = input;
        raster::transform::resize_exact_width(&mut input.image, 500).unwrap();
        Some(input)
    }
}

struct SaveImageAndGetResult;
impl InOut<ImageToProcess, String> for SaveImageAndGetResult {
    fn process(&mut self, input: ImageToProcess) -> Option<String> {
        let result_dir = "../processed_images";

        let result = result_dir.to_owned()
            + "/"
            + input.path.file_stem().unwrap().to_str().unwrap()
            + "_processed."
            + input.path.extension().unwrap().to_str().unwrap();

        raster::save(&input.image, &result).unwrap();

        return Some(result.to_string());
    }
}

struct PrintResult;
impl In<String> for PrintResult {
    fn process(&mut self, input: String, order: u64) {
        // println!("Finished image {:?} {:?}", order, input)
    }
}

pub fn process_images(threads: i32) {
    let mut pipeline = pipeline![
        parallel!(
            |input: PathBuf| {
                Some(ImageToProcess {
                    image: raster::open(input.to_str().unwrap()).unwrap(),
                    path: input,
                })
            },
            50
        ),
        parallel!(ApplyMoreSaturation, threads),
        parallel!(ApplyEmboss, threads),
        parallel!(ApplyGamma, threads),
        parallel!(ApplySharpen, threads),
        parallel!(ApplyGrayscale, threads),
        parallel!(ResizeTo500pxWidth, threads),
        parallel!(SaveImageAndGetResult, 50),
        sequential!(|input: String| {})
    ];

    let dir_entries = std::fs::read_dir("../images");

    for entry in dir_entries.unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().is_none() {
            continue;
        }

        pipeline.post(path).unwrap();
    }

    pipeline.end_and_wait();
}

fn load_all_images() -> Vec<ImageToProcess> {
    
    let pipeline = pipeline![
        parallel!(LoadImage, 50),
        sequential!(|image: ImageToProcess| {image})
    ];

    let dir_entries = std::fs::read_dir("../images");
    
    for entry in dir_entries.unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().is_none() {
            continue;
        }

        pipeline.post(path).unwrap();
    }

    let collected = pipeline.collect();
    println!("All {:?} images loaded", collected.len());
    collected
}

struct DummySave;
impl In<ImageToProcess> for DummySave {
    fn process(&mut self, input: ImageToProcess, order: u64) {
        // println!("Finished image {:?} {:?}", order, input)
    }
}

macro_rules! time {
    ($e:expr, $clock_name:expr) => {
        //  let start = time::precise_time_s();
        $e;
        //  let end = time::precise_time_s();
        // println!("{:?} took {:?}", $clock_name, end - start);
    };
}

pub fn process_sequential() -> f64 {
    let all_images = load_all_images();
    // println!("Sequential");
    let result_dir = "../processed_images";

    let start = time::precise_time_s();

    // println!("dir entreis {:?}", dir_entries);
    for image_to_process in all_images.into_iter() {
        let path = image_to_process.path;
        let mut image = image_to_process.image;

        //let result = result_dir.to_owned() + "/"+
        //    path.file_stem().unwrap().to_str().unwrap() +
        //     "_processed." +
        //    path.extension().unwrap().to_str().unwrap();

        //   println!("Processing {:?}", path);
        time!(filter::saturation(&mut image, 0.2).unwrap(), "saturation");
        time!(filter::emboss(&mut image).unwrap(), "emboss");
        time!(filter::gamma(&mut image, 2.0).unwrap(), "gamma");
        time!(filter::sharpen(&mut image).unwrap(), "sharpen");
        time!(filter::grayscale(&mut image).unwrap(), "grayscale");
        time!(
            raster::transform::resize_exact_width(&mut image, 500).unwrap(),
            "resize(500px)"
        );

        // time!(raster::save(&image, &result).unwrap(), "save");

        // println!("");
        // println!("");
    }

    let end = time::precise_time_s();
    return end - start;
}

fn emboss(mut input: ImageToProcess) -> ImageToProcess {
    filter::emboss(&mut input.image).unwrap();
    input
}

pub fn process_images_no_IO(threads: i32) -> f64 {
    let all_images = load_all_images();

   
    let mut pipeline = pipeline![
        parallel!(|mut image_to_process: ImageToProcess| {
                filter::saturation(&mut image_to_process.image, 0.2).unwrap();
                Some(image_to_process)
        }, threads),
        parallel!(|mut input: ImageToProcess| {
                filter::emboss(&mut input.image).unwrap();
                Some(input)
        }, threads),
        parallel!(|mut input: ImageToProcess| {
                filter::gamma(&mut input.image, 2.0).unwrap();
                Some(input) 
        }, threads),
        parallel!(|mut input: ImageToProcess| {
                filter::sharpen(&mut input.image).unwrap();
                Some(input)
        }, threads),
        parallel!(|mut input: ImageToProcess| {
                filter::grayscale(&mut input.image).unwrap();
                Some(input)
        }, threads),
        parallel!(|mut input: ImageToProcess| {
                raster::transform::resize_exact_width(&mut input.image, 500).unwrap();
                Some(input)
        }, threads),
        sequential!(|_|{})
    ];

    let start = time::precise_time_s();

    for entry in all_images {
        pipeline.post(entry).unwrap();
    }

    pipeline.end_and_wait();

    let end = time::precise_time_s();
    return end - start;
}

use futures::future::lazy;
use futures::sync::*;
use futures::{stream, Future, Stream};
use tokio::prelude::*;
use tokio::*;
use tokio_core::reactor::Core;

macro_rules! spawn_return {
    ($block:expr) => {{
        let (sender, receiver) = oneshot::channel::<_>();
        tokio::spawn(lazy(move || {
            let result = $block;
            sender.send(result).ok();
            Ok(())
        }));
        receiver
    }};
}

pub fn process_images_tokio(threads: i32) -> f64 {
    let threads = threads as usize;

    let all_images = load_all_images();

    let start = time::precise_time_s();

    let image_stream = stream::iter_ok(all_images);

    let processing_pipeline = image_stream
        .map(move |mut image_to_process: ImageToProcess| {
            spawn_return!({
                filter::saturation(&mut image_to_process.image, 0.2).unwrap();
                image_to_process
            })
        })
        .buffer_unordered(threads)
        .map(move |mut img: ImageToProcess| {
            spawn_return!({
                filter::emboss(&mut img.image).unwrap();
                img
            })
        })
        .buffer_unordered(threads)
        .map(move |mut img: ImageToProcess| {
            spawn_return!({
                filter::gamma(&mut img.image, 2.0).unwrap();
                img
            })
        })
        .buffer_unordered(threads)
        .map(move |mut img: ImageToProcess| {
            spawn_return!({
                filter::sharpen(&mut img.image).unwrap();
                img
            })
        })
        .buffer_unordered(threads)
        .map(move |mut img: ImageToProcess| {
            spawn_return!({
                filter::grayscale(&mut img.image).unwrap();
                img
            })
        })
        .buffer_unordered(threads)
        .map(move |mut img: ImageToProcess| {
            spawn_return!({
                raster::transform::resize_exact_width(&mut img.image, 500).unwrap();
                img
            })
        })
        .buffer_unordered(threads)
        .for_each(|_rendered_line| Ok(()))
        .map_err(|e| println!("listener error = {:?}", e));

    tokio::run(processing_pipeline);

    let end = time::precise_time_s();
    return end - start;
}

pub fn process_images_tokio_unbuffered() -> f64 {
    let all_images = load_all_images();

    let start = time::precise_time_s();

    let image_stream = stream::iter_ok(all_images);

    let processing_pipeline = image_stream
        .map(move |mut image_to_process: ImageToProcess| {
            filter::saturation(&mut image_to_process.image, 0.2).unwrap();
            image_to_process
        })
        .map(move |mut img: ImageToProcess| {
            filter::emboss(&mut img.image).unwrap();
            img
        })
        .map(move |mut img: ImageToProcess| {
            filter::gamma(&mut img.image, 2.0).unwrap();
            img
        })
        .map(move |mut img: ImageToProcess| {
            filter::sharpen(&mut img.image).unwrap();
            img
        })
        .map(move |mut img: ImageToProcess| {
            filter::grayscale(&mut img.image).unwrap();
            img
        })
        .map(move |mut img: ImageToProcess| {
            raster::transform::resize_exact_width(&mut img.image, 500).unwrap();
            img
        })
        .for_each(|_rendered_line| Ok(()));

    tokio::run(processing_pipeline);

    let end = time::precise_time_s();
    return end - start;
}

pub fn process_images_rayon(threads: i32) -> f64 {
    let all_images = load_all_images();

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(threads as usize)
        .build()
        .unwrap();

    let mut b = vec![];

    let start = time::precise_time_s();

    thread_pool.install(|| {
        all_images
            .into_par_iter()
            .map(|mut img| {
                filter::saturation(&mut img.image, 0.2).unwrap();
                img
            })
            .map(|mut img| {
                filter::emboss(&mut img.image).unwrap();
                img
            })
            .map(|mut img| {
                filter::gamma(&mut img.image, 2.0).unwrap();
                img
            })
            .map(|mut img| {
                filter::sharpen(&mut img.image).unwrap();
                img
            })
            .map(|mut img| {
                filter::grayscale(&mut img.image).unwrap();
                img
            })
            .map(|mut img| {
                raster::transform::resize_exact_width(&mut img.image, 500).unwrap();
                img
            })
            .collect_into_vec(&mut b);
    });

    let end = time::precise_time_s();
    return end - start;
}

/*
pub fn resize_images(parallelism_level: i32) -> f64 {

    let all_images_paths = load_images_from_dir();

    let mut pipeline = Pipeline::new();

    let steps = pipeline![
        Pipeline::new(),
        parallel!(LoadImage, parallelism_level),
        parallel!(Resize { width: 500 }, parallelism_level),
        parallel!(SaveToDisk, parallelism_level),
        sequential!(CollectResult)];

    for entry in all_images{
        steps.post(entry);
    }

    steps.end();

    pipeline.wait();
}

*/
