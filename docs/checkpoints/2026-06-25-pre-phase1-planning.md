# AgoraMesh Checkpoint — 2026-06-25

> **Note**
> This document records the pre-Phase 1 planning checkpoint.
> The Phase 1 implementation completion checkpoint is:
> `docs/checkpoints/2026-06-25-phase1-completion.md`

## 현재 상태

Phase 0 문서화 및 Phase 1 코딩 전 ADR/스펙 작성이 완료되었다.  
로드맵은 V1.0 최종안으로 동결되었으며, Phase 1 구현에 필요한 핵심 ADR/스펙/법률 예비 검토가 모두 마련되었다.

## 완료된 산출물

### 1. V1.0 로드맵 (최종안)

- 파일: `docs/v1.0-roadmap.md`
- 상태: 최종안 (Final Draft)
- 범위:
  - MVP-alpha = Phase 0~2 (불변 원칙 동결 + 최소 P2P 텍스트 + 최소 UI)
  - MVP-beta = Phase 3~4 (헌장/관리자/신고 + media_ref/최소 media-node)
  - v1.0 = Phase 5~10 (관리자 대시보드 + safe_mode + 번들 + 백업/이주 + 노드 패키징 + 보안 감사 + 릴리스)
- 핵심 반영:
  - 중앙 서버/릴레이/검색/게이트웨이/카테고리 목록/미디어 노드/추천 없음
  - 공통 envelope: `signing_payload` 서명, `object_id` = canonical(signing_payload) hash
  - category_id: creator_pubkey + display_name + created_at + initial_charter_hash 기반, 불변
  - moderation_evidence_ref: active moderator key 개별 암호화, audit_log에는 hash/encrypted reference만
  - 카테고리 상태: active/safe_mode/frozen/recovery/deprecated, 공식 상태 변경은 category_state 객체 + 서명/투표
  - report_bundle: local admin UI view, 원본 객체는 report, moderation_action은 report hash 목록 참조
  - text-node 스팸 방어: node_policy/local 기반, 전역 보장 아님
  - 초기 피어 연결: 수동 peer address, 로컬 discovery, 번들 포함 노드 후보, 기본 공용 노드 없음

### 2. ADR

| 파일 | 주제 | 상태 |
|---|---|---|
| `docs/adr/0006-envelope-signatures.md` | 공통 envelope 단일서명/다중서명 구조 | Accepted |
| `docs/adr/0007-text-node-spam-defense.md` | text-node 스팸 방어 범위(node_policy/local) | Accepted |

### 3. 스펙

| 파일 | 주제 |
|---|---|
| `docs/specs/category-id.md` | category_id 생성 규칙(initial_charter_hash 기준, 불변) |
| `docs/specs/report-bundle.md` | report_bundle의 local view 성격 및 deterministic grouping |

### 4. 법률 예비 검토

| 파일 | 주제 |
|---|---|
| `docs/legal-preliminary-phase-0-3.md` | Phase 0~3 법률 문구 예비 검토, maintainer position, 테스트 규칙, disclaimer |

### 5. 이전 단계 문서

- `docs/v1.0-roadmap-revised.md`: 수정안 (최종안으로 대체됨)
- `docs/agoramesh-design-notes.md`: 초기 설계 정리
- `docs/idea.txt`: 아이디어 초안
- `docs/agoramesh-a-plus-additions.md`, `docs/A플러스플러스플러스_추가_보강안_정리.md`: 추가 보강안

## 다음 단계 후보

Phase 1 코딩을 시작할 수 있다. 추천 순서:

1. **저장소 초기화**: README.md, LICENSE, .gitignore, 기본 프로젝트 구조
2. **공통 envelope 구현**: signing_payload, signature, object_id, verification 헬퍼
3. **키 관리**: Ed25519 키페어 생성/백업/복구, revocation_certificate
4. **최소 CLI**: post/comment/category 객체 생성, 서명, 검증, 로컬 저장
5. **최소 P2P**: 수동 peer 연결, gossip/direct peer, 객체 전파, 중복 제거
6. **MVP-alpha 완료 조건 충족**: 3개 이상 피어 동기화, 재시작 복원, 미래 created_at 처리, 네트워크 단절 후 재동기화

## 알려진 미해결 의사결정 (Phase 1~2 중 확정 예정)

- 객체 직렬화: Canonical JSON vs CBOR (ADR 0001)
- 네트워크 전송: libp2p vs WebSocket direct peer (ADR 0003)
- 로컬 저장소: SQLite vs LevelDB (ADR 0004)
- 클라이언트 스택: TUI 언어/프레임워크 (ADR 0005)
- 투표권 계산식 (Phase 3 전 확정)
- media_ref 세부 필드 (Phase 4 전 확정)
- discovery 방식 (V1.1 전 확정)

## 주의사항

- Phase 0에서 동결된 것: 불변 원칙, 위협 모델, 책임 경계, 문서 구조, 메인테이너 운영 불가 항목
- Provisional(이후 동결): 객체 스키마, gossip, 저장소, 투표권 계산식, media_ref 세부 필드, discovery
- Phase 0~3 동안 공식 인프라 운영 금지, public test node는 임시/제3자로 명확히 분리
