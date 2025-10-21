use std::env;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Instant;

use hound::{WavReader, WavWriter};
use korangar_audio::{Frame, Resampler};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} --sample-rate=<rate> <input.wav> <output.wav>", args[0]);
        std::process::exit(1);
    }

    let sample_rate_arg = &args[1];
    let target_sample_rate = if sample_rate_arg.starts_with("--sample-rate=") {
        sample_rate_arg.strip_prefix("--sample-rate=").unwrap().parse::<u32>().unwrap()
    } else {
        eprintln!("Expected --sample-rate=<rate>, got: {sample_rate_arg}");
        std::process::exit(1);
    };

    let input_path = &args[2];
    let output_path = &args[3];

    // Read input WAV file.
    let mut reader = WavReader::open(input_path).unwrap();
    let spec = reader.spec();
    let input_sample_rate = spec.sample_rate;

    println!(
        "Input: {} Hz, {} channels, {} bits",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );
    println!("Output: {} Hz", target_sample_rate);

    // Read all samples and convert to Frame (stereo f32).
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
        hound::SampleFormat::Int => {
            // Convert integer samples to f32 in range [-1.0, 1.0]
            let max_value = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.unwrap() as f32 / max_value).collect()
        }
    };

    // Convert to Frame structs (stereo).
    let frames: Vec<Frame> = match spec.channels {
        1 => {
            // Mono: duplicate to both channels.
            samples.iter().map(|&sample| Frame::from_mono(sample)).collect()
        }
        2 => {
            // Stereo: pair up left and right.
            samples.chunks_exact(2).map(|chunk| Frame::new(chunk[0], chunk[1])).collect()
        }
        _ => {
            eprintln!("Unsupported channel count: {}", spec.channels);
            std::process::exit(1);
        }
    };

    println!("Input frames: {}", frames.len());

    let input_size_bytes = frames.len() * size_of::<Frame>();
    let input_size_mib = input_size_bytes as f64 / (1024.0 * 1024.0);

    let mut resampler = Resampler::new(input_sample_rate, target_sample_rate);

    let start = Instant::now();
    let resampled_frames: Arc<[Frame]> = resampler.resample_batch(&frames);
    let elapsed = start.elapsed();

    println!("Output frames: {}", resampled_frames.len());

    let elapsed_secs = elapsed.as_secs_f64();
    let throughput_mib_per_sec = input_size_mib / elapsed_secs;
    println!(
        "Resampling took {:.3} ms ({:.2} MiB/s)",
        elapsed.as_secs_f64() * 1000.0,
        throughput_mib_per_sec
    );

    // Write output WAV file.
    let output_spec = hound::WavSpec {
        channels: 2,
        sample_rate: target_sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create(output_path, output_spec).unwrap();

    for frame in resampled_frames.iter() {
        writer.write_sample(frame.left).unwrap();
        writer.write_sample(frame.right).unwrap();
    }

    writer.finalize().unwrap();

    println!("Done! Written to {output_path}");
}
