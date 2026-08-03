//! Semantic-aware grouping of decoded [`ComponentData`] into typed, per-element arrays.
//!
//! [`data::ComponentData`](crate::data::ComponentData) is a flat, ungrouped scalar buffer
//! (`component_count * element_count` values). This module groups that buffer into
//! `Vec<[T; N]>` (one array per element) using the shapes fixed by SPECIFICATION.md section 4
//! for topology fields (`Topology::positions`, `Topology::edges`, ...) and for attribute
//! `semantic` values ([`Attribute::values`]).

use crate::data::ComponentData;
use crate::error::TopolyxError;
use crate::file::{Attribute, ComponentType, DataDescriptor, Mesh, Semantic, Topology};

impl ComponentData {
    /// Groups this flat buffer into `N`-component elements.
    ///
    /// Returns `None` if this is not an `F32` buffer, or its length is not a multiple of `N`.
    pub fn grouped_f32<const N: usize>(&self) -> Option<Vec<[f32; N]>> {
        group(self.as_f32()?)
    }
    /// See [`ComponentData::grouped_f32`].
    pub fn grouped_i32<const N: usize>(&self) -> Option<Vec<[i32; N]>> {
        group(self.as_i32()?)
    }
    /// See [`ComponentData::grouped_f32`].
    pub fn grouped_u32<const N: usize>(&self) -> Option<Vec<[u32; N]>> {
        group(self.as_u32()?)
    }
    /// See [`ComponentData::grouped_f32`].
    pub fn grouped_i8<const N: usize>(&self) -> Option<Vec<[i8; N]>> {
        group(self.as_i8()?)
    }
    /// See [`ComponentData::grouped_f32`].
    pub fn grouped_u8<const N: usize>(&self) -> Option<Vec<[u8; N]>> {
        group(self.as_u8()?)
    }
    /// See [`ComponentData::grouped_f32`].
    pub fn grouped_bool<const N: usize>(&self) -> Option<Vec<[bool; N]>> {
        group(self.as_bool()?)
    }
}

fn group<T: Copy, const N: usize>(slice: &[T]) -> Option<Vec<[T; N]>> {
    if N == 0 || !slice.len().is_multiple_of(N) {
        return None;
    }
    Some(slice.chunks_exact(N).map(|c| c.try_into().unwrap()).collect())
}

impl DataDescriptor {
    fn check_shape(
        &self,
        field: &'static str,
        expected_type: ComponentType,
        expected_count: u32,
    ) -> Result<(), TopolyxError> {
        if self.component_type != expected_type || self.component_count != expected_count {
            return Err(TopolyxError::UnexpectedComponentShape {
                field,
                expected_type,
                expected_count,
                found_type: self.component_type,
                found_count: self.component_count,
            });
        }
        Ok(())
    }

    /// Decodes and groups this descriptor's data, checking it is `F32` x `N` first.
    fn extract_f32<const N: usize>(&self, bin: &[u8], field: &'static str) -> Result<Vec<[f32; N]>, TopolyxError> {
        self.check_shape(field, ComponentType::F32, N as u32)?;
        Ok(self.extract(bin)?.grouped_f32::<N>().expect("shape checked above"))
    }

    /// Decodes and groups this descriptor's data, checking it is `U32` x `N` first.
    fn extract_u32<const N: usize>(&self, bin: &[u8], field: &'static str) -> Result<Vec<[u32; N]>, TopolyxError> {
        self.check_shape(field, ComponentType::U32, N as u32)?;
        Ok(self.extract(bin)?.grouped_u32::<N>().expect("shape checked above"))
    }

    /// Decodes and groups this descriptor's data, checking it is `U8` x `N` first.
    fn extract_u8<const N: usize>(&self, bin: &[u8], field: &'static str) -> Result<Vec<[u8; N]>, TopolyxError> {
        self.check_shape(field, ComponentType::U8, N as u32)?;
        Ok(self.extract(bin)?.grouped_u8::<N>().expect("shape checked above"))
    }

    /// Decodes this descriptor's data as a flat `U32` scalar array, checking `component_count == 1` first.
    fn extract_u32_scalar(&self, bin: &[u8], field: &'static str) -> Result<Vec<u32>, TopolyxError> {
        Ok(self.extract_u32::<1>(bin, field)?.into_iter().map(|[v]| v).collect())
    }
}

impl Topology {
    /// The local-space position of each vertex (SPECIFICATION.md section 4, `positions`).
    pub fn positions(&self, bin: &[u8]) -> Result<Vec<[f32; 3]>, TopolyxError> {
        self.positions.extract_f32::<3>(bin, "topology.positions")
    }

    /// The two vertex indices that make up each edge (SPECIFICATION.md section 4, `edges`).
    pub fn edges(&self, bin: &[u8]) -> Result<Vec<[u32; 2]>, TopolyxError> {
        self.edges.extract_u32::<2>(bin, "topology.edges")
    }

    /// The vertex referenced by each face corner (SPECIFICATION.md section 4, `corner_vertices`).
    pub fn corner_vertices(&self, bin: &[u8]) -> Result<Vec<u32>, TopolyxError> {
        self.corner_vertices.extract_u32_scalar(bin, "topology.corner_vertices")
    }

    /// The edge leading from each face corner to the next corner of the same face
    /// (SPECIFICATION.md section 4, `corner_edges`).
    pub fn corner_edges(&self, bin: &[u8]) -> Result<Vec<u32>, TopolyxError> {
        self.corner_edges.extract_u32_scalar(bin, "topology.corner_edges")
    }

    /// The starting corner index of each face's corner range, plus a trailing total corner
    /// count (SPECIFICATION.md section 4, `face_offsets`). Always `faces + 1` elements long.
    pub fn face_offsets(&self, bin: &[u8]) -> Result<Vec<u32>, TopolyxError> {
        self.face_offsets.extract_u32_scalar(bin, "topology.face_offsets")
    }
}

/// Grouped attribute values, shaped according to [`Attribute::semantic`]
/// (SPECIFICATION.md section 4, `attribute.semantic` table).
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValues {
    Position(Vec<[f32; 3]>),
    Direction(Vec<[f32; 3]>),
    Normal(Vec<[f32; 3]>),
    Rotation(Vec<[f32; 4]>),
    Tangent(Vec<[f32; 4]>),
    ColorF32(Vec<[f32; 4]>),
    ColorU8(Vec<[u8; 4]>),
    /// `semantic == NONE`: shape is not fixed by the spec, so the data is left ungrouped.
    /// Callers who know the attribute's `component_count` can group it themselves via
    /// [`ComponentData::grouped_f32`] and friends.
    None(ComponentData),
}

impl Attribute {
    /// Decodes and groups this attribute's data according to its `semantic`.
    pub fn values(&self, bin: &[u8]) -> Result<AttributeValues, TopolyxError> {
        const FIELD: &str = "attribute.data";

        Ok(match self.semantic {
            Semantic::Position => AttributeValues::Position(self.data.extract_f32::<3>(bin, FIELD)?),
            Semantic::Direction => AttributeValues::Direction(self.data.extract_f32::<3>(bin, FIELD)?),
            Semantic::Normal => AttributeValues::Normal(self.data.extract_f32::<3>(bin, FIELD)?),
            Semantic::Rotation => AttributeValues::Rotation(self.data.extract_f32::<4>(bin, FIELD)?),
            Semantic::Tangent => AttributeValues::Tangent(self.data.extract_f32::<4>(bin, FIELD)?),
            Semantic::Color if self.data.component_type == ComponentType::U8 => {
                AttributeValues::ColorU8(self.data.extract_u8::<4>(bin, FIELD)?)
            }
            Semantic::Color => AttributeValues::ColorF32(self.data.extract_f32::<4>(bin, FIELD)?),
            Semantic::None => AttributeValues::None(self.data.extract(bin)?),
        })
    }
}

impl Mesh {
    /// Finds this mesh's attribute with the given name, if any (`attribute.name` is unique
    /// within a mesh per SPECIFICATION.md section 5, "Name Constraints" — but that is only
    /// guaranteed once [`TopolyxFile::validate`](crate::file::TopolyxFile::validate) has run).
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    /// Iterates this mesh's attributes with the given `semantic`. The spec does not guarantee
    /// a `semantic` is used by at most one attribute, so this may yield more than one match.
    pub fn attributes_by_semantic(&self, semantic: Semantic) -> impl Iterator<Item = &Attribute> {
        self.attributes.iter().filter(move |a| a.semantic == semantic)
    }
}
