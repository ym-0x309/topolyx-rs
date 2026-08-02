/// Top-level structure of a Topolyx file
#[derive(serde::Deserialize, Debug)]
pub struct TopolyxFile {
    pub header: Header,
    pub coordinate_system: CoordinateSystem,
    pub objects: Vec<Object>,
    pub meshes: Vec<Mesh>,
}

/// Header of a Topolyx file
#[derive(serde::Deserialize, Debug)]
pub struct Header {
    pub format: String,  // "Topolyx"
    pub version: String, // "1.0"
}

/// Coordinate system information stored in the file. All fields except `meters_per_unit` are set to a single value.
#[derive(serde::Deserialize, Debug)]
pub struct CoordinateSystem {
    pub up_axis: String,      // "+Z"
    pub forward_axis: String, // "+Y"
    pub handedness: String,   // "RIGHT"
    pub winding: String,      // "CCW"
    pub meters_per_unit: f32, // 0 초과, 유효한 수
}

/// Object information referencing a single mesh.
#[derive(serde::Deserialize, Debug)]
pub struct Object {
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String, // "MESH"
    pub index: usize,
    pub transform: [f32; 16], // Column-Major 4*4 변환 행렬
}

/// Topology and attribute data for a single mesh
#[derive(serde::Deserialize, Debug)]
pub struct Mesh {
    pub name: String,
    pub element_counts: ElementCounts,
    pub topology: Topology,
    #[serde(default)] // JSON에 없으면 빈 Vec
    pub attributes: Vec<Attribute>,
}

/// Number of vertices, edges, faces, and face corners in a mesh
#[derive(serde::Deserialize, Debug)]
pub struct ElementCounts {
    pub vertices: u32,
    pub edges: u32,
    pub faces: u32,
    pub corners: u32,
}

/// Basic Topology Data for the Mesh
#[derive(serde::Deserialize, Debug)]
pub struct Topology {
    pub positions: DataDescriptor,
    pub edges: DataDescriptor,
    pub corner_vertices: DataDescriptor,
    pub corner_edges: DataDescriptor,
    pub face_offsets: DataDescriptor,
}

/// Attribute Data Information in BIN
#[derive(serde::Deserialize, Debug, Clone, Copy)]
pub struct DataDescriptor {
    pub byte_offset: u32,
    pub byte_length: u32,
    pub component_type: ComponentType,
    pub component_count: u32,
    pub element_count: u32,
}

/// List of currently supported attribute components
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    F32,
    I32,
    U32,
    I8,
    U8,
    #[serde(rename = "BOOL")]
    Bool,
}

impl ComponentType {
    /// Size in bytes of a single component of this type (spec section 4, component type table).
    pub const fn byte_size(self) -> usize {
        match self {
            ComponentType::F32 | ComponentType::I32 | ComponentType::U32 => 4,
            ComponentType::I8 | ComponentType::U8 | ComponentType::Bool => 1,
        }
    }
}

/// Attribute Information
#[derive(serde::Deserialize, Debug)]
pub struct Attribute {
    pub name: String,
    pub domain: Domain,
    pub data: DataDescriptor,
    #[serde(default)] // JSON에 없으면 Semantic::None
    pub semantic: Semantic,
}

/// The domain where the attribute is stored
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    #[serde(rename = "POINT")]
    Point,
    #[serde(rename = "EDGE")]
    Edge,
    #[serde(rename = "FACE")]
    Face,
    #[serde(rename = "CORNER")]
    Corner,
}

/// The assigned role of an attribute
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Semantic {
    #[serde(rename = "POSITION")]
    Position,
    #[serde(rename = "DIRECTION")]
    Direction,
    #[serde(rename = "NORMAL")]
    Normal,
    #[serde(rename = "ROTATION")]
    Rotation,
    #[serde(rename = "TANGENT")]
    Tangent,
    #[serde(rename = "COLOR")]
    Color,
    #[default]
    #[serde(rename = "NONE")]
    None,
}
