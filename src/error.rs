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
}
