// shared/src/compression.rs
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
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

    let dictionary = zstd::dict::from_samples(samples, target_dict_size)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut file = File::create(output_path)?;
    file.write_all(&dictionary)?;

    println!("Successfully saved dictionary to {:?}", output_path);

    Ok(())
}

/// Loads the dictionary from disk and prepares it for fast, repeated compression.
/// Call this ONCE at the start of your mass-processing phase.
pub fn load_zstd_encoder_dictionary(
    dict_path: &Path,
    level: i32,
) -> io::Result<EncoderDictionary<'static>> {
    let mut file = File::open(dict_path)?;
    let mut dict_bytes = Vec::new();
    file.read_to_end(&mut dict_bytes)?;

    // This parses the dictionary mathematically so it doesn't have to be recalculated
    let prepared_dict = EncoderDictionary::copy(&dict_bytes, level);
    Ok(prepared_dict)
}

/// Loads the dictionary for decompression (useful for testing on the desktop).
pub fn load_zstd_decoder_dictionary(dict_path: &Path) -> io::Result<DecoderDictionary<'static>> {
    let mut file = File::open(dict_path)?;
    let mut dict_bytes = Vec::new();
    file.read_to_end(&mut dict_bytes)?;

    let prepared_dict = DecoderDictionary::copy(&dict_bytes);
    Ok(prepared_dict)
}

/// Compresses a single article in memory.
/// Notice how it takes a byte slice (`&[u8]`), avoiding slow file paths entirely.
pub fn compress_data_zstd(
    raw_data: &[u8],
    prepared_dict: &EncoderDictionary,
    window_size_kb: usize,
) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(raw_data.len()); // Pre-allocate to save time

    let mut encoder = Encoder::with_prepared_dictionary(&mut buffer, prepared_dict)?;

    // CRITICAL: We must enforce the maximum sliding window size for the ESP32!
    // zstd window_log is a power of 2. 32 KB = 2^15 bytes.
    let window_log = (window_size_kb * 1024).ilog2() as u32;
    encoder.window_log(window_log)?;

    encoder.write_all(raw_data)?;
    encoder.finish()?;

    Ok(buffer)
}

/// Decompresses raw binary data from memory.
/// This perfectly mimics how the ESP32 will behave when it reads SD card sectors into RAM.
pub fn decompress_data_zstd(
    compressed_data: &[u8],
    prepared_dict: &DecoderDictionary,
) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    // The decoder only needs a slice of memory (compressed_data)
    let mut decoder = Decoder::with_prepared_dictionary(compressed_data, prepared_dict)?;
    decoder.read_to_end(&mut buffer)?;

    Ok(buffer)
}
