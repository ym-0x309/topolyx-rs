pub const DEFAULT_CUBE_JSON: &str = r#"{
    "header": { "format": "Topolyx", "version": "1.0" },
    "coordinate_system": {
        "up_axis": "+Z", "forward_axis": "+Y",
        "handedness": "RIGHT", "winding": "CCW",
        "meters_per_unit": 1.0
    },
    "objects": [
        { "name": "object1", "type": "MESH", "index": 0,
          "transform": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1] }
    ],
    "meshes": [
        { "name": "mesh1",
          "element_counts": {"vertices":8,"edges":12,"faces":6,"corners":24},
          "topology": {
            "positions": {"byte_offset":0,"byte_length":96,"component_type":"F32","component_count":3,"element_count":8},
            "edges": {"byte_offset":96,"byte_length":96,"component_type":"U32","component_count":2,"element_count":12},
            "corner_vertices": {"byte_offset":192,"byte_length":96,"component_type":"U32","component_count":1,"element_count":24},
            "corner_edges": {"byte_offset":288,"byte_length":96,"component_type":"U32","component_count":1,"element_count":24},
            "face_offsets": {"byte_offset":384,"byte_length":28,"component_type":"U32","component_count":1,"element_count":7}
          },
          "attributes": [
            {"name":"sharp_face","domain":"FACE","semantic":"NONE","data":{"byte_offset":412,"byte_length":6,"component_type":"BOOL","component_count":1,"element_count":6}},
            {"name":"UVMap","domain":"CORNER","semantic":"NONE","data":{"byte_offset":420,"byte_length":192,"component_type":"F32","component_count":2,"element_count":24}},
            {"name":"custom_attribute","domain":"EDGE","semantic":"ROTATION","data":{"byte_offset":612,"byte_length":192,"component_type":"F32","component_count":4,"element_count":12}}
          ]
        }
    ]
}"#;

/// `element_counts`가 모두 0인 "빈 메시" 픽스처. `DEFAULT_CUBE_JSON`은 항상 비어있지 않으므로
/// 빈 메시 규칙 테스트에는 별도 픽스처가 필요하다.
#[allow(unused)]
pub const EMPTY_MESH_JSON: &str = r#"{
    "header": { "format": "Topolyx", "version": "1.0" },
    "coordinate_system": {
        "up_axis": "+Z", "forward_axis": "+Y",
        "handedness": "RIGHT", "winding": "CCW",
        "meters_per_unit": 1.0
    },
    "objects": [],
    "meshes": [
        { "name": "empty_mesh",
          "element_counts": {"vertices":0,"edges":0,"faces":0,"corners":0},
          "topology": {
            "positions": {"byte_offset":0,"byte_length":0,"component_type":"F32","component_count":3,"element_count":0},
            "edges": {"byte_offset":0,"byte_length":0,"component_type":"U32","component_count":2,"element_count":0},
            "corner_vertices": {"byte_offset":0,"byte_length":0,"component_type":"U32","component_count":1,"element_count":0},
            "corner_edges": {"byte_offset":0,"byte_length":0,"component_type":"U32","component_count":1,"element_count":0},
            "face_offsets": {"byte_offset":0,"byte_length":4,"component_type":"U32","component_count":1,"element_count":1}
          },
          "attributes": []
        }
    ]
}"#;

/// `DEFAULT_CUBE_JSON`을 파싱해 `mutate`로 정확히 한 필드만 깨뜨린 뒤 재직렬화한다.
/// 유효성 검증 규칙 하나를 어기는 픽스처를 손으로 다시 쓰지 않아도 되게 해준다.
#[allow(unused)]
pub fn mutate_cube_json(mutate: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value = serde_json::from_str(DEFAULT_CUBE_JSON).unwrap();
    mutate(&mut value);
    value.to_string()
}

/// `DEFAULT_CUBE_JSON`의 바이트 레이아웃과 정확히 일치하는, 위상적으로 실제 유효한 큐브의
/// BIN 버퍼(804바이트)를 만든다. `tests/v0_1_0.rs`/`tests/v0_2_0.rs`가 쓰는 zero-filled 버퍼는
/// 디코딩 자체만 테스트하도록 의도된 것이라 위상은 무효하다 — 여기서는 그와 별개로 실제
/// 코너/엣지 일관성이 성립하는 정육면체 데이터를 만든다.
#[allow(unused)]
pub fn valid_cube_bin() -> Vec<u8> {
    let mut bin = vec![0u8; 804];

    let positions: [[f32; 3]; 8] = [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    write_f32_array(&mut bin, 0, positions.iter().flatten().copied());

    // 정육면체의 12개 변, 정점 인덱스 쌍
    let edges: [[u32; 2]; 12] = [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [4, 5],
        [5, 6],
        [6, 7],
        [7, 4],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ];
    write_u32_array(&mut bin, 96, edges.iter().flatten().copied());

    // 6개 면(각 4개 코너), 코너가 참조하는 정점
    let corner_vertices: [[u32; 4]; 6] = [
        [0, 1, 2, 3],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [0, 3, 7, 4],
        [1, 2, 6, 5],
    ];
    write_u32_array(&mut bin, 192, corner_vertices.iter().flatten().copied());

    // 각 코너 -> 같은 면의 다음 코너로 이어지는 변의 인덱스 (edges 배열 기준)
    let corner_edges: [[u32; 4]; 6] = [
        [0, 1, 2, 3],
        [4, 5, 6, 7],
        [0, 9, 4, 8],
        [2, 11, 6, 10],
        [3, 11, 7, 8],
        [1, 10, 5, 9],
    ];
    write_u32_array(&mut bin, 288, corner_edges.iter().flatten().copied());

    let face_offsets: [u32; 7] = [0, 4, 8, 12, 16, 20, 24];
    write_u32_array(&mut bin, 384, face_offsets.iter().copied());

    bin
}

#[allow(unused)]
fn write_f32_array(bin: &mut [u8], offset: usize, values: impl Iterator<Item = f32>) {
    let mut cursor = offset;
    for v in values {
        bin[cursor..cursor + 4].copy_from_slice(&v.to_le_bytes());
        cursor += 4;
    }
}

#[allow(unused)]
fn write_u32_array(bin: &mut [u8], offset: usize, values: impl Iterator<Item = u32>) {
    let mut cursor = offset;
    for v in values {
        bin[cursor..cursor + 4].copy_from_slice(&v.to_le_bytes());
        cursor += 4;
    }
}

/// 테스트 전용 헬퍼 — 크레이트의 공개 API가 아님. write_tlyx_container를 흉내내되
/// 검증(validity) 없이 단순히 magic/chunk 프레이밍만 조립한다.
pub fn build_container(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json_bytes = json.as_bytes().to_vec();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' '); // 0x20 패딩
    }
    let mut bin_bytes = bin.to_vec();
    while !bin_bytes.len().is_multiple_of(4) {
        bin_bytes.push(0); // 0x00 패딩
    }

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    let mut out = Vec::with_capacity(total_length);
    out.extend_from_slice(b"TLYX");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(total_length as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin_bytes);
    out
}
