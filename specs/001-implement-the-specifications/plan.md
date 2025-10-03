
# Implementation Plan: Vertex AI and Claude 4 Sonnet Priority Implementation

**Branch**: `001-implement-the-specifications` | **Date**: October 2, 2025 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-implement-the-specifications/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from file system structure or context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code or `AGENTS.md` for opencode).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Implement OpenAI-compatible API service that transparently routes chat completion requests to Google Vertex AI's Claude 4 Sonnet model (claude-4-sonnet-20250514). Core focus: chat completions with Supabase JWT authentication, Server-Sent Events streaming, and provider fallback capabilities. Performance target: 95% of requests under 5 seconds with support for 1000 concurrent connections.

## Technical Context
**Language/Version**: Rust 1.75+ (Cargo.toml edition 2021)
**Primary Dependencies**: Axum 0.8.6, Tokio async runtime, Tower middleware, Reqwest HTTP client, GCP Auth, Supabase integration
**Storage**: Supabase (PostgreSQL) for authentication, request logging, and webhook configuration
**Testing**: cargo test (standard Rust testing framework)
**Target Platform**: Linux server (containerized deployment)
**Project Type**: single (web service API server)
**Performance Goals**: 95% requests <5 seconds, 1000 concurrent requests
**Constraints**: OpenAI API compatibility required, Vertex AI as primary provider, JWT authentication mandatory
**Scale/Scope**: Enterprise-grade AI proxy service, multi-provider support, comprehensive observability

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: PASS (constitution template not yet customized - proceeding with standard best practices)
- Clean architecture with separation of concerns ✓
- Test-driven development approach planned ✓
- Library-first approach for reusable components ✓
- Clear API contracts and documentation ✓
- Observability and monitoring built-in ✓

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
src/
├── main.rs                  # Application entry point
├── lib.rs                   # Library root
├── app.rs                   # Axum application setup
├── config/                  # Configuration management
│   ├── mod.rs
│   └── app.rs
├── infrastructure/          # External service integrations
│   ├── mod.rs
│   ├── vertex/              # Vertex AI client implementation
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── converter.rs     # OpenAI ↔ Vertex format conversion
│   ├── supabase.rs          # Supabase integration
│   └── logging.rs           # Structured logging
├── features/                # Feature-based business logic
│   ├── mod.rs
│   └── auth/                # Authentication feature
│       ├── mod.rs
│       └── middleware.rs
├── shared/                  # Common utilities
│   ├── mod.rs
│   ├── error.rs             # Error types and handling
│   ├── types.rs             # Shared type definitions
│   └── utils.rs
└── api/                     # HTTP API layer
    ├── mod.rs
    ├── routes.rs            # Route definitions
    └── handlers/            # Request handlers
        ├── mod.rs
        ├── chat.rs          # /v1/chat/completions
        ├── models.rs        # /v1/models
        └── health.rs        # Health checks

tests/
├── integration/             # End-to-end API tests
├── contract/                # API contract validation tests
└── unit/                    # Unit tests (embedded in modules)

Cargo.toml                   # Project dependencies
```

**Structure Decision**: Single Rust project with feature-based clean architecture. The existing `src/` structure aligns with the planned implementation, focusing on separation between infrastructure (external services), features (business logic), and API layers.

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude`
     **IMPORTANT**: Execute it exactly as specified above. Do not add or remove any arguments.
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- Each OpenAPI endpoint → contract test task [P]
- Each data model entity → Rust struct/enum creation task [P]
- Each acceptance scenario from spec → integration test task
- Implementation tasks to make tests pass (handlers, services, clients)

**Ordering Strategy**:
- TDD order: Contract tests → Unit tests → Implementation → Integration tests
- Dependency order:
  1. Core types and errors (shared module)
  2. Configuration management
  3. Provider client implementations (Vertex AI)
  4. Authentication middleware
  5. API handlers (chat completions, models, health)
  6. Integration and performance tests
- Mark [P] for parallel execution (independent files/modules)

**Specific Task Categories**:
1. **Foundation** (1-5): Error types, config, logging setup
2. **Provider Integration** (6-12): Vertex AI client, format conversion, authentication
3. **API Layer** (13-18): Axum routes, handlers, middleware
4. **Testing** (19-25): Contract tests, integration tests, performance validation
5. **Deployment** (26-30): Docker, health checks, monitoring

**Estimated Output**: 28-32 numbered, ordered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning approach documented (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved
- [x] No complexity deviations requiring documentation

**Artifacts Generated**:
- [x] research.md - Technical research and decisions
- [x] data-model.md - Entity definitions and relationships
- [x] contracts/openai-api.yaml - OpenAPI specification
- [x] quickstart.md - Validation test suite
- [x] CLAUDE.md - Updated agent context (Claude Code)

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
