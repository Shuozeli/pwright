//! Bounded gzip, deflate and Brotli response decoding for API requests.

use std::io::{Cursor, Read};

use flate2::read::{GzDecoder, ZlibDecoder};
use pwright_api_request_transport::{ApiBodyDecodeError, ApiResponseBodyDecoder};

#[derive(Clone, Copy, Debug, Default)]
pub struct StandardBodyDecoder;

impl ApiResponseBodyDecoder for StandardBodyDecoder {
    fn decode(
        &self,
        content_encoding: Option<&str>,
        body: Vec<u8>,
        max_output_bytes: usize,
    ) -> Result<Vec<u8>, ApiBodyDecodeError> {
        if body.is_empty() {
            return Ok(body);
        }
        match content_encoding {
            Some("gzip" | "x-gzip") => {
                decode_reader("gzip", GzDecoder::new(Cursor::new(body)), max_output_bytes)
            }
            Some("deflate") => decode_reader(
                "deflate",
                ZlibDecoder::new(Cursor::new(body)),
                max_output_bytes,
            ),
            Some("br") => decode_reader(
                "br",
                brotli::Decompressor::new(Cursor::new(body), 4096),
                max_output_bytes,
            ),
            _ => Ok(body),
        }
    }
}

fn decode_reader(
    encoding: &'static str,
    reader: impl Read,
    max_output_bytes: usize,
) -> Result<Vec<u8>, ApiBodyDecodeError> {
    let read_limit = u64::try_from(max_output_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut output = Vec::new();
    reader
        .take(read_limit)
        .read_to_end(&mut output)
        .map_err(|error| {
            ApiBodyDecodeError::new(format!(
                "failed to decompress '{encoding}' encoding: {error}"
            ))
        })?;
    if output.len() > max_output_bytes {
        return Err(ApiBodyDecodeError::new(format!(
            "failed to decompress '{encoding}' encoding: decoded body exceeds {max_output_bytes} bytes"
        )));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use brotli::CompressorWriter;
    use flate2::{
        Compression,
        write::{GzEncoder, ZlibEncoder},
    };
    use pwright_api_request_transport::ApiResponseBodyDecoder;

    use super::StandardBodyDecoder;

    #[test]
    fn gzip_and_x_gzip_decode_to_original_bytes() {
        // Arrange
        let body = b"hello gzip";
        let compressed = gzip(body);
        let decoder = StandardBodyDecoder;

        // Act
        let gzip = decoder
            .decode(Some("gzip"), compressed.clone(), 1024)
            .expect("gzip should decode");
        let x_gzip = decoder
            .decode(Some("x-gzip"), compressed, 1024)
            .expect("x-gzip should decode");

        // Assert
        assert_eq!(gzip, body);
        assert_eq!(x_gzip, body);
    }

    #[test]
    fn deflate_decodes_to_original_bytes() {
        // Arrange
        let body = b"hello deflate";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body).expect("test input should compress");
        let compressed = encoder.finish().expect("deflate should finish");

        // Act
        let decoded = StandardBodyDecoder
            .decode(Some("deflate"), compressed, 1024)
            .expect("deflate should decode");

        // Assert
        assert_eq!(decoded, body);
    }

    #[test]
    fn brotli_decodes_to_original_bytes() {
        // Arrange
        let body = b"hello brotli";
        let mut compressed = Vec::new();
        {
            let mut encoder = CompressorWriter::new(&mut compressed, 4096, 5, 22);
            encoder.write_all(body).expect("test input should compress");
        }

        // Act
        let decoded = StandardBodyDecoder
            .decode(Some("br"), compressed, 1024)
            .expect("Brotli should decode");

        // Assert
        assert_eq!(decoded, body);
    }

    #[test]
    fn unknown_encoding_and_empty_supported_stream_pass_through() {
        // Arrange
        let decoder = StandardBodyDecoder;

        // Act
        let unknown = decoder
            .decode(Some("custom"), b"raw".to_vec(), 1024)
            .expect("unknown encoding should pass through");
        let empty = decoder
            .decode(Some("br"), Vec::new(), 1024)
            .expect("empty supported stream should pass through");

        // Assert
        assert_eq!(unknown, b"raw");
        assert!(empty.is_empty());
    }

    #[test]
    fn corrupt_stream_returns_encoding_specific_error() {
        // Arrange
        let decoder = StandardBodyDecoder;

        // Act
        let error = decoder
            .decode(Some("gzip"), b"not gzip".to_vec(), 1024)
            .expect_err("corrupt gzip should fail");

        // Assert
        assert!(
            error
                .to_string()
                .contains("failed to decompress 'gzip' encoding")
        );
    }

    #[test]
    fn decoded_expansion_is_rejected_at_the_public_body_limit() {
        // Arrange
        let compressed = gzip(&[b'a'; 1024]);

        // Act
        let error = StandardBodyDecoder
            .decode(Some("gzip"), compressed, 32)
            .expect_err("expanded body should exceed the limit");

        // Assert
        assert!(error.to_string().contains("decoded body exceeds 32 bytes"));
    }

    fn gzip(body: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body).expect("test input should compress");
        encoder.finish().expect("gzip should finish")
    }
}
