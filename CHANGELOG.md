# topolyx-rs 버전별 변경 기록 및 향후 계획

## 향후 계획

### v0.6.0

- 적대적 입력 테스트

### v0.7.0

- 문서화
- api 동결

### v1.0.0

- topolyx 명세 1.0 완벽 대응

### v1.0.0 이후

- writer 기능 추가

---

## 변경 기록

### v0.5.0

- `Mesh::triangulate_fan_corner_indices` 추가 (`src/faces.rs`) — `triangulate_fan_indices`(정점 인덱스, `0..vertices`)와 달리 코너 인덱스(`0..corners`) 기준으로 fan triangulate. `corner_positions`/`world_corner_positions`나 CORNER 도메인 attribute(노멀/UV 등)와 짝지어 인터리브 정점 버퍼를 그릴 때 사용 — `triangulate_fan_indices`의 정점 인덱스는 CORNER 도메인 데이터에는 대응이 안 됨(같은 정점이 face마다 다른 코너 값을 가질 수 있으므로)
- `Mesh::triangulate_fan_face_indices` 추가 (`src/faces.rs`) — `triangulate_fan_indices`/`triangulate_fan_corner_indices`가 만든 삼각형 하나당 원래 face 인덱스를 병렬 배열로 반환, FACE 도메인 attribute(재질 인덱스, `sharp_face` 등)를 삼각화된 출력에서도 조회할 수 있게 함. 블렌더의 `bpy.types.MeshLoopTriangle.polygon_index`와 동일한 패턴
- EDGE 도메인 attribute는 삼각화된 출력에서 조회할 방법이 근본적으로 없다는 점을 rustdoc에 명시 — fan triangulation이 만드는 대각선은 `topology.edges`에 대응 항목이 없어 값 자체가 존재하지 않음(블렌더의 `MeshLoopTriangle`에도 동일한 이유로 엣지 프로브넌스가 없음)
- `TopolyxError`에 `#[non_exhaustive]` 추가 — 이번 릴리스는 `CornerIndexOutOfRange`류 새 variant 자체가 이미 breaking이지만, 이후로는 새 에러 variant 추가가 breaking change가 되지 않도록 함
- 컨테이너 검증 버그 수정: `parse_container`가 BIN 청크 뒤에 남는 바이트를 검사하지 않던 문제 (`src/reader.rs`) — `header.total_length`만 실제 파일 길이와 맞추면 BIN 청크 선언 길이 뒤에 임의의 바이트를 덧붙인 파일도 통과했음(스펙 5장 "파일은 정확히 이 두 청크만 포함"). `TopolyxError::TrailingBytes` 추가
- 실행 테스트 추가 (`tests/v0_5_0.rs`, `tests/v0_3_0.rs`)

### v0.4.0

- 엔진에서 사용할 편의 api
  - semantic 기반 그루핑 (`src/grouped.rs`)
    - `ComponentData::grouped_f32`/`grouped_i32`/`grouped_u32`/`grouped_i8`/`grouped_u8`/`grouped_bool` — 평탄한 배열을 `Vec<[T; N]>`로 그루핑
    - `Topology::positions`/`edges`/`corner_vertices`/`corner_edges`/`face_offsets` — 명세 4장 shape 그대로 타입 있는 값 반환
    - `AttributeValues`, `Attribute::values` — `semantic`에 따라 그루핑된 값을 반환 (`NONE`은 그루핑하지 않고 `ComponentData` 그대로 반환)
    - `Mesh::attribute`/`attributes_by_semantic` — 이름/semantic으로 속성 조회
  - face 순회 및 간이 삼각화 (`src/faces.rs`)
    - `Mesh::faces`, `FaceCorners`, `Face` — `face_offsets` 기준 face별 코너 슬라이스 순회
    - `Face::triangulate_fan` — fan triangulation (convex/planar face에 대해서만 정확함; non-planar/오목 n-gon 처리는 명세 범위 밖)
  - 월드 스페이스 transform 적용 (`src/transform.rs`, 명세 4장 "Object Transform Application Rules")
    - `Mesh::world_positions`, `Attribute::world_values` — `object.transform`을 semantic별 규칙(`POSITION`/`DIRECTION`/`NORMAL`/`TANGENT`/`COLOR`/`NONE`)에 따라 적용
    - `ROTATION` semantic은 **미지원** — 일반적인 경우 `transform`의 선형부에서 순수 회전 성분을 뽑아내는 분해(polar decomposition 등)가 필요해 구현 범위를 넘어섬. 호출 시 `TopolyxError::UnsupportedTransformSemantic` 반환. 추후 버전에서 다룰 후보.
  - 관련 에러 종류 추가 (`UnexpectedComponentShape`, `UnsupportedTransformSemantic`, `SingularObjectTransform`)
  - 실행 테스트 추가 (`tests/v0_4_0-1.rs`)

- 렌더링용 편의 메서드 추가 (`src/faces.rs`)
  - `Mesh::triangulate_fan_indices` — 메시 전체 face를 fan triangulate해 평탄한 삼각형 인덱스 버퍼(`Vec<u32>`)로 반환, GPU 인덱스 버퍼로 바로 사용 가능
  - `Mesh::corner_positions`/`world_corner_positions` — `positions`(또는 `world_positions`)를 `corner_vertices`로 재인덱싱해 CORNER 도메인 attribute와 길이를 맞춘 코너 단위 위치 배열 반환. 인터리브된 단일 GPU 정점 버퍼 구성에 사용
  - 범위를 벗어난 재인덱싱 인덱스에 대한 에러 `TopolyxError::CornerIndexOutOfRange` 추가
- `Face::triangulate_fan`의 패닉 버그 수정 — `validate()`를 거치지 않은 파일에서 3개 미만의 코너를 가진(명세 위반) face에 대해 패닉하던 것을 빈 이터레이터 반환으로 변경
- 실행 테스트 추가 (`tests/v0_4_0-2.rs`)

### v0.3.0

- 파일 전체 유효성 검증 (topolyx 명세 5장 "Validity Conditions")
  - 컨테이너 레벨 검사(필수, `read_topolyx`/`read_topolyx_from_data`에 통합)
    - 컨테이너 버전과 JSON `header.version`의 주 버전 일치 여부
    - JSON 청크 패딩 바이트가 `0x20`인지 여부
  - `TopolyxFile::validate` 추가 (opt-in — 자동으로 실행되지 않음)
    - 인덱스 범위 (`object.index`, `edges`/`corner_vertices`/`corner_edges`의 참조 인덱스)
    - 이름 제약 (빈 이름 금지, object/mesh 이름 유일성, mesh 내 attribute 이름 유일성)
    - 속성 데이터 제약 (topology 필드 및 semantic이 있는 attribute의 타입/개수 일치)
    - element_count가 대응하는 도메인의 개수와 일치하는지 여부
    - 좌표계 고정 필드 값 및 `meters_per_unit` 유효성
    - transform 선형부(3x3)의 특이행렬 여부
    - 코너-엣지 일관성, self/중복 edge 금지, `face_offsets` 구조 유효성
    - 빈 메시 규칙
    - BIN 청크 꼬리 패딩 바이트가 `0x00`인지 여부
  - 관련 에러 종류 추가
  - `Semantic`, `Domain`, 및 `TopolyxFile`을 구성하는 구조체들에 `Debug` derive 추가
  - 실행 테스트 추가 (`tests/v0_3_0.rs`)

### v0.2.0

- 바이너리에서 데이터 추출
  - `ComponentType::byte_size`, `DataDescriptor::extract`, `ComponentData` 추가
  - 관련 에러 종류 추가 (`UnalignedByteOffset`, `ByteLengthMismatch`, `DataOutOfBounds`, `InvalidBoolValue`)
  - 실행 테스트 추가

### v0.1.0

> [!IMPORTANT]
> 첫 버전

- 에러 정의(`Io`, `InvalidMagic`, `UnsupportedContainerVersion`, `MalformedContainer`, `TotalLengthMismatch`, `Json`)
- 파일 헤더/JSON/BIN 추출 로직(`read_topolyx()`, `read_topolyx_from_data`) 정의
- 실행 테스트 정의

---