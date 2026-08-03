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
