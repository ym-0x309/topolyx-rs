//! A crate for reading .tlyx (Topolyx) files.
//!
//! Supported format version: Topolyx 1.0. [`read_topolyx`]/[`read_topolyx_from_data`] parse the
//! container and JSON metadata, enforcing the container-level rules of spec section 5
//! ("Container Validity"); [`file::DataDescriptor::extract`] decodes the referenced BIN bytes
//! into typed [`data::ComponentData`] arrays. The remaining spec section 5 rules (index ranges,
//! name uniqueness, semantic/type matching, transform validity, ...) are not checked
//! automatically by these functions — call [`file::TopolyxFile::validate`] explicitly once the
//! full spec-validity guarantee is needed.
//!
//! On top of that raw decoding layer, a convenience API groups flat [`data::ComponentData`]
//! into typed, per-element arrays and applies `object.transform` to convert values to world
//! space:
//!
//! - [`grouped`]: semantic-aware grouping ([`file::Topology::positions`],
//!   [`file::Attribute::values`], ...).
//! - [`faces`]: per-face corner traversal ([`file::Mesh::faces`]), naive fan triangulation as
//!   vertex indices ([`file::Mesh::triangulate_fan_indices`]), corner indices
//!   ([`file::Mesh::triangulate_fan_corner_indices`]), or originating face indices
//!   ([`file::Mesh::triangulate_fan_face_indices`]), and reindexing `POINT`-domain data to
//!   per-corner ([`file::Mesh::corner_positions`], [`file::Mesh::world_corner_positions`]) —
//!   see the module docs for which index buffer pairs with which vertex-buffer layout and
//!   domain (`EDGE`-domain attributes cannot be resolved post-triangulation at all).
//! - [`transform`]: world-space conversion ([`file::Mesh::world_positions`],
//!   [`file::Attribute::world_values`]) per spec section 4, "Object Transform Application
//!   Rules". `ROTATION` is not supported yet — see the module docs.

use std::path::Path;

pub mod data;
pub mod error;
pub mod faces;
pub mod file;
pub mod grouped;
pub mod reader;
pub mod transform;
pub mod validate;

use error::TopolyxError;
use file::TopolyxFile;
use reader::{check_header_version, check_json_padding, parse_container};

/// Reads a `.tlyx` file and returns the parsed structure and the original binary data.
pub fn read_topolyx(path: impl AsRef<Path>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let data = std::fs::read(path)?;

    read_topolyx_from_data(&data)
}

/// Returns a parsed structure and binary data based on the input data without reading the file.
pub fn read_topolyx_from_data(data: &Vec<u8>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let (container_version, json_bytes, bin_bytes) = parse_container(&data)?;

    // `into_iter` (rather than plain `from_slice`) is used so `byte_offset()` reports exactly
    // where the JSON value ends, letting the tail be checked as spec-mandated `0x20` padding
    // instead of arbitrary JSON whitespace.
    let mut json_stream = serde_json::Deserializer::from_slice(json_bytes).into_iter::<TopolyxFile>();
    let file = json_stream
        .next()
        .unwrap_or_else(|| serde_json::from_slice(b""))?;
    check_json_padding(json_bytes, json_stream.byte_offset())?;
    check_header_version(container_version, &file.header.version)?;

    Ok((file, bin_bytes.to_vec()))
}
