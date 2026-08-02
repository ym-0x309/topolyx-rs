# topolyx-rs 버전별 변경 기록 및 향후 계획

## 향후 계획

### v0.4.0

- 엔진에서 사용할 편의 api

### v0.5.0

- 적대적 입력 테스트

### v0.6.0

- 문서화
- api 동결

### v1.0.0

- topolyx 명세 1.0 완벽 대응

### v1.0.0 이후

- writer 기능 추가

---

## 변경 기록

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