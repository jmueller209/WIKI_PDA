// shared/src/compression.rs
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Instant;
use zstd::dict::{DecoderDictionary, EncoderDictionary};
use zstd::stream::{read::Decoder, write::Encoder};

pub fn train_and_save_zstd_dictionary<T: AsRef<[u8]>>(
    samples: &[T],
    target_dict_size: usize,
    output_path: &Path,
) -> io::Result<()> {
    println!(
        "Training dictionary of target size {} bytes using {} samples...",
        target_dict_size,
        samples.len()
    );

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let dictionary = zstd::dict::from_samples(samples, target_dict_size)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut file = File::create(output_path)?;
    file.write_all(&dictionary)?;

    println!("Successfully saved dictionary to {:?}", output_path);

    Ok(())
}

pub fn test_zstd_compression_rate<T: AsRef<[u8]>>(
    samples: &[T],
    dict_path: &Path,
    level: i32,
    window_size_kb: usize,
) -> io::Result<String> {
    let prepared_dict = load_zstd_encoder_dictionary(dict_path, level)?;

    let num_samples = samples.len();
    if num_samples == 0 {
        return Ok("No data provided to test compression rate.".to_string());
    }

    let mut total_original_bytes: u64 = 0;
    let mut total_compressed_bytes: u64 = 0;
    let mut total_time_sec: f64 = 0.0;

    let mut ratios = Vec::with_capacity(num_samples);
    let mut times_us = Vec::with_capacity(num_samples);

    for sample in samples {
        let raw_bytes = sample.as_ref();
        let orig_len = raw_bytes.len() as f64;
        total_original_bytes += raw_bytes.len() as u64;

        let start = Instant::now();
        let compressed = compress_data_zstd(raw_bytes, &prepared_dict, window_size_kb)?;
        let duration = start.elapsed().as_secs_f64();

        let comp_len = compressed.len() as f64;
        total_compressed_bytes += compressed.len() as u64;
        total_time_sec += duration;

        times_us.push(duration * 1_000_000.0);

        if orig_len > 0.0 {
            ratios.push((comp_len / orig_len) * 100.0);
        }
    }

    if total_original_bytes == 0 {
        return Ok("Total original bytes is 0. Cannot compute meaningful stats.".to_string());
    }

    let calc_stats = |data: &[f64]| -> (f64, f64, f64, f64) {
        if data.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let mut sum = 0.0;
        let mut min = f64::MAX;
        let mut max = f64::MIN;

        for &val in data {
            sum += val;
            if val < min {
                min = val;
            }
            if val > max {
                max = val;
            }
        }

        let mean = sum / data.len() as f64;

        let mut var_sum = 0.0;
        for &val in data {
            var_sum += (val - mean) * (val - mean);
        }

        let std_dev = (var_sum / data.len() as f64).sqrt();

        (mean, std_dev, min, max)
    };

    let (ratio_mean, ratio_std, ratio_min, ratio_max) = calc_stats(&ratios);
    let (time_mean, time_std, time_min, time_max) = calc_stats(&times_us);

    let overall_ratio = (total_compressed_bytes as f64 / total_original_bytes as f64) * 100.0;
    let space_saved = 100.0 - overall_ratio;

    let total_mb = total_original_bytes as f64 / (1024.0 * 1024.0);
    let throughput_mb_s = if total_time_sec > 0.0 {
        total_mb / total_time_sec
    } else {
        0.0
    };

    let report = format!(
        "--- Zstandard Compression Test Report ---\n\
         Tested Samples:         {}\n\
         Total Original Size:    {} bytes\n\
         Total Compressed Size:  {} bytes\n\
         \n\
         [ Overall Bulk Performance ]\n\
         Overall Compression:    {:.2}% of original size\n\
         Overall Space Saved:    {:.2}%\n\
         Total Bulk Time:        {:.4} seconds\n\
         Throughput:             {:.2} MB/s\n\
         \n\
         [ Per-Article Compression Ratio ]\n\
         Mean Ratio:             {:.2}%\n\
         Std Deviation:          {:.2}%\n\
         Min / Max Ratio:        {:.2}% / {:.2}%\n\
         \n\
         [ Per-Article Compression Time ]\n\
         Mean Time:              {:.2} µs\n\
         Std Deviation:          {:.2} µs\n\
         Min / Max Time:         {:.2} µs / {:.2} µs\n\
         -----------------------------------------",
        num_samples,
        total_original_bytes,
        total_compressed_bytes,
        overall_ratio,
        space_saved,
        total_time_sec,
        throughput_mb_s,
        ratio_mean,
        ratio_std,
        ratio_min,
        ratio_max,
        time_mean,
        time_std,
        time_min,
        time_max
    );

    Ok(report)
}

pub fn load_zstd_encoder_dictionary(
    dict_path: &Path,
    level: i32,
) -> io::Result<EncoderDictionary<'static>> {
    let mut file = File::open(dict_path)?;
    let mut dict_bytes = Vec::new();
    file.read_to_end(&mut dict_bytes)?;

    let prepared_dict = EncoderDictionary::copy(&dict_bytes, level);
    Ok(prepared_dict)
}

pub fn load_zstd_decoder_dictionary(dict_path: &Path) -> io::Result<DecoderDictionary<'static>> {
    let mut file = File::open(dict_path)?;
    let mut dict_bytes = Vec::new();
    file.read_to_end(&mut dict_bytes)?;

    let prepared_dict = DecoderDictionary::copy(&dict_bytes);
    Ok(prepared_dict)
}

pub fn compress_data_zstd(
    raw_data: &[u8],
    prepared_dict: &EncoderDictionary,
    window_size_kb: usize,
) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(raw_data.len() + 4);
    buffer.extend_from_slice(&(raw_data.len() as u32).to_le_bytes());
    let mut encoder = Encoder::with_prepared_dictionary(&mut buffer, prepared_dict)?;
    let window_log = (window_size_kb * 1024).ilog2() as u32;
    encoder.window_log(window_log)?;
    encoder.write_all(raw_data)?;
    encoder.finish()?;
    Ok(buffer)
}

pub fn decompress_data_zstd(
    compressed_data: &[u8],
    prepared_dict: &DecoderDictionary,
) -> io::Result<Vec<u8>> {
    let actual_zstd_data = &compressed_data[4..];
    let mut buffer = Vec::new();
    let mut decoder = Decoder::with_prepared_dictionary(actual_zstd_data, prepared_dict)?;
    decoder.read_to_end(&mut buffer)?;

    Ok(buffer)
}
