# Tasks: Vertex AI and Claude 4 Sonnet Priority Implementation

**Input**: Design documents from `/specs/001-implement-the-specifications/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/openai-api.yaml, quickstart.md

## Execution Flow (main)
```
1. Load plan.md from feature directory ✓
   → Extracted: Rust 1.75+, Axum 0.8.6, Tokio, Vertex AI, Supabase
2. Load design documents ✓:
   → data-model.md: 9 core entities + enums
   → contracts/openai-api.yaml: 3 endpoints (/health, /v1/chat/completions, /v1/models)
   → research.md: Technical decisions for Vertex AI integration
   → quickstart.md: 9 validation test scenarios
3. Generate tasks by category ✓:
   → Setup: project structure, dependencies, configuration
   → Tests: contract tests, integration tests (TDD approach)
   → Core: data models, provider clients, format conversion
   → Integration: API handlers, middleware, authentication
   → Polish: unit tests, performance validation, documentation
4. Apply task rules ✓:
   → Different files = marked [P] for parallel execution
   → Tests before implementation (TDD mandatory)
   → Dependencies properly ordered
5. Tasks numbered T001-T032 with clear file paths ✓
6. Parallel execution examples provided ✓
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- All paths are relative to repository root
- Each task specifies exact file location

## Phase 3.1: Setup & Foundation
- [x] **T001** Initialize project structure with missing directories (`tests/contract/`, `tests/integration/`)
- [x] **T002** Update Cargo.toml dependencies for missing crates (tower-http CORS, prometheus metrics)
- [x] **T003** [P] Configure development tooling (rustfmt.toml, clippy.toml, .gitignore updates)

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (OpenAPI Specification)
- [x] **T004** [P] Contract test GET /health endpoint in `tests/contract/test_health.rs`
- [x] **T005** [P] Contract test POST /v1/chat/completions (non-streaming) in `tests/contract/test_chat_completions.rs`
- [x] **T006** [P] Contract test POST /v1/chat/completions (streaming) in `tests/contract/test_chat_streaming.rs`
- [x] **T007** [P] Contract test GET /v1/models endpoint in `tests/contract/test_models.rs`

### Integration Tests (Quickstart Scenarios)
- [x] **T008** [P] Integration test: Health check validation in `tests/integration/test_health_validation.rs`
- [x] **T009** [P] Integration test: Authentication flow (valid JWT) in `tests/integration/test_auth_flow.rs`
- [x] **T010** [P] Integration test: Chat completion E2E in `tests/integration/test_chat_e2e.rs`
- [x] **T011** [P] Integration test: Streaming chat completion in `tests/integration/test_streaming_e2e.rs`
- [x] **T012** [P] Integration test: Error handling scenarios in `tests/integration/test_error_handling.rs`
- [x] **T013** [P] Integration test: Performance validation (concurrent requests) in `tests/integration/test_performance.rs`

## Phase 3.3: Core Data Models (ONLY after tests are failing)
### Shared Types & Errors
- [x] **T014** [P] Core error types and HTTP mapping in `src/shared/error.rs` (extend existing)
- [x] **T015** [P] OpenAI API types (ChatRequest, Message, etc.) in `src/models/` (comprehensive implementation)
- [x] **T016** [P] Validation utilities and parameter checking in `src/shared/utils.rs` (extend existing)

### Authentication Models
- [x] **T017** [P] Authentication context and JWT claims in `src/features/auth/types.rs`
- [x] **T018** [P] Supabase integration types in `src/features/auth/supabase.rs` (extend existing)

### Provider Models
- [x] **T019** [P] Vertex AI request/response types in `src/infrastructure/vertex/types.rs`
- [x] **T020** [P] Provider configuration types in `src/config/providers.rs`

## Phase 3.4: Provider Integration
### Vertex AI Client
- [x] **T021** [P] Vertex AI authentication client in `src/infrastructure/vertex/auth.rs`
- [x] **T022** [P] Vertex AI HTTP client implementation in `src/infrastructure/vertex/client.rs` (extend existing)
- [x] **T023** [P] OpenAI ↔ Vertex AI format conversion in `src/infrastructure/vertex/converter.rs` (extend existing)
- [x] **T024** [P] Vertex AI streaming response handler in `src/infrastructure/vertex/streaming.rs`

### Authentication Middleware
- [x] **T025** JWT validation and user context extraction in `src/features/auth/middleware.rs` (extend existing)

## Phase 3.5: API Layer Implementation
### Route Handlers (Sequential - same app.rs file)
- [x] **T026** Health endpoint handler implementation in `src/api/handlers/health.rs`
- [x] **T027** Models endpoint handler in `src/api/handlers/models.rs`
- [x] **T028** Chat completions handler (non-streaming) in `src/api/handlers/chat.rs`
- [ ] **T029** Chat completions handler (streaming) in `src/api/handlers/chat.rs` (extend T028)

### Application Integration
- [ ] **T030** Route registration and middleware setup in `src/app.rs` (extend existing)
- [ ] **T031** Application state and configuration management in `src/config/app.rs` (extend existing)

## Phase 3.6: Polish & Validation
- [ ] **T032** [P] Run quickstart.md validation suite and fix any failures

## Dependencies
```
Setup (T001-T003) → Tests (T004-T013) → Core Models (T014-T020) → Provider Integration (T021-T025) → API Layer (T026-T031) → Validation (T032)

Critical Blocking Dependencies:
- All tests (T004-T013) MUST complete before any implementation
- T015 (OpenAI types) blocks T019 (Vertex AI types)
- T021 (Vertex AI auth) blocks T022-T024 (Vertex AI client components)
- T025 (auth middleware) blocks T026-T029 (handlers requiring auth)
- T026-T029 (handlers) must complete before T030 (route registration)
- T020, T025, T026-T029 must complete before T031 (app state)
```

## Parallel Execution Examples

### Phase 3.2 - All Contract Tests (can run simultaneously):
```bash
# Launch T004-T007 together:
Task: "Contract test GET /health endpoint in tests/contract/test_health.rs"
Task: "Contract test POST /v1/chat/completions (non-streaming) in tests/contract/test_chat_completions.rs"
Task: "Contract test POST /v1/chat/completions (streaming) in tests/contract/test_chat_streaming.rs"
Task: "Contract test GET /v1/models endpoint in tests/contract/test_models.rs"
```

### Phase 3.2 - All Integration Tests (can run simultaneously):
```bash
# Launch T008-T013 together:
Task: "Integration test: Health check validation in tests/integration/test_health_validation.rs"
Task: "Integration test: Authentication flow (valid JWT) in tests/integration/test_auth_flow.rs"
Task: "Integration test: Chat completion E2E in tests/integration/test_chat_e2e.rs"
Task: "Integration test: Streaming chat completion in tests/integration/test_streaming_e2e.rs"
Task: "Integration test: Error handling scenarios in tests/integration/test_error_handling.rs"
Task: "Integration test: Performance validation (concurrent requests) in tests/integration/test_performance.rs"
```

### Phase 3.3 - Core Models (can run simultaneously):
```bash
# Launch T014-T020 together (different files):
Task: "Core error types and HTTP mapping in src/shared/error.rs"
Task: "OpenAI API types (ChatRequest, Message, etc.) in src/shared/types.rs"
Task: "Validation utilities and parameter checking in src/shared/utils.rs"
Task: "Authentication context and JWT claims in src/features/auth/types.rs"
Task: "Supabase integration types in src/features/auth/supabase.rs"
Task: "Vertex AI request/response types in src/infrastructure/vertex/types.rs"
Task: "Provider configuration types in src/config/providers.rs"
```

### Phase 3.4 - Provider Components (can run simultaneously after T015, T019):
```bash
# Launch T021-T024 together:
Task: "Vertex AI authentication client in src/infrastructure/vertex/auth.rs"
Task: "Vertex AI HTTP client implementation in src/infrastructure/vertex/client.rs"
Task: "OpenAI ↔ Vertex AI format conversion in src/infrastructure/vertex/converter.rs"
Task: "Vertex AI streaming response handler in src/infrastructure/vertex/streaming.rs"
```

## File-by-File Task Mapping
**Files with multiple tasks (MUST be sequential)**:
- `src/shared/types.rs`: T015
- `src/shared/error.rs`: T014
- `src/shared/utils.rs`: T016
- `src/features/auth/supabase.rs`: T018
- `src/infrastructure/vertex/client.rs`: T022
- `src/infrastructure/vertex/converter.rs`: T023
- `src/features/auth/middleware.rs`: T025
- `src/api/handlers/chat.rs`: T028 → T029 (streaming extends non-streaming)
- `src/app.rs`: T030
- `src/config/app.rs`: T031

**Files with single tasks (can be [P])**:
- All test files (T004-T013)
- All new model files (T017, T019, T020, T021, T024, T026, T027)

## Critical Success Criteria
1. **TDD Compliance**: All tests (T004-T013) must fail before implementation begins
2. **API Compatibility**: Contract tests must validate exact OpenAI API compliance
3. **Performance**: Integration tests must verify 95% of requests complete under 5 seconds
4. **Concurrency**: Support 1000 concurrent requests without degradation
5. **Streaming**: Server-Sent Events must follow OpenAI's exact format
6. **Authentication**: Supabase JWT validation must work for all protected endpoints
7. **Provider Integration**: Vertex AI claude-4-sonnet-20250514 model must be accessible
8. **Error Handling**: All error scenarios must return OpenAI-compatible responses

## Validation Checklist
*GATE: Checked before task execution begins*

- [x] All contracts have corresponding tests (T004-T007 cover all 3 endpoints)
- [x] All entities have model tasks (T014-T020 cover 9 core entities)
- [x] All tests come before implementation (T004-T013 before T014+)
- [x] Parallel tasks are truly independent (different files verified)
- [x] Each task specifies exact file path (all tasks include file locations)
- [x] No task modifies same file as another [P] task (dependencies mapped)
- [x] Critical path identified (tests → models → providers → handlers → integration)

## Notes
- **[P] tasks**: Different files, no dependencies, can run concurrently
- **Sequential tasks**: Same file modifications, must run in order
- **TDD Required**: Tests MUST fail before writing implementation code
- **Cargo commands**: Use `cargo test`, `cargo build`, `cargo fmt`, `cargo clippy`
- **Environment**: Copy `.env.example` to `.env` and configure before testing
- **GCP Setup**: Requires service account key at `./gcp-credentials.json`
- **Supabase**: Requires valid JWT secret for authentication testing

---
**Generated from**: plan.md, data-model.md, contracts/openai-api.yaml, research.md, quickstart.md
**Ready for execution**: All 32 tasks defined with clear dependencies and parallel execution paths