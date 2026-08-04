use topolyx::error::TopolyxError;
use topolyx::faces::Face;
use topolyx::file::Object;
use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, build_container, valid_cube_bin};

fn identity_object() -> Object {
    Object {
        name: "object1".to_string(),
        object_type: "MESH".to_string(),
        index: 0,
        transform: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

// ---------------------------------------------------------------------------------------------
// Mesh::triangulate_fan_indices
// ---------------------------------------------------------------------------------------------

#[test]
fn triangulate_fan_indices_flattens_every_face_of_the_cube() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let indices = mesh.triangulate_fan_indices(&bin).unwrap();

    // 6 quad faces * 2 triangles * 3 indices = 36.
    assert_eq!(indices.len(), 36);
    // First face is corner_vertices[0..4] == [0, 1, 2, 3] -> fan triangles (0,1,2), (0,2,3).
    assert_eq!(&indices[0..6], &[0, 1, 2, 0, 2, 3]);
}

#[test]
fn triangulate_fan_does_not_panic_on_degenerate_faces() {
    let empty = Face { vertices: &[], edges: &[] };
    assert_eq!(empty.triangulate_fan().collect::<Vec<_>>(), Vec::<[u32; 3]>::new());

    let one = Face { vertices: &[5], edges: &[10] };
    assert_eq!(one.triangulate_fan().collect::<Vec<_>>(), Vec::<[u32; 3]>::new());

    let two = Face { vertices: &[5, 6], edges: &[10, 11] };
    assert_eq!(two.triangulate_fan().collect::<Vec<_>>(), Vec::<[u32; 3]>::new());

    let triangle = Face { vertices: &[5, 6, 7], edges: &[10, 11, 12] };
    assert_eq!(triangle.triangulate_fan().collect::<Vec<_>>(), vec![[5, 6, 7]]);
}

// ---------------------------------------------------------------------------------------------
// Mesh::corner_positions / world_corner_positions
// ---------------------------------------------------------------------------------------------

#[test]
fn corner_positions_reindexes_positions_by_corner_vertices() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let positions = mesh.topology.positions(&bin).unwrap();
    let corner_vertices = mesh.topology.corner_vertices(&bin).unwrap();
    let expected: Vec<[f32; 3]> = corner_vertices.iter().map(|&i| positions[i as usize]).collect();

    let actual = mesh.corner_positions(&bin).unwrap();

    assert_eq!(actual.len(), 24); // corners count
    assert_eq!(actual, expected);
}

#[test]
fn corner_positions_out_of_range_index_is_reported_without_panicking() {
    let mut bin = valid_cube_bin();
    bin[192..196].copy_from_slice(&99u32.to_le_bytes()); // corner_vertices[0] = 99; only 8 vertices exist
    let data = build_container(DEFAULT_CUBE_JSON, &bin);
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    let err = mesh.corner_positions(&bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::CornerIndexOutOfRange { field: "topology.positions", corner: 0, index: 99, length: 8 }
    ));
}

#[test]
fn world_corner_positions_combines_transform_and_reindexing() {
    let data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let (file, bin) = read_topolyx_from_data(&data).unwrap();
    let mesh = &file.meshes[0];

    // Same transform as the v0.4.0 world_positions tests: L = diag(2, 1, 1), t = (1, 2, 3).
    let object = Object {
        transform: [2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 2.0, 3.0, 1.0],
        ..identity_object()
    };

    let world_positions = mesh.world_positions(&object, &bin).unwrap();
    let corner_vertices = mesh.topology.corner_vertices(&bin).unwrap();
    let expected: Vec<[f32; 3]> = corner_vertices.iter().map(|&i| world_positions[i as usize]).collect();

    let actual = mesh.world_corner_positions(&object, &bin).unwrap();

    assert_eq!(actual, expected);
    // Spot check corner 0 (vertex 0 = [-1,-1,-1] -> world (2*-1+1, -1+2, -1+3) = (-1, 1, 2)).
    assert_eq!(actual[0], [-1.0, 1.0, 2.0]);
}
