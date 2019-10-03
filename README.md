# Rust SSP #

Structured Stream Parallelism for Rust

Define a pipeline with N steps. Pipelines can be normal pipelines or "farm pipelines" (in which some steps are parallel).
You can also define pipelines with mutable state.

    fn pipelined() {
        let pipeline = pipeline![
            pipeline,
            parallel!(LoadImage, 40),
            parallel!(ApplyMoreSaturation, 2),
            parallel!(ApplyEmboss, 2),
            parallel!(ApplyGamma, 2),
            parallel!(ApplySharpen, 2),
            parallel!(ApplyGrayscale, 2),
            parallel!(SaveImageAndGetResult, 40),
            sequential!(PrintResult)];

        let dir_entries = std::fs::read_dir("/Users/user/Desktop/imagens");

        for entry in dir_entries.unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().is_none() { continue; }

            println!("Posting {:?}", path.to_str().unwrap());

            pipeline.post(path).unwrap();
            
        }

        pipeline.end_and_wait();

        println!("Finished.");
    }


# How to Cite our Work
	
Ricardo Pieper, Dalvan Griebler, and Luiz Gustavo Fernandes. 2019. **Structured Stream Parallelism for Rust.** In Proceedings of the XXIII Brazilian Symposium on Programming Languages (SBLP 2019). ACM, New York, NY, USA, 54-61. DOI: https://doi.org/10.1145/3355378.3355384 