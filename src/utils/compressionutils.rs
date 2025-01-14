use super::extensions::ResultExtensions;
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

/// Compression Utilities.
pub struct CompressionUtils;

impl CompressionUtils {
    /// Compresses the specified bytes.
    pub fn write_compressed(mut bytes: Vec<u8>) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::with_capacity(512), Compression::fast());
        encoder
            .write_all(&bytes)
            .dynamic_expect(zencstr!("Failed writing bytes"));
        bytes.clear();
        drop(bytes);

        encoder
            .finish()
            .dynamic_expect(zencstr!("Failed compressing bytes"))
    }

    /// Decompresses the given bytes.
    pub fn decompress(bytes: &[u8], output: &mut Vec<u8>) {
        ZlibDecoder::new(bytes)
            .read_to_end(output)
            .dynamic_expect(zencstr!("Failed reading decompressed bytes to output"));
    }
}
