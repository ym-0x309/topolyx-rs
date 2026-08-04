use crate::file::{ComponentType, Domain, Semantic};

/// Error occurred while loading a Topolyx file
///
/// `#[non_exhaustive]`: new variants may be added in any `0.y` release without that being
/// treated as a breaking change. Downstream `match`es must include a wildcard arm.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
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

    // --- container-level validity (Section 5 "Container Validity") ---
    #[error(
        "container header.version ({container_version}) does not match JSON header.version major ({json_major})"
    )]
    HeaderVersionMismatch {
        container_version: u32,
        json_major: u32,
    },
    #[error("JSON header.version is not a valid \"x.y\" version string: {0:?}")]
    InvalidHeaderVersion(String),
    #[error("invalid {chunk} chunk padding byte 0x{found:02X} (expected 0x{expected:02X})")]
    InvalidPadding {
        chunk: &'static str,
        expected: u8,
        found: u8,
    },

    // --- name constraints ---
    #[error("empty {0} is not allowed")]
    EmptyName(&'static str),
    #[error("empty attribute.name is not allowed (mesh {mesh})")]
    EmptyAttributeName { mesh: usize },
    #[error("duplicate {kind}: {name:?}")]
    DuplicateName { kind: &'static str, name: String },
    #[error("duplicate attribute.name {name:?} within mesh {mesh}")]
    DuplicateAttributeName { mesh: usize, name: String },

    // --- index range ---
    #[error("object.index {index} out of range (meshes.len() == {length})")]
    ObjectMeshIndexOutOfRange {
        object: usize,
        index: usize,
        length: usize,
    },
    #[error("mesh {mesh}: {field} index {index} out of range (length {length})")]
    IndexOutOfRange {
        mesh: usize,
        field: &'static str,
        index: usize,
        length: usize,
    },

    // --- attribute data / shape constraints ---
    #[error(
        "mesh {mesh}: topology.{field} has component_type {found_type:?} x{found_count}, expected {expected_type:?} x{expected_count}"
    )]
    ComponentShapeMismatch {
        mesh: usize,
        field: &'static str,
        expected_type: ComponentType,
        expected_count: u32,
        found_type: ComponentType,
        found_count: u32,
    },
    #[error(
        "mesh {mesh}: attribute {attribute:?} has semantic {semantic:?} with an invalid shape (component_type {found_type:?} x{found_count})"
    )]
    SemanticShapeMismatch {
        mesh: usize,
        attribute: String,
        semantic: Semantic,
        found_type: ComponentType,
        found_count: u32,
    },

    // --- element_count domain matching ---
    #[error("mesh {mesh}: topology.{field}.element_count is {found}, expected {expected}")]
    TopologyElementCountMismatch {
        mesh: usize,
        field: &'static str,
        expected: u32,
        found: u32,
    },
    #[error(
        "mesh {mesh}: attribute {attribute:?} ({domain:?}).element_count is {found}, expected {expected}"
    )]
    AttributeElementCountMismatch {
        mesh: usize,
        attribute: String,
        domain: Domain,
        expected: u32,
        found: u32,
    },

    // --- coordinate system ---
    #[error("coordinate_system.{field} is {found:?}, must be {expected:?}")]
    InvalidCoordinateSystemField {
        field: &'static str,
        expected: &'static str,
        found: String,
    },
    #[error("coordinate_system.meters_per_unit must be finite and > 0, got {0}")]
    InvalidMetersPerUnit(f32),

    // --- transform validity ---
    #[error("object {object}: transform's linear 3x3 part is singular (determinant == 0)")]
    SingularTransform { object: usize },

    // --- topology structure / corner-edge consistency ---
    #[error(
        "mesh {mesh}: topology.face_offsets is not a valid partition of the corner range (must start at 0, be non-decreasing, end at corners)"
    )]
    InvalidFaceOffsets { mesh: usize },
    #[error("mesh {mesh}: corner {corner}'s corner_edges entry does not connect its corner_vertices pair")]
    CornerEdgeMismatch { mesh: usize, corner: u32 },
    #[error("mesh {mesh}: edge {edge} is a self-edge (both endpoints are vertex {vertex})")]
    SelfEdge { mesh: usize, edge: u32, vertex: u32 },
    #[error("mesh {mesh}: edges {first} and {second} are duplicates")]
    DuplicateEdge { mesh: usize, first: u32, second: u32 },

    // --- empty mesh ---
    #[error(
        "mesh {mesh} has all-zero element_counts but topology.face_offsets does not decode to exactly [0]"
    )]
    EmptyMeshFaceOffsets { mesh: usize },

    // --- convenience API (v0.4.0) ---
    #[error(
        "{field} has component_type {found_type:?} x{found_count}, expected {expected_type:?} x{expected_count}"
    )]
    UnexpectedComponentShape {
        field: &'static str,
        expected_type: ComponentType,
        expected_count: u32,
        found_type: ComponentType,
        found_count: u32,
    },
    #[error("world-space transform application is not supported for semantic {0:?}")]
    UnsupportedTransformSemantic(Semantic),
    #[error("object transform's linear 3x3 part is singular (determinant == 0)")]
    SingularObjectTransform,
    #[error("corner {corner}'s {field} index {index} is out of range (length {length})")]
    CornerIndexOutOfRange {
        field: &'static str,
        corner: usize,
        index: usize,
        length: usize,
    },
}
