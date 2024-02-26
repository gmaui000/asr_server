use actix_web::web::BufMut;
use actix_web::{web, HttpResponse, http::StatusCode};
use futures::StreamExt;
use tracing::{self, info};
use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};
use super::super::super::AppState;
use chrono::Local;
use serde_json::json;
use actix_multipart::Multipart;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use opencc_rs::{OpenCC, Config};
use opus::Decoder;
use webm_iterable::{
    WebmIterator, 
    matroska_spec::{MatroskaSpec, Master, SimpleBlock},
};
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use minimp3;
use fdk_aac::dec;
 
#[actix_web::post("/api/asr")]
pub async fn api_asr(data: web::Data<Arc<Mutex<AppState>>>, mut payload: Multipart) -> HttpResponse {
    let start_time = Local::now();
    let mut content_type = None;
    let mut audio = Vec::new();
    let audio_to_recognize: Vec<f32>;
    while let Some(item) = payload.next().await {
        let mut field = item.unwrap();
        if let Some(ct) = field.content_type() {
            content_type = Some(ct.to_string());
        }
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            audio.put_slice(data.as_ref());
        }
    }
    
    // std::fs::write("audio.webm", &audio).unwrap();
    let mut cursor = Cursor::new(audio.to_vec());
    match content_type {
        Some(ct) if ct == "audio/webm;codecs=opus" => {
            let mut decoder = Decoder::new(48000, opus::Channels::Mono).expect("Failed to create decoder");
            let mut opus_data = Vec::new();
            let mut pcm_data: Vec<i16> = Vec::new();
            let tag_iterator = WebmIterator::new(&mut cursor, &[MatroskaSpec::TrackEntry(Master::Start)]);
            for tag in tag_iterator {
                if let MatroskaSpec::SimpleBlock(ref data) = tag.unwrap() {
                    let block: SimpleBlock = data.try_into().unwrap();
                    opus_data.push(block.raw_frame_data().to_vec());
                }
            }
        
            for opus in opus_data.iter() {
                let mut output: Vec<i16> = vec![0; 4096];
                let size = decoder.decode(&opus, &mut output, false).expect("解码 Opus 数据时发生错误");
                pcm_data.extend(&output[..size]);
            }
            let pcm_data_f32: Vec<f32> = pcm_data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let mut resampler = SincFixedIn::<f32>::new(16000 as f64 / 48000 as f64, 1.0, params, pcm_data_f32.len(), 1).unwrap();
            let converted_data: Vec<Vec<f32>> = vec![pcm_data_f32;1];
            let audio = resampler.process(&converted_data, None).unwrap();
            audio_to_recognize = audio[0].to_vec();
        }
        Some(ct) if ct == "audio/wav;codecs=pcm" => {
            let mut reader = hound::WavReader::new(&mut cursor).expect("Failed to read sample to WAV.");
            // #[allow(unused_variables)]
            // let hound::WavSpec {
            //     channels,
            //     sample_rate,
            //     bits_per_sample,
            //     ..
            // } = reader.spec();
            // println!("channels:{}, sample_rate:{}, bits_per_sample:{}", channels, sample_rate, bits_per_sample);
            // Convert the audio to floating point samples.
            audio_to_recognize = whisper_rs::convert_integer_to_float_audio(
                &reader
                    .samples::<i16>()
                    .map(|s| s.expect("invalid sample"))
                    .collect::<Vec<_>>(),
            );
        }
        Some(ct) if ct == "audio/mp3;codecs=mp3" => {
            let mut decoder = minimp3::Decoder::new(cursor);
            let mut pcm_data = Vec::new();
            loop {
                match decoder.next_frame() {
                    Ok(frame) => {
                        let mono_samples: Vec<i16> = frame.data.chunks(frame.channels as usize)
                            .map(|channel_samples| channel_samples[0])
                            .collect();
                        pcm_data.extend(mono_samples);
                    }
                    Err(_) => break, // 解码出错
                }
            }

            // Convert the audio to floating point samples.
            let pcm_data_f32: Vec<f32> = pcm_data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let mut resampler = SincFixedIn::<f32>::new(16000 as f64 / 48000 as f64, 1.0, params, pcm_data_f32.len(), 1).unwrap();
            let converted_data: Vec<Vec<f32>> = vec![pcm_data_f32;1];
            let audio = resampler.process(&converted_data, None).unwrap();
            audio_to_recognize = audio[0].to_vec();
        }
        Some(ct) if ct == "audio/aac;codecs=aac" => {
            let mut decoder = dec::Decoder::new(dec::Transport::Adts);
            let config = vec![0x10, 0x90]; //LC 48KHz 2channels
            decoder.config_raw(&config).unwrap();
            let mut pcm_data: Vec<i16> = Vec::new();
            loop {
                let mut header_bytes = [0;7];
                if cursor.read_exact(&mut header_bytes).is_err() {
                    break;
                }

                let frame_length = (((header_bytes[3] & 0x3) as usize)  << 11) | ((header_bytes[4] << 3) as usize) | (((header_bytes[5] & 0xE0) as usize) >> 5);
                let mut aac_frame = vec![0; frame_length];
                aac_frame[..7].copy_from_slice(&header_bytes);
                if cursor.read_exact(&mut aac_frame[7..]).is_err() {
                    break;
                }
                decoder.fill(&aac_frame).unwrap();

                let mut pcm_frame = vec![0; 2048];
                if let Err(dec::DecoderError::NOT_ENOUGH_BITS) = decoder.decode_frame(&mut pcm_frame) {
                    continue;
                }

                let mono_samples: Vec<i16> = pcm_frame.chunks(2)
                    .map(|channel_samples| channel_samples[0])
                    .collect();
                pcm_data.extend(&mono_samples);
            }

            // Convert the audio to floating point samples.
            let pcm_data_f32: Vec<f32> = pcm_data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let mut resampler = SincFixedIn::<f32>::new(16000 as f64 / 48000 as f64, 1.0, params, pcm_data_f32.len(), 1).unwrap();
            let converted_data: Vec<Vec<f32>> = vec![pcm_data_f32;1];
            let audio = resampler.process(&converted_data, None).unwrap();
            audio_to_recognize = audio[0].to_vec();

            // let bytes: Vec<u8> = pcm_data_f32.iter().flat_map(|&x| x.to_le_bytes().to_vec()).collect();
            // std::fs::write("output.pcm", &bytes).unwrap();

        }
        _ => {
            // 处理未知格式的音频数据，可能需要特定的逻辑或者返回错误响应
            return HttpResponse::build(StatusCode::BAD_REQUEST).body("err: unsupported audio format.");
        }
    }

    // let mut ogg_reader = PacketReader::new(&mut cursor);
    // let mut reader = hound::WavReader::new(&mut cursor).expect("Failed to read sample to WAV.");
    // Load a context and model.
    let ctx = WhisperContext::new_with_params(
        "models/ggml-large-v3-q5_0.bin",
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

    // Run the model.
    state.full(params, &audio_to_recognize[..]).expect("failed to run model");

    let opencc = OpenCC::new([Config::T2S]).unwrap();
    // Iterate through the segments of the transcript.
    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");
    let mut text = String::new();
    for i in 0..num_segments {
        // Get the transcribed text and timestamps for the current segment.
        let segment = state
            .full_get_segment_text(i)
            .expect("failed to get segment");
        text = opencc.convert(segment).unwrap();

        println!("text: {}", text);
    }

    let duration = Local::now().signed_duration_since(start_time);
    data.get_ref().lock().unwrap().track.record_query(text.to_owned(), duration);
    info!("req: {:?} cost: {:.2}s", text, duration.num_milliseconds() as f64 / 1000.0);

    HttpResponse::Ok().json(json!({ "text": text }))
}
