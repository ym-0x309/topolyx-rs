use std::path::Path;

pub mod error;
pub mod reader;
pub mod file;

use error::TopolyxError;
use reader::parse_container;
use file::TopolyxFile;

pub fn read_topolyx(path: impl AsRef<Path>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let data = std::fs::read(path)?;

    read_topolyx_from_data(data)
}

pub fn read_topolyx_from_data(data: Vec<u8>) -> Result<(TopolyxFile, Vec<u8>), TopolyxError> {
    let (json_bytes, bin_bytes) = parse_container(&data)?;

    let file: TopolyxFile = serde_json::from_slice(json_bytes)?;

    Ok((file, bin_bytes.to_vec()))
}
