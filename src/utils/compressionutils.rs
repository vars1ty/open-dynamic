use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

/// Compression Utilities.
pub struct CompressionUtils;

impl CompressionUtils {
    /// Compresses the specified bytes.
    pub fn write_compressed(bytes: Vec<u8>) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::with_capacity(512), Compression::fast());
        encoder
            .write_all(&bytes)
            .unwrap_or_else(|error| crash!("[ERROR] Failed writing bytes, error: ", error));
        encoder
            .finish()
            .unwrap_or_else(|error| crash!("[ERROR] Failed compressing bytes, error: ", error))
    }

    /// Decompresses the given bytes.
    pub fn decompress(bytes: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        ZlibDecoder::new(bytes)
            .read_to_end(&mut output)
            .unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed reading decompressed bytes to output, error: ",
                    error
                )
            });

        output
    }
}
