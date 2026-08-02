use crate::error::TopolyxError;

/// Get the entire file and split it into the container version, a JSON part, and a BIN part.
pub fn parse_container(data: &[u8]) -> Result<(u32, &[u8], &[u8]), TopolyxError> {
    // 헤더 확인
    let header = data.get(0..12).ok_or(TopolyxError::MalformedContainer)?;

    let magic = &header[0..4];
    if magic != b"TLYX" {
        return Err(TopolyxError::InvalidMagic);
    }

    let version = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if version != 1 {
        return Err(TopolyxError::UnsupportedContainerVersion(version));
    }

    let total_length = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
    if total_length != data.len() {
        return Err(TopolyxError::TotalLengthMismatch);
    }

    // 청크 0 (JSON)
    let (json_bytes, offset) = read_chunk(data, 12, b"JSON")?;

    // 청크 1 (BIN)
    let (bin_bytes, _) = read_chunk(data, offset, b"BIN\0")?;

    Ok((version, json_bytes, bin_bytes))
}

/// Checks that every byte of `json_bytes` after the `consumed` bytes of actual JSON content
/// is the spec-mandated `0x20` padding byte (spec section 5, "Container Validity").
pub(crate) fn check_json_padding(json_bytes: &[u8], consumed: usize) -> Result<(), TopolyxError> {
    for &byte in &json_bytes[consumed..] {
        if byte != b' ' {
            return Err(TopolyxError::InvalidPadding {
                chunk: "JSON",
                expected: b' ',
                found: byte,
            });
        }
    }
    Ok(())
}

/// Checks that the container's major version matches the `x` of the JSON `header.version`
/// (`"x.y"`) field (spec section 5, "Container Validity").
pub(crate) fn check_header_version(
    container_version: u32,
    json_version: &str,
) -> Result<(), TopolyxError> {
    let json_major: u32 = json_version
        .split_once('.')
        .and_then(|(major, _)| major.parse().ok())
        .ok_or_else(|| TopolyxError::InvalidHeaderVersion(json_version.to_string()))?;

    if container_version != json_major {
        return Err(TopolyxError::HeaderVersionMismatch {
            container_version,
            json_major,
        });
    }

    Ok(())
}

/// Extract a chunk from the entire file based on the offset.
fn read_chunk<'a>(
    data: &'a [u8],
    offset: usize,
    expected_type: &[u8; 4],
) -> Result<(&'a [u8], usize), TopolyxError> {
    // 헤더 얻기, 검사
    let chunk_header = data
        .get(offset..offset + 8)
        .ok_or(TopolyxError::MalformedContainer)?;
    let chunk_length = u32::from_le_bytes(chunk_header[0..4].try_into().unwrap()) as usize;
    let chunk_type = &chunk_header[4..8];

    if chunk_type != expected_type {
        return Err(TopolyxError::MalformedContainer);
    }
    if !chunk_length.is_multiple_of(4) {
        return Err(TopolyxError::MalformedContainer);
    }

    // 데이터 얻기
    let data_start = offset + 8;
    let data_end = data_start
        .checked_add(chunk_length)
        .filter(|&e| e <= data.len())
        .ok_or(TopolyxError::MalformedContainer)?;

    Ok((&data[data_start..data_end], data_end))
}
