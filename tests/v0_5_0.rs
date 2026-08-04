use topolyx::data::ComponentData;
use topolyx::file::{ComponentType, DataDescriptor, ElementCounts, Mesh, Topology};
use topolyx::grouped::AttributeValues;
use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, build_container, valid_cube_bin};

// ---------------------------------------------------------------------------------------------
// Mesh::triangulate_fan_corner_indices
// ---------------------------------------------------------------------------------------------

fn mesh_with_face_offsets(face_offsets: &[u32]) -> (Mesh, Vec<u8>) {
    let bin: Vec<u8> = face_offsets.iter().flat_map(|v| v.to_le_bytes()).collect();
    let empty = |component_type: ComponentType, component_count: u32| DataDescriptor {
        byte_offset: 0,
        byte_length: 0,
        component_type,
        component_count,
        element_count: 0,
    };
    let mesh = Mesh {
        name: "m".to_string(),
        element_counts: ElementCounts {
            vertices: 0,
            edges: 0,
            faces: (face_offsets.len() - 1) as u32,
            corners: *face_offsets.last().unwrap(),
        },
        topology: Topology {
            positions: empty(ComponentType::F32, 3),
            edges: empty(ComponentType::U32, 2),
            corner_vertices: empty(ComponentType::U32, 1),
            corner_edges: empty(ComponentType::U32, 1),
            face_offsets: DataDescriptor {
                byte_offset: 0,
                byte_length: bin.len() as u32,
                component_type: ComponentType::U32,
                component_count: 1,
                element_count: face_offsets.len() as u32,
            },
        },
        attributes: vec![],
    };
    (mesh, bin)
}

#[test]
fn triangulate_fan_corner_indices_uses_corner_indices_not_vertex_indices() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let indices = mesh.triangulate_fan_corner_indices(&bin).unwrap();

    assert_eq!(indices.len(), 36);
    // Face 2 occupies corners [8, 12); corner_vertices[8..12] == [0, 1, 5, 4], so the
    // vertex-index fan (triangulate_fan_indices) would be [0, 1, 5, 0, 5, 4] here — the
    // corner-index fan must use the corner positions themselves instead.
    assert_eq!(&indices[12..18], &[8, 9, 10, 8, 10, 11]);
}

#[test]
fn triangulate_fan_corner_and_vertex_indices_describe_the_same_geometry() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let vertex_indices = mesh.triangulate_fan_indices(&bin).unwrap();
    let corner_indices = mesh.triangulate_fan_corner_indices(&bin).unwrap();
    let positions = mesh.topology.positions(&bin).unwrap();
    let corner_positions = mesh.corner_positions(&bin).unwrap();

    // Same triangle count, and each pair of indices resolves to the same position — the two
    // index buffers describe identical geometry, just against different-domain vertex buffers.
    assert_eq!(vertex_indices.len(), corner_indices.len());
    for (&v, &c) in vertex_indices.iter().zip(&corner_indices) {
        assert_eq!(positions[v as usize], corner_positions[c as usize]);
    }
}

#[test]
fn triangulate_fan_corner_indices_safely_index_a_corner_domain_attribute() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let corner_indices = mesh.triangulate_fan_corner_indices(&bin).unwrap();
    let uv = match mesh.attribute("UVMap").unwrap().values(&bin).unwrap() {
        AttributeValues::None(ComponentData::F32(v)) => v,
        other => panic!("expected AttributeValues::None(F32(_)), got {other:?}"),
    };
    let uvs: Vec<[f32; 2]> = uv.chunks_exact(2).map(|c| [c[0], c[1]]).collect();

    assert_eq!(uvs.len(), 24); // == corners, matches the corner-index domain
    for &c in &corner_indices {
        let _ = uvs[c as usize]; // in-bounds for every corner index; would panic otherwise
    }
}

#[test]
fn triangulate_fan_corner_indices_skips_degenerate_faces() {
    // Face 0 spans corners [0, 2) — only 2 corners, degenerate, contributes no triangles.
    // Face 1 spans corners [2, 5) — a valid triangle.
    let (mesh, bin) = mesh_with_face_offsets(&[0, 2, 5]);

    let indices = mesh.triangulate_fan_corner_indices(&bin).unwrap();

    assert_eq!(indices, vec![2, 3, 4]);
}

// ---------------------------------------------------------------------------------------------
// Mesh::triangulate_fan_face_indices
// ---------------------------------------------------------------------------------------------

#[test]
fn triangulate_fan_face_indices_maps_each_triangle_to_its_source_face() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let face_indices = mesh.triangulate_fan_face_indices(&bin).unwrap();

    // 6 quad faces, 2 triangles each -> each face index repeated twice, in face order.
    assert_eq!(face_indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
}

#[test]
fn triangulate_fan_face_indices_lines_up_with_the_corner_index_buffer() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let corner_indices = mesh.triangulate_fan_corner_indices(&bin).unwrap();
    let face_indices = mesh.triangulate_fan_face_indices(&bin).unwrap();

    // face_indices[i] corresponds to corner_indices[3*i..3*i+3].
    assert_eq!(face_indices.len() * 3, corner_indices.len());
}

#[test]
fn triangulate_fan_face_indices_skips_degenerate_faces() {
    // Face 0: corners [0, 2) — degenerate, skipped, contributes no entries.
    // Face 1: corners [2, 5) — one triangle.
    // Face 2: corners [5, 9) — a quad, two triangles.
    let (mesh, bin) = mesh_with_face_offsets(&[0, 2, 5, 9]);

    let face_indices = mesh.triangulate_fan_face_indices(&bin).unwrap();

    assert_eq!(face_indices, vec![1, 2, 2]);
}
