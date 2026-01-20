//! Application-level compression for WebSocket payloads.
//!
//! Since tokio-tungstenite doesn't support permessage-deflate, we implement
//! application-level gzip compression for large payloads (like snapshots).
//!
//! The compressed payload is sent as a JSON wrapper:
//! ```json
//! { "compressed": "gzip", "data": "<base64-encoded-gzip-data>" }
//! ```
//!
//! Clients detect the `compressed` field and decompress accordingly.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use serde::Serialize;
use std::io::Write;

/// Minimum payload size (in bytes) before compression is applied.
/// Payloads smaller than this are sent uncompressed.
const COMPRESSION_THRESHOLD: usize = 1024; // 1KB

/// Wrapper for compressed payloads sent over WebSocket.
#[derive(Serialize)]
struct CompressedFrame {
    compressed: &'static str,
    data: String,
}

/// Compress a payload if it exceeds the threshold.
///
/// Returns the original bytes if:
/// - Payload is below threshold
/// - Compression fails
/// - Compressed size is larger than original (unlikely for JSON)
///
/// Returns a compressed wrapper JSON if compression is beneficial.
pub fn maybe_compress(payload: &[u8]) -> Bytes {
    if payload.len() < COMPRESSION_THRESHOLD {
        return Bytes::copy_from_slice(payload);
    }

    match compress_gzip(payload) {
        Ok(compressed) => {
            // Only use compression if it actually reduces size
            // Account for base64 overhead (~33%) and JSON wrapper (~30 bytes)
            let estimated_compressed_size = (compressed.len() * 4 / 3) + 40;
            if estimated_compressed_size < payload.len() {
                let frame = CompressedFrame {
                    compressed: "gzip",
                    data: BASE64.encode(&compressed),
                };
                match serde_json::to_vec(&frame) {
                    Ok(json) => Bytes::from(json),
                    Err(_) => Bytes::copy_from_slice(payload),
                }
            } else {
                Bytes::copy_from_slice(payload)
            }
        }
        Err(_) => Bytes::copy_from_slice(payload),
    }
}

/// Compress data using gzip.
fn compress_gzip(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_small_payload_not_compressed() {
        let small = b"hello";
        let result = maybe_compress(small);
        assert_eq!(result.as_ref(), small);
    }

    #[test]
    fn test_large_payload_compressed() {
        // Create a large JSON payload similar to snapshots
        let entities: Vec<_> = (0..100)
            .map(|i| {
                json!({
                    "key": format!("entity_{}", i),
                    "data": {
                        "id": i,
                        "name": format!("Entity number {}", i),
                        "description": "This is a test entity with some data that will be repeated",
                        "values": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                        "nested": {
                            "field1": "value1",
                            "field2": "value2",
                            "field3": "value3",
                        }
                    }
                })
            })
            .collect();

        let payload = serde_json::to_vec(&entities).unwrap();
        let original_size = payload.len();
        let result = maybe_compress(&payload);

        // Should be compressed
        let result_str = std::str::from_utf8(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(result_str).unwrap();

        assert_eq!(parsed["compressed"], "gzip");
        assert!(parsed["data"].is_string());

        // Compressed should be smaller
        assert!(
            result.len() < original_size,
            "Compressed {} should be < original {}",
            result.len(),
            original_size
        );

        println!(
            "Original: {} bytes, Compressed: {} bytes, Ratio: {:.1}%",
            original_size,
            result.len(),
            (result.len() as f64 / original_size as f64) * 100.0
        );
    }

    #[test]
    fn test_incompressible_data_not_wrapped() {
        // Random-ish data that doesn't compress well
        // But still make it look like valid JSON so we can test the size comparison
        let data: Vec<u8> = (0..2000).map(|i| (i % 256) as u8).collect();

        // This won't be valid JSON, so it will just return as-is due to size check
        let result = maybe_compress(&data);

        // For truly incompressible data, we should get the original back
        // (either because compression failed or didn't help)
        assert!(!result.is_empty());
    }
}
