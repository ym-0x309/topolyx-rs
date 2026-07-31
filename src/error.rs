/// Error occurred while loading a Topolyx file
#[derive(thiserror::Error, Debug)]
pub enum TopolyxError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic bytes")]
    InvalidMagic,
    #[error("unsupported container version: {0}")]
    UnsupportedContainerVersion(u32),
    #[error("malformed container")]
    MalformedContainer,
    #[error("total_length mismatch")]
    TotalLengthMismatch,
    #[error("json error:: {0}")]
    Json(#[from] serde_json::Error),
    #[error("byte_offset is not 4-byte aligned: {0}")]
    UnalignedByteOffset(u32),
    #[error("byte_length does not match component_size * component_count * element_count")]
    ByteLengthMismatch,
    #[error("data descriptor references bytes outside the BIN chunk")]
    DataOutOfBounds,
    #[error("invalid BOOL byte value: {0}")]
    InvalidBoolValue(u8),
}
