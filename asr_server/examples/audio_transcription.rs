// This example is not going to build in this folder.
// You need to copy this code into your project and add the dependencies whisper_rs and hound in your cargo.toml

use hound;
use std::fs::File;
use std::io::Write;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use opencc_rs::{OpenCC, Config};

/// Loads a context and model, processes an audio file, and prints the resulting transcript to stdout.
fn main() -> Result<(), &'static str> {
    // Load a context and model.
    let ctx = WhisperContext::new_with_params(
        "asr_server/whisper-rs/sys/whisper.cpp/models/ggml-large-v3-q5_0.bin",
        WhisperContextParameters::default(),
    )
    .expect("failed to load model");
    // Create a state
    let mut state = ctx.create_state().expect("failed to create key");

    // Create a params object for running the model.
    // The number of past samples to consider defaults to 0.
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });

    // Edit params as needed.
    // Set the number of threads to use to 1.
    params.set_n_threads(4);
    // Enable translation.
    params.set_translate(false);
    // Set the language to translate to to English.
    params.set_language(Some("zh"));
    // Disable anything that prints to stdout.
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_no_timestamps(true);

    // Open the audio file.
    let mut reader = hound::WavReader::open("A11_0.wav").expect("failed to open file");
    #[allow(unused_variables)]
    let hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        ..
    } = reader.spec();
    println!("channels:{}, sample_rate:{}, bits_per_sample:{}", channels, sample_rate, bits_per_sample);

    // Convert the audio to floating point samples.
    let mut audio = whisper_rs::convert_integer_to_float_audio(
        &reader
            .samples::<i16>()
            .map(|s| s.expect("invalid sample"))
            .collect::<Vec<_>>(),
    );

    // Convert audio to 16KHz mono f32 samples, as required by the model.
    // These utilities are provided for convenience, but can be replaced with custom conversion logic.
    // SIMD variants of these functions are also available on nightly Rust (see the docs).
    if channels == 2 {
        audio = whisper_rs::convert_stereo_to_mono_audio(&audio)?;
    } else if channels != 1 {
        panic!(">2 channels unsupported");
    }

    if sample_rate != 16000 {
        panic!("sample rate must be 16KHz");
    }

    // Run the model.
    state.full(params, &audio[..]).expect("failed to run model");

    // Create a file to write the transcript to.
    let mut file = File::create("transcript.txt").expect("failed to create file");

    let opencc = OpenCC::new([Config::T2S]).unwrap();
    // Iterate through the segments of the transcript.
    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");
    for i in 0..num_segments {
        // Get the transcribed text and timestamps for the current segment.
        let segment = state
            .full_get_segment_text(i)
            .expect("failed to get segment");
        // let start_timestamp = state
        //     .full_get_segment_t0(i)
        //     .expect("failed to get start timestamp");
        // let end_timestamp = state
        //     .full_get_segment_t1(i)
        //     .expect("failed to get end timestamp");

        // Print the segment to stdout.
        // println!("[{} - {}]: {}", start_timestamp, end_timestamp, segment);

        // Format the segment information as a string.
        // let line = format!("[{} - {}]: {}\n", start_timestamp, end_timestamp, segment);
        println!("segment: {}", segment.clone());
        let line = opencc.convert(segment).unwrap();

        println!("text: {}", line);

        // Write the segment information to the file.
        file.write_all(line.as_bytes())
            .expect("failed to write to file");
    }
    Ok(())
}
