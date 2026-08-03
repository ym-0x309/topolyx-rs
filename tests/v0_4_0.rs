use topolyx::data::ComponentData;
use topolyx::error::TopolyxError;
use topolyx::file::{Attribute, ComponentType, DataDescriptor, Domain, Object, Semantic, TopolyxFile};
use topolyx::grouped::AttributeValues;
use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, build_container, mutate_cube_json, valid_cube_bin};

/// Parses+reads a container built from `json`/`bin`, without running `validate()` — the
/// convenience API must not require `validate()` to have run first.
fn parse(json: &str, bin: &[u8]) -> (TopolyxFile, Vec<u8>) {
    let data = build_container(json, bin);
    read_topolyx_from_data(data).expect("container/JSON should parse")
}

fn f32_bin(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn identity_object() -> Object {
    Object {
        name: "object1".to_string(),
        object_type: "MESH".to_string(),
        index: 0,
        transform: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

// ---------------------------------------------------------------------------------------------
// Semantic-aware grouping (src/grouped.rs)
// ---------------------------------------------------------------------------------------------

#[test]
fn topology_positions_groups_into_vec3() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());

    let positions = file.meshes[0].topology.positions(&bin).unwrap();

    assert_eq!(
        positions,
        vec![
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ]
    );
}

#[test]
fn topology_edges_groups_into_pairs() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());

    let edges = file.meshes[0].topology.edges(&bin).unwrap();

    assert_eq!(
        edges,
        vec![[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]]
    );
}

#[test]
fn topology_scalar_fields_stay_flat() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let topology = &file.meshes[0].topology;

    let corner_vertices = topology.corner_vertices(&bin).unwrap();
    let corner_edges = topology.corner_edges(&bin).unwrap();
    let face_offsets = topology.face_offsets(&bin).unwrap();

    assert_eq!(corner_vertices, vec![0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 5, 4, 2, 3, 7, 6, 0, 3, 7, 4, 1, 2, 6, 5]);
    assert_eq!(corner_edges, vec![0, 1, 2, 3, 4, 5, 6, 7, 0, 9, 4, 8, 2, 11, 6, 10, 3, 11, 7, 8, 1, 10, 5, 9]);
    assert_eq!(face_offsets, vec![0, 4, 8, 12, 16, 20, 24]);
}

#[test]
fn attribute_values_dispatches_rotation_semantic() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let attribute = file.meshes[0].attribute("custom_attribute").unwrap();

    let values = attribute.values(&bin).unwrap();

    match values {
        AttributeValues::Rotation(v) => assert_eq!(v.len(), 12),
        other => panic!("expected AttributeValues::Rotation, got {other:?}"),
    }
}

#[test]
fn attribute_values_leaves_none_semantic_ungrouped() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let attribute = file.meshes[0].attribute("UVMap").unwrap();

    let values = attribute.values(&bin).unwrap();

    match values {
        AttributeValues::None(ComponentData::F32(v)) => assert_eq!(v.len(), 48), // component_count 2 * element_count 24
        other => panic!("expected AttributeValues::None(F32(_)), got {other:?}"),
    }
}

#[test]
fn shape_mismatch_is_reported_without_calling_validate() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["topology"]["positions"]["component_count"] = serde_json::json!(4);
    });
    let (file, bin) = parse(&json, &valid_cube_bin());

    let err = file.meshes[0].topology.positions(&bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::UnexpectedComponentShape {
            field: "topology.positions",
            expected_type: ComponentType::F32,
            expected_count: 3,
            found_type: ComponentType::F32,
            found_count: 4,
        }
    ));
}

#[test]
fn mesh_attribute_lookup_helpers() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let mesh = &file.meshes[0];
    let _ = &bin;

    assert_eq!(mesh.attribute("UVMap").unwrap().name, "UVMap");
    assert!(mesh.attribute("does_not_exist").is_none());

    let rotations: Vec<_> = mesh.attributes_by_semantic(Semantic::Rotation).collect();
    assert_eq!(rotations.len(), 1);
    assert_eq!(rotations[0].name, "custom_attribute");
}

// ---------------------------------------------------------------------------------------------
// Face traversal + triangulation (src/faces.rs)
// ---------------------------------------------------------------------------------------------

#[test]
fn faces_traverses_corner_vertices_and_edges_per_face() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let faces = file.meshes[0].faces(&bin).unwrap();

    assert_eq!(faces.len(), 6);

    let face2 = faces.face(2).unwrap();
    assert_eq!(face2.vertices, &[0, 1, 5, 4]);
    assert_eq!(face2.edges, &[0, 9, 4, 8]);

    let all: Vec<_> = faces.iter().map(|f| f.vertices.to_vec()).collect();
    assert_eq!(
        all,
        vec![
            vec![0, 1, 2, 3],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![2, 3, 7, 6],
            vec![0, 3, 7, 4],
            vec![1, 2, 6, 5],
        ]
    );
}

#[test]
fn triangulate_fan_splits_quad_into_two_triangles() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let faces = file.meshes[0].faces(&bin).unwrap();

    let triangles: Vec<_> = faces.face(0).unwrap().triangulate_fan().collect();

    assert_eq!(triangles, vec![[0, 1, 2], [0, 2, 3]]);
}

// ---------------------------------------------------------------------------------------------
// World-space transform application (src/transform.rs)
// ---------------------------------------------------------------------------------------------

#[test]
fn world_positions_is_a_no_op_under_the_identity_transform() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let mesh = &file.meshes[0];

    let local = mesh.topology.positions(&bin).unwrap();
    let world = mesh.world_positions(&identity_object(), &bin).unwrap();

    assert_eq!(local, world);
}

#[test]
fn world_positions_applies_non_uniform_scale_and_translation() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let mesh = &file.meshes[0];

    // L = diag(2, 1, 1), t = (1, 2, 3): world = (2x + 1, y + 2, z + 3). Chosen so every cube
    // vertex (coordinates in {-1, 1}) maps to exact integers, avoiding float epsilon comparisons.
    let object = Object {
        transform: [2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 2.0, 3.0, 1.0],
        ..identity_object()
    };

    let world = mesh.world_positions(&object, &bin).unwrap();

    assert_eq!(
        world,
        vec![
            [-1.0, 1.0, 2.0],
            [3.0, 1.0, 2.0],
            [3.0, 3.0, 2.0],
            [-1.0, 3.0, 2.0],
            [-1.0, 1.0, 4.0],
            [3.0, 1.0, 4.0],
            [3.0, 3.0, 4.0],
            [-1.0, 3.0, 4.0],
        ]
    );
}

#[test]
fn world_positions_rejects_singular_transform() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let mesh = &file.meshes[0];

    // Column-major 4x4; column 0 zeroed out makes the linear 3x3 part singular.
    let object = Object {
        transform: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ..identity_object()
    };

    let err = mesh.world_positions(&object, &bin).unwrap_err();

    assert!(matches!(err, TopolyxError::SingularObjectTransform));
}

#[test]
fn world_values_applies_direction_rule_without_translation_or_renormalization() {
    // local direction (1, 1, 0)
    let bin = f32_bin(&[1.0, 1.0, 0.0]);
    let attribute = Attribute {
        name: "dir".to_string(),
        domain: Domain::Point,
        semantic: Semantic::Direction,
        data: DataDescriptor {
            byte_offset: 0,
            byte_length: bin.len() as u32,
            component_type: ComponentType::F32,
            component_count: 3,
            element_count: 1,
        },
    };
    // L = diag(2, 1, 1); DIRECTION is L*v with no translation and no renormalization.
    let object = Object {
        transform: [2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 5.0, 5.0, 1.0],
        ..identity_object()
    };

    let values = attribute.world_values(&object, &bin).unwrap();

    match values {
        AttributeValues::Direction(v) => assert_eq!(v, vec![[2.0, 1.0, 0.0]]),
        other => panic!("expected AttributeValues::Direction, got {other:?}"),
    }
}

#[test]
fn world_values_applies_inverse_transpose_and_renormalizes_normals() {
    // local unit-ish normal (1, 1, 0)
    let bin = f32_bin(&[1.0, 1.0, 0.0]);
    let attribute = Attribute {
        name: "n".to_string(),
        domain: Domain::Point,
        semantic: Semantic::Normal,
        data: DataDescriptor {
            byte_offset: 0,
            byte_length: bin.len() as u32,
            component_type: ComponentType::F32,
            component_count: 3,
            element_count: 1,
        },
    };
    // L = diag(2, 1, 1) => transpose(inverse(L)) = diag(0.5, 1, 1).
    // raw = (0.5, 1, 0), normalized = (0.5, 1, 0) / sqrt(1.25).
    let object = Object {
        transform: [2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ..identity_object()
    };

    let values = attribute.world_values(&object, &bin).unwrap();

    match values {
        AttributeValues::Normal(v) => {
            assert_eq!(v.len(), 1);
            let [x, y, z] = v[0];
            let expected_len = (1.25f32).sqrt();
            assert!((x - 0.5 / expected_len).abs() < 1e-5, "x = {x}");
            assert!((y - 1.0 / expected_len).abs() < 1e-5, "y = {y}");
            assert_eq!(z, 0.0);
        }
        other => panic!("expected AttributeValues::Normal, got {other:?}"),
    }
}

#[test]
fn world_values_flips_tangent_handedness_under_reflection() {
    // local tangent xyz=(1,0,0), handedness w=1
    let bin = f32_bin(&[1.0, 0.0, 0.0, 1.0]);
    let attribute = Attribute {
        name: "tan".to_string(),
        domain: Domain::Point,
        semantic: Semantic::Tangent,
        data: DataDescriptor {
            byte_offset: 0,
            byte_length: bin.len() as u32,
            component_type: ComponentType::F32,
            component_count: 4,
            element_count: 1,
        },
    };
    // L = diag(-1, 1, 1): a reflection (det == -1 < 0, but not singular).
    let object = Object {
        transform: [-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ..identity_object()
    };

    let values = attribute.world_values(&object, &bin).unwrap();

    match values {
        AttributeValues::Tangent(v) => assert_eq!(v, vec![[-1.0, 0.0, 0.0, -1.0]]),
        other => panic!("expected AttributeValues::Tangent, got {other:?}"),
    }
}

#[test]
fn world_values_does_not_transform_color_or_none_semantic() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let attribute = file.meshes[0].attribute("UVMap").unwrap(); // semantic NONE

    let local = attribute.values(&bin).unwrap();
    let world = attribute.world_values(&identity_object(), &bin).unwrap();

    assert_eq!(local, world);
}

#[test]
fn world_values_rejects_rotation_semantic_as_unsupported() {
    let (file, bin) = parse(DEFAULT_CUBE_JSON, &valid_cube_bin());
    let attribute = file.meshes[0].attribute("custom_attribute").unwrap(); // semantic ROTATION

    let err = attribute.world_values(&identity_object(), &bin).unwrap_err();

    assert!(matches!(err, TopolyxError::UnsupportedTransformSemantic(Semantic::Rotation)));
}
