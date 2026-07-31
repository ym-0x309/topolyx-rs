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
