use serde_json::json;
use topolyx::error::TopolyxError;
use topolyx::read_topolyx_from_data;

mod common;
use common::{DEFAULT_CUBE_JSON, EMPTY_MESH_JSON, build_container, mutate_cube_json, valid_cube_bin};

/// Parses+reads a container built from `json`/`bin`, then runs the full `validate()` pass.
fn read_and_validate(json: &str, bin: &[u8]) -> Result<(), TopolyxError> {
    let data = build_container(json, bin);
    let (file, bin) = read_topolyx_from_data(&data).expect("container/JSON should parse");
    file.validate(&bin)
}

// ---------------------------------------------------------------------------------------------
// Cross-cutting
// ---------------------------------------------------------------------------------------------

#[test]
fn valid_cube_fixture_passes_full_validation() {
    let result = read_and_validate(DEFAULT_CUBE_JSON, &valid_cube_bin());

    assert!(result.is_ok(), "expected valid cube fixture to pass validate(), got {result:?}");
}

#[test]
fn validate_surfaces_existing_data_rs_errors_for_attributes_no_semantic_rule_touches() {
    // UVMap has no semantic (semantic == NONE), so no section-5 shape/semantic rule looks at
    // it directly — but validate() should still surface the byte_length mismatch data.rs would
    // report, since it extracts every descriptor in the file.
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["attributes"][1]["data"]["byte_length"] = json!(191);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::ByteLengthMismatch));
}

// ---------------------------------------------------------------------------------------------
// Container validity (mandatory path, via read_topolyx_from_data)
// ---------------------------------------------------------------------------------------------

#[test]
fn container_version_mismatching_json_header_major_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["header"]["version"] = json!("2.0");
    });
    let data = build_container(&json, &valid_cube_bin());

    let err = read_topolyx_from_data(&data).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::HeaderVersionMismatch { container_version: 1, json_major: 2 }
    ));
}

#[test]
fn json_header_version_with_invalid_format_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["header"]["version"] = json!("not-a-number");
    });
    let data = build_container(&json, &valid_cube_bin());

    let err = read_topolyx_from_data(&data).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidHeaderVersion(_)));
}

#[test]
fn json_chunk_padding_byte_other_than_space_is_rejected() {
    // Hand-build a container whose JSON chunk's alignment padding uses a tab instead of 0x20.
    let mut json_bytes = DEFAULT_CUBE_JSON.as_bytes().to_vec();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b'\t');
    }
    assert_ne!(
        json_bytes.len(),
        DEFAULT_CUBE_JSON.len(),
        "fixture must actually need padding for this test to be meaningful"
    );

    let bin_bytes = valid_cube_bin();
    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    let mut data = Vec::with_capacity(total_length);
    data.extend_from_slice(b"TLYX");
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&(total_length as u32).to_le_bytes());
    data.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(b"JSON");
    data.extend_from_slice(&json_bytes);
    data.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(b"BIN\0");
    data.extend_from_slice(&bin_bytes);

    let err = read_topolyx_from_data(&data).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::InvalidPadding { chunk: "JSON", expected: b' ', found: b'\t' }
    ));
}

#[test]
fn trailing_bytes_after_bin_chunk_are_rejected() {
    // Append 4 extra bytes after the BIN chunk, and fix up `total_length` to match the new
    // (bogus) file length so this doesn't just get rejected as a TotalLengthMismatch instead.
    let mut data = build_container(DEFAULT_CUBE_JSON, &valid_cube_bin());
    data.extend_from_slice(&[0u8; 4]);
    let new_total_length = data.len() as u32;
    data[8..12].copy_from_slice(&new_total_length.to_le_bytes());

    let err = read_topolyx_from_data(&data).unwrap_err();

    assert!(matches!(err, TopolyxError::TrailingBytes(4)));
}

// ---------------------------------------------------------------------------------------------
// Index range
// ---------------------------------------------------------------------------------------------

#[test]
fn object_index_out_of_range_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["objects"][0]["index"] = json!(5);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::ObjectMeshIndexOutOfRange { object: 0, index: 5, length: 1 }
    ));
}

#[test]
fn edge_vertex_index_out_of_range_is_rejected() {
    let mut bin = valid_cube_bin();
    bin[96..100].copy_from_slice(&99u32.to_le_bytes()); // edges[0] = (99, 1); only 8 vertices exist

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::IndexOutOfRange { field: "edges", index: 99, length: 8, .. }
    ));
}

#[test]
fn corner_vertices_index_out_of_range_is_rejected() {
    let mut bin = valid_cube_bin();
    bin[192..196].copy_from_slice(&99u32.to_le_bytes()); // corner_vertices[0] = 99

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::IndexOutOfRange { field: "corner_vertices", index: 99, length: 8, .. }
    ));
}

#[test]
fn corner_edges_index_out_of_range_is_rejected() {
    let mut bin = valid_cube_bin();
    bin[288..292].copy_from_slice(&99u32.to_le_bytes()); // corner_edges[0] = 99

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::IndexOutOfRange { field: "corner_edges", index: 99, length: 12, .. }
    ));
}

// ---------------------------------------------------------------------------------------------
// Name constraints
// ---------------------------------------------------------------------------------------------

#[test]
fn empty_object_name_is_rejected() {
    let json = mutate_cube_json(|v| v["objects"][0]["name"] = json!(""));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::EmptyName("object.name")));
}

#[test]
fn empty_mesh_name_is_rejected() {
    let json = mutate_cube_json(|v| v["meshes"][0]["name"] = json!(""));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::EmptyName("mesh.name")));
}

#[test]
fn empty_attribute_name_is_rejected() {
    let json = mutate_cube_json(|v| v["meshes"][0]["attributes"][0]["name"] = json!(""));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::EmptyAttributeName { mesh: 0 }));
}

#[test]
fn duplicate_object_names_are_rejected() {
    let json = mutate_cube_json(|v| {
        let objects = v["objects"].as_array_mut().unwrap();
        let mut second = objects[0].clone();
        second["index"] = json!(0);
        objects.push(second);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::DuplicateName { kind: "object.name", .. }));
}

#[test]
fn duplicate_mesh_names_are_rejected() {
    let json = mutate_cube_json(|v| {
        let meshes = v["meshes"].as_array_mut().unwrap();
        let second = meshes[0].clone();
        meshes.push(second);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::DuplicateName { kind: "mesh.name", .. }));
}

#[test]
fn duplicate_attribute_names_within_same_mesh_are_rejected() {
    let json = mutate_cube_json(|v| {
        let attributes = v["meshes"][0]["attributes"].as_array_mut().unwrap();
        let mut duplicate = attributes[1].clone(); // UVMap
        duplicate["name"] = json!("sharp_face"); // clashes with attributes[0]'s name
        attributes.push(duplicate);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::DuplicateAttributeName { mesh: 0, .. }
    ));
}

#[test]
fn same_attribute_name_across_different_meshes_is_allowed() {
    // attribute.name only needs to be unique *within* its own mesh, per spec. mesh2 is a clone
    // referencing the same BIN byte ranges as mesh1 — legal (if unusual) for meshes to share.
    let json = mutate_cube_json(|v| {
        let meshes = v["meshes"].as_array_mut().unwrap();
        let mut second = meshes[0].clone();
        second["name"] = json!("mesh2"); // avoid tripping the mesh-name-uniqueness rule instead
        meshes.push(second);
    });

    let result = read_and_validate(&json, &valid_cube_bin());

    assert!(result.is_ok(), "expected shared attribute names across meshes to be valid, got {result:?}");
}

// ---------------------------------------------------------------------------------------------
// Attribute data constraints
// ---------------------------------------------------------------------------------------------

#[test]
fn topology_positions_wrong_component_type_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["topology"]["positions"]["component_type"] = json!("U32");
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::ComponentShapeMismatch { field: "positions", .. }
    ));
}

#[test]
fn topology_edges_wrong_component_count_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["topology"]["edges"]["component_count"] = json!(3);
        // keep byte_length internally consistent so data.rs's own check doesn't fire first
        v["meshes"][0]["topology"]["edges"]["byte_length"] = json!(144);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::ComponentShapeMismatch { field: "edges", .. }
    ));
}

#[test]
fn semantic_rotation_wrong_component_count_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["attributes"][2]["data"]["component_count"] = json!(3);
        v["meshes"][0]["attributes"][2]["data"]["byte_length"] = json!(144);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::SemanticShapeMismatch { semantic: topolyx::file::Semantic::Rotation, .. }
    ));
}

#[test]
fn semantic_color_accepts_both_f32_and_u8_x4() {
    // custom_attribute is F32 already (ROTATION); repoint it at COLOR to exercise the
    // "F32 or U8" alternative shape rule, keeping its existing F32x4 storage.
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["attributes"][2]["semantic"] = json!("COLOR");
    });

    let result = read_and_validate(&json, &valid_cube_bin());

    assert!(result.is_ok(), "COLOR semantic should accept F32x4, got {result:?}");
}

// ---------------------------------------------------------------------------------------------
// element_count domain matching
// ---------------------------------------------------------------------------------------------

#[test]
fn positions_element_count_mismatch_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["topology"]["positions"]["element_count"] = json!(7);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::TopologyElementCountMismatch { field: "positions", expected: 8, found: 7, .. }
    ));
}

#[test]
fn face_offsets_element_count_must_be_faces_plus_one() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["topology"]["face_offsets"]["element_count"] = json!(6);
        v["meshes"][0]["topology"]["face_offsets"]["byte_length"] = json!(24);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::TopologyElementCountMismatch { field: "face_offsets", expected: 7, found: 6, .. }
    ));
}

#[test]
fn attribute_element_count_mismatch_is_rejected() {
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["attributes"][0]["data"]["element_count"] = json!(5); // sharp_face is FACE-domain, expects 6
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::AttributeElementCountMismatch { expected: 6, found: 5, .. }
    ));
}

// ---------------------------------------------------------------------------------------------
// Coordinate system
// ---------------------------------------------------------------------------------------------

#[test]
fn wrong_up_axis_is_rejected() {
    let json = mutate_cube_json(|v| v["coordinate_system"]["up_axis"] = json!("-Z"));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::InvalidCoordinateSystemField { field: "up_axis", .. }
    ));
}

#[test]
fn wrong_winding_is_rejected() {
    let json = mutate_cube_json(|v| v["coordinate_system"]["winding"] = json!("CW"));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::InvalidCoordinateSystemField { field: "winding", .. }
    ));
}

#[test]
fn zero_meters_per_unit_is_rejected() {
    let json = mutate_cube_json(|v| v["coordinate_system"]["meters_per_unit"] = json!(0.0));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidMetersPerUnit(_)));
}

#[test]
fn negative_meters_per_unit_is_rejected() {
    let json = mutate_cube_json(|v| v["coordinate_system"]["meters_per_unit"] = json!(-1.0));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidMetersPerUnit(_)));
}

#[test]
fn infinite_meters_per_unit_is_rejected() {
    // NaN has no JSON literal representation, so Infinity (reachable via an oversized f32
    // literal) is the only reachable way to exercise this rule from JSON text.
    let json = mutate_cube_json(|v| v["coordinate_system"]["meters_per_unit"] = json!(1e40));

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidMetersPerUnit(v) if v.is_infinite()));
}

// ---------------------------------------------------------------------------------------------
// Transform validity
// ---------------------------------------------------------------------------------------------

#[test]
fn singular_transform_is_rejected() {
    let json = mutate_cube_json(|v| {
        // Column-major 4x4; zero out column 0 to make the linear 3x3 part singular.
        v["objects"][0]["transform"] = json!([0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
    });

    let err = read_and_validate(&json, &valid_cube_bin()).unwrap_err();

    assert!(matches!(err, TopolyxError::SingularTransform { object: 0 }));
}

// ---------------------------------------------------------------------------------------------
// Corner/edge consistency, self/duplicate edges, face_offsets structure
// ---------------------------------------------------------------------------------------------

#[test]
fn self_edge_is_rejected() {
    let mut bin = valid_cube_bin();
    bin[96..104].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // edges[0] = (0, 0)

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(err, TopolyxError::SelfEdge { edge: 0, vertex: 0, .. }));
}

#[test]
fn duplicate_edge_is_rejected() {
    let mut bin = valid_cube_bin();
    // edges[1] (index 1) currently (1,2); overwrite with edges[0]'s pair (0,1) to duplicate it.
    bin[104..112].copy_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0]);

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(
        err,
        TopolyxError::DuplicateEdge { first: 0, second: 1, .. }
    ));
}

#[test]
fn corner_edge_mismatch_is_rejected() {
    let mut bin = valid_cube_bin();
    // corner_edges[0] currently references edge 0 (connects corner_vertices[0..1] = 0,1);
    // repoint it at edge 5 ((5,6)), which doesn't connect vertices 0 and 1.
    bin[288..292].copy_from_slice(&5u32.to_le_bytes());

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(err, TopolyxError::CornerEdgeMismatch { corner: 0, .. }));
}

#[test]
fn non_monotonic_face_offsets_is_rejected() {
    let mut bin = valid_cube_bin();
    // face_offsets[1] was 4; bumping it above face_offsets[2] (8, unchanged) breaks the
    // required non-decreasing order.
    bin[388..392].copy_from_slice(&10u32.to_le_bytes());

    let err = read_and_validate(DEFAULT_CUBE_JSON, &bin).unwrap_err();

    assert!(matches!(err, TopolyxError::InvalidFaceOffsets { mesh: 0 }));
}

#[test]
fn loose_vertex_is_allowed() {
    // Give positions its own fresh region at the tail of the BIN buffer (rather than growing
    // it in place) so no other descriptor's byte_offset needs to change.
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["element_counts"]["vertices"] = json!(9);
        v["meshes"][0]["topology"]["positions"]["element_count"] = json!(9);
        v["meshes"][0]["topology"]["positions"]["byte_length"] = json!(108);
        v["meshes"][0]["topology"]["positions"]["byte_offset"] = json!(804);
    });

    let mut bin = valid_cube_bin();
    bin.extend_from_slice(&[0u8; 108]); // 9 positions; the 9th is referenced by nothing

    let result = read_and_validate(&json, &bin);

    assert!(result.is_ok(), "loose (unreferenced) vertices should be allowed, got {result:?}");
}

#[test]
fn loose_edge_is_allowed() {
    // Same tail-region trick as loose_vertex_is_allowed, applied to edges: the first 12 edges
    // are identical to the fixture's (so corner_edges' existing references stay valid), plus a
    // 13th edge (0,2) that no corner_edges entry points at. custom_attribute is EDGE-domain and
    // would need its own extra element to keep matching the new edge count, so it's dropped
    // here to keep the test isolated to the edges/loose-edge rule.
    let json = mutate_cube_json(|v| {
        v["meshes"][0]["element_counts"]["edges"] = json!(13);
        v["meshes"][0]["topology"]["edges"]["element_count"] = json!(13);
        v["meshes"][0]["topology"]["edges"]["byte_length"] = json!(104);
        v["meshes"][0]["topology"]["edges"]["byte_offset"] = json!(804);
        v["meshes"][0]["attributes"].as_array_mut().unwrap().truncate(2);
    });

    let mut bin = valid_cube_bin();
    let pairs: [(u32, u32); 13] = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
        (0, 2),
    ];
    for (a, b) in pairs {
        bin.extend_from_slice(&a.to_le_bytes());
        bin.extend_from_slice(&b.to_le_bytes());
    }

    let result = read_and_validate(&json, &bin);

    assert!(result.is_ok(), "loose (unreferenced) edges should be allowed, got {result:?}");
}

// ---------------------------------------------------------------------------------------------
// Empty mesh
// ---------------------------------------------------------------------------------------------

#[test]
fn empty_mesh_with_face_offsets_zero_is_valid() {
    let result = read_and_validate(EMPTY_MESH_JSON, &[0u8; 4]);

    assert!(result.is_ok(), "an all-zero empty mesh with face_offsets == [0] should be valid, got {result:?}");
}

#[test]
fn empty_mesh_with_nonzero_face_offsets_value_is_rejected() {
    let err = read_and_validate(EMPTY_MESH_JSON, &5u32.to_le_bytes()).unwrap_err();

    assert!(matches!(err, TopolyxError::EmptyMeshFaceOffsets { mesh: 0 }));
}
