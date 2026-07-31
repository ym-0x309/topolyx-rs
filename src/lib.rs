//! A crate for reading .tlyx (Topolyx) files.
//!
//! Supported format version: Topolyx 1.0. [`read_topolyx`]/[`read_topolyx_from_data`] parse the
//! container and JSON metadata; [`file::DataDescriptor::extract`] decodes the referenced BIN
//! bytes into typed [`data::ComponentData`] arrays. Full cross-field validation (index ranges,
//! name uniqueness, semantic/type matching, transform validity, ...) is scheduled for v0.3.0.

use std::path::Path;

pub mod data;
pub mod error;
pub mod file;
pub mod reader;

use error::TopolyxError;
use file::TopolyxFile;
use reader::parse_container;

/// Reads a `.tlyx` file and returns the parsed structure and the original binary data.
pub fn read_topolyx(path: impl AsRef<Path>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let data = std::fs::read(path)?;

    read_topolyx_from_data(data)
}

/// Returns a parsed structure and binary data based on the input data without reading the file.
pub fn read_topolyx_from_data(data: Vec<u8>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let (json_bytes, bin_bytes) = parse_container(&data)?;

    let file: TopolyxFile = serde_json::from_slice(json_bytes)?;

    Ok((file, bin_bytes.to_vec()))
}
