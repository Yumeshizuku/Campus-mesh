//! CM2 compresses encrypted envelope JSON (including repetitive headers/hex),
//! never plaintext. Decompression is bounded before JSON/signature processing.
use crate::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::io::{Read, Write};
pub fn encode(frame: &Frame) -> Result<String> {
    let bytes = serde_json::to_vec(frame).map_err(err)?;
    if bytes.len() > MAX_FRAME {
        return Err("frame limit".into());
    }
    let mut compressor = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    compressor.write_all(&bytes).map_err(err)?;
    let compressed = compressor.finish().map_err(err)?;
    let mut wire = b"CM2".to_vec();
    wire.extend(compressed);
    if wire.len() > MAX_FRAME {
        return Err("wire limit".into());
    }
    Ok(STANDARD.encode(wire))
}
pub fn decode(encoded: &str) -> Result<Frame> {
    if encoded.len() > MAX_FRAME.div_ceil(3) * 4 {
        return Err("wire limit".into());
    }
    let wire = STANDARD.decode(encoded).map_err(err)?;
    if wire.len() > MAX_FRAME {
        return Err("wire limit".into());
    }
    let plain = if wire.starts_with(b"CM2") {
        let mut output = Vec::new();
        flate2::read::ZlibDecoder::new(&wire[3..])
            .take((MAX_FRAME + 1) as u64)
            .read_to_end(&mut output)
            .map_err(err)?;
        output
    } else {
        wire
    };
    if plain.len() > MAX_FRAME {
        return Err("decompressed frame limit".into());
    }
    serde_json::from_slice(&plain).map_err(err)
}
