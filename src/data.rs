//! Decoding of BIN chunk bytes referenced by a `DataDescriptor` into typed component arrays.

use crate::error::TopolyxError;
use crate::file::{ComponentType, DataDescriptor};

/// Decoded, flat component values for one data array, one variant per `ComponentType`.
///
/// Length is always `component_count * element_count` of the originating `DataDescriptor`.
/// Values are not grouped per element (e.g. positions are not split into `[f32; 3]` chunks);
/// see [`ComponentData::grouped_f32`] and friends, or the higher-level
/// [`crate::grouped`]/[`crate::transform`] convenience API, for semantic-aware grouping.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentData {
    F32(Vec<f32>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I8(Vec<i8>),
    U8(Vec<u8>),
    Bool(Vec<bool>),
}

impl ComponentData {
    /// Number of decoded scalar values (`component_count * element_count`).
    pub fn len(&self) -> usize {
        match self {
            ComponentData::F32(v) => v.len(),
            ComponentData::I32(v) => v.len(),
            ComponentData::U32(v) => v.len(),
            ComponentData::I8(v) => v.len(),
            ComponentData::U8(v) => v.len(),
            ComponentData::Bool(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_f32(&self) -> Option<&[f32]> {
        match self {
            ComponentData::F32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<&[i32]> {
        match self {
            ComponentData::I32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<&[u32]> {
        match self {
            ComponentData::U32(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_i8(&self) -> Option<&[i8]> {
        match self {
            ComponentData::I8(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> Option<&[u8]> {
        match self {
            ComponentData::U8(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<&[bool]> {
        match self {
            ComponentData::Bool(v) => Some(v),
            _ => None,
        }
    }
}

impl DataDescriptor {
    /// Decodes this descriptor's referenced bytes out of `bin`.
    ///
    /// `bin` must be the BIN chunk's `chunk_data`, as returned by
    /// [`crate::read_topolyx`]/[`crate::read_topolyx_from_data`].
    pub fn extract(&self, bin: &[u8]) -> Result<ComponentData, TopolyxError> {
        if !self.byte_offset.is_multiple_of(4) {
            return Err(TopolyxError::UnalignedByteOffset(self.byte_offset));
        }

        let component_size = self.component_type.byte_size() as u64;
        let expected_len = component_size
            .checked_mul(self.component_count as u64)
            .and_then(|v| v.checked_mul(self.element_count as u64));
        if expected_len != Some(self.byte_length as u64) {
            return Err(TopolyxError::ByteLengthMismatch);
        }

        let start = self.byte_offset as usize;
        let end = start
            .checked_add(self.byte_length as usize)
            .filter(|&e| e <= bin.len())
            .ok_or(TopolyxError::DataOutOfBounds)?;
        let slice = &bin[start..end];

        Ok(match self.component_type {
            ComponentType::F32 => ComponentData::F32(decode_le::<4, _>(slice, f32::from_le_bytes)),
            ComponentType::I32 => ComponentData::I32(decode_le::<4, _>(slice, i32::from_le_bytes)),
            ComponentType::U32 => ComponentData::U32(decode_le::<4, _>(slice, u32::from_le_bytes)),
            ComponentType::I8 => ComponentData::I8(decode_le::<1, _>(slice, i8::from_le_bytes)),
            ComponentType::U8 => ComponentData::U8(slice.to_vec()),
            ComponentType::Bool => {
                let mut out = Vec::with_capacity(slice.len());
                for &b in slice {
                    out.push(match b {
                        0x00 => false,
                        0x01 => true,
                        other => return Err(TopolyxError::InvalidBoolValue(other)),
                    });
                }
                ComponentData::Bool(out)
            }
        })
    }
}

/// `slice.len()` is guaranteed to be an exact multiple of `N` by the byte_length check in
/// `extract`, so `chunks_exact` never drops a partial trailing chunk and the `try_into`
/// below can never fail.
fn decode_le<const N: usize, T>(slice: &[u8], from_le_bytes: fn([u8; N]) -> T) -> Vec<T> {
    slice
        .chunks_exact(N)
        .map(|c| from_le_bytes(c.try_into().unwrap()))
        .collect()
}
