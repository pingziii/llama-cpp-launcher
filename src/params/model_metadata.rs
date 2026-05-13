use std::io::Read;
use std::path::Path;

use crate::error::{AppError, Result};

/// Metadata extracted from a GGUF model file header.
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub architecture: String,
    pub parameter_count: f64,
    pub context_length: u32,
    #[allow(dead_code)]
    pub file_type: u32,
    #[allow(dead_code)]
    pub file_size_bytes: u64,
}

/// Read and parse the GGUF header to extract model metadata.
///
/// GGUF format:
///   magic: 4 bytes ("GGUF")
///   version: u32 LE
///   tensor_count: u64 LE
///   metadata_kv_count: u64 LE
///   followed by metadata KV pairs
pub fn read_gguf_metadata(path: &Path) -> Result<ModelMetadata> {
    let file_size = std::fs::metadata(path).map(|m| m.len()).map_err(|e| {
        AppError::Other(format!(
            "Cannot read file metadata for '{}': {e}",
            path.display()
        ))
    })?;

    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::Other(format!("Cannot open GGUF file '{}': {e}", path.display())))?;

    // Read header: magic(4) + version(4) + tensor_count(8) + metadata_kv_count(8) = 24 bytes
    let mut header = [0u8; 24];
    file.read_exact(&mut header).map_err(|e| {
        AppError::Other(format!(
            "Failed to read GGUF header from '{}': {e}",
            path.display()
        ))
    })?;

    // Validate magic number
    if &header[0..4] != b"GGUF" {
        return Err(AppError::Other(format!(
            "Not a valid GGUF file (bad magic): {}",
            path.display()
        )));
    }

    let _version = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let _tensor_count = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let metadata_count = u64::from_le_bytes(header[16..24].try_into().unwrap());

    let mut architecture = String::new();
    let mut parameter_count = 0.0f64;
    let mut context_length = 0u32;
    let mut file_type = 0u32;

    for _ in 0..metadata_count {
        let key = read_gguf_string(&mut file)?;
        let value_type = read_u32(&mut file)?;

        match key.as_str() {
            "general.architecture" if value_type == 8 => {
                architecture = read_gguf_string(&mut file)?;
            }
            "general.parameter_count" if value_type == 12 => {
                parameter_count = read_f64(&mut file)?;
            }
            "llama.context_length" if value_type == 4 => {
                context_length = read_u32(&mut file)?;
            }
            "general.file_type" if value_type == 4 => {
                file_type = read_u32(&mut file)?;
            }
            _ => {
                skip_gguf_value(&mut file, value_type)?;
            }
        }
    }

    Ok(ModelMetadata {
        architecture,
        parameter_count,
        context_length,
        file_type,
        file_size_bytes: file_size,
    })
}

// -- GGUF binary readers --

fn read_u32<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64<R: Read>(r: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_f64<R: Read>(r: &mut R) -> Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

/// Read a GGUF string: u64 length + UTF-8 bytes.
fn read_gguf_string<R: Read>(r: &mut R) -> Result<String> {
    let len = read_u64(r)?;
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Skip a GGUF value based on its type tag without interpreting it.
fn skip_gguf_value<R: Read>(r: &mut R, value_type: u32) -> Result<()> {
    match value_type {
        // 0-3: uint8, int8, uint16, int16
        0 | 1 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
        }
        2 | 3 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
        }
        // 4-6: uint32, int32, float32
        4..=6 => {
            let mut b = [0u8; 4];
            r.read_exact(&mut b)?;
        }
        // 7: bool
        7 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
        }
        // 8: string
        8 => {
            let len = read_u64(r)?;
            let mut buf = vec![0u8; len as usize];
            r.read_exact(&mut buf)?;
        }
        // 9: array
        9 => {
            let element_type = read_u32(r)?;
            let count = read_u64(r)?;
            for _ in 0..count {
                skip_gguf_value(r, element_type)?;
            }
        }
        // 10-12: uint64, int64, float64
        10..=12 => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b)?;
        }
        unknown => {
            return Err(AppError::Other(format!(
                "Unknown GGUF value type: {unknown}"
            )));
        }
    }
    Ok(())
}
