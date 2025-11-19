# Prometheus PAS-X Agent Artifact Specification (Agent Artifacts v1.0)

## Overview

This document defines the full technical specification for **Agent Artifacts**
under the Prometheus PAS-X Unified Artifact Standard. Agent Artifacts describe,
store, and operationalize autonomous agent behavior across all Prometheus
environments, including:

- **Prometheus LLM Gateway**
- **Prometheus Studio**
- **AG-UI agent endpoints**
- **OpenWebUI agent extensions**
- **Supabase storage + GraphQL APIs**
- **Local PGLite-powered runtimes**
- **Distributed, sync-ready agent execution environments**

An Agent Artifact is a **fully portable, declarative, schema-stable unit**
representing the behavior, memory, reasoning structure, tools, workflows, and
runtime definitions of an agent.

Agent Artifacts are intended to:

- Provide an **LLM-agnostic universal representation** of an agent.
- Enable **cross-runtime execution** (browser, server, microVM, mobile,
  desktop).
- Integrate directly with **AG-UI** for sandboxing and visualization of agent
  behavior.
- Allow persistent storage (Supabase, SurrealDB, IPFS, PGLite).
- Serve as authoritative definitions for the LLM Gateway to construct, run, and
  orchestrate agents.

---

## Agent Artifact Identity

Every agent artifact follows PAS-X identity rules:

```json
{
  "schema": "https://prometheusags.ai/specs/agent-artifact/1.0.0",
  "id": "pas://agent/<uuid>",
  "version": "1.0.0",
  "type": "agent.behavior",
  "subtype": "generic|task|workflow|assistant|expert",
  "origin": "supabase|pglite|ipfs|surrealdb|local|remote",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

---

## Core Structure of an Agent Artifact

```json
{
  "schema": "https://prometheusags.ai/specs/agent-artifact/1.0.0",
  "id": "pas://agent/uuid",
  "type": "agent.behavior",
  "subtype": "task|workflow|assistant|expert",
  "version": "1.0.0",
  "metadata": {},
  "agent": {
    "role": "expert-agent|assistant-agent|task-runner",
    "objectives": [],
    "prompt": "",
    "memory": {},
    "tools": [],
    "workflow": {},
    "reasoning": {},
    "events": []
  },
  "runtime": {},
  "links": {},
  "signature": {}
}
```

---

# 1. Metadata Specification

### Required

```json
{
  "title": "Research Assistant Agent",
  "description": "An agent optimized for research tasks.",
  "tags": ["research", "assistant"],
  "language": "none",
  "author": "system|user|agent",
  "permissions": {
    "read": true,
    "write": true,
    "execute": true
  }
}
```

---

# 2. Agent Role Definition

### `agent.role`

Defines the top-level behavioral identity.

Enum:

- `assistant-agent`
- `expert-agent`
- `task-runner`
- `workflow-coordinator`
- `planner`
- `analyst`
- `router`

Example:

```json
"role": "expert-agent"
```

---

# 3. Objectives

List of declarative goals.

```json
"objectives": [
  "Interpret user queries",
  "Route tasks to appropriate tool",
  "Summarize results back to user"
]
```

---

# 4. Prompt Specification

### `agent.prompt`

The agent's system or behavior prompt.

```json
"prompt": "You are a senior full-stack engineer..."
```

Supports:

- multi-part prompts
- templating (LLM Gateway JSON templating)
- persona embedding

---

# 5. Memory Specification

Memory includes short-term, long-term, and graph memory.

```json
"memory": {
  "short_term": [],
  "long_term": [],
  "graph": {
    "nodes": [],
    "edges": []
  }
}
```

### Memory Types

- `short_term` – ephemeral between calls
- `long_term` – persisted in Supabase
- `graph` – stored via SurrealDB or PGLite vector/graph extension

---

# 6. Tool Specification

Tools define external MCP, AgentKit, Prometheus Gateway, or AG-UI endpoints.

### Example

```json
"tools": [
  "search.web",
  "filesystem.read",
  "code.run",
  "prometheus.agui.workspace.update"
]
```

### Tool Descriptor (optional inline)

```json
{
  "name": "filesystem.read",
  "input_schema": {},
  "output_schema": {},
  "runtime": "node|deno|bun|rust|python|wasm"
}
```

---

# 7. Workflow Specification

Workflows define multi-step agent reasoning.

```json
"workflow": {
  "type": "directed_acyclic_graph",
  "nodes": [
    { "id": "n1", "action": "collect_input" },
    { "id": "n2", "action": "tool:search.web" },
    { "id": "n3", "action": "summarize" }
  ],
  "edges": [
    { "from": "n1", "to": "n2" },
    { "from": "n2", "to": "n3" }
  ]
}
```

---

# 8. Reasoning Specification

A fully structured reasoning framework.

```json
"reasoning": {
  "mode": "chain_of_thought|supervised|hidden|visible",
  "max_depth": 10,
  "allow_reflection": true,
  "reflection_prompts": []
}
```

---

# 9. Event Specification

Agents can subscribe to events.

```json
"events": [
  {
    "event": "onStart",
    "action": "announce_ready"
  },
  {
    "event": "onToolComplete",
    "action": "evaluate_next_step"
  }
]
```

---

# 10. Runtime Specification

Defines where the agent can be executed.

```json
"runtime": {
  "engine": "node|bun|deno|rust|python|wasm|microvm",
  "sandbox": "browser|server|microvm|tauri",
  "requirements": []
}
```

---

# 11. Storage Specification (Supabase-ready)

### Table: `agent_artifacts`

| Column     | Type        | Notes                 |
| ---------- | ----------- | --------------------- |
| id         | uuid        | primary key           |
| data       | jsonb       | full artifact payload |
| created_at | timestamptz |                       |
| updated_at | timestamptz |                       |
| owner      | uuid        | user id               |

Indexes:

- GIN index on JSONB for querying

---

# 12. Execution Specification (LLM Gateway + AG-UI)

### LLM Gateway Endpoint Format

```json
POST /api/agent/execute
{
  "artifact_id": "pas://agent/uuid",
  "input": "User query here",
  "session": "session-id"
}
```

### AG-UI Interaction Endpoints

- `agui.workspace.update`
- `agui.canvas.render`
- `agui.graph.update`
- `agui.session.state`
- `agui.agent.inspect`

---

# 13. Linking & Graph

Agents may reference:

- other agents
- tools
- documents
- UI artifacts

```json
"links": {
  "dependencies": ["pas://artifact/ui-123"],
  "dependents": []
}
```

---

# 14. Signature

Cryptographic validation.

```json
"signature": {
  "hash": "sha256-...",
  "algorithm": "sha256"
}
```

---

## TypeScript Type Definitions

### Agent Artifact Interface

```typescript
interface AgentArtifact {
  schema: string;
  id: string;
  type: "agent.behavior";
  subtype: "generic" | "task" | "workflow" | "assistant" | "expert";
  version: string;
  metadata: AgentMetadata;
  agent: AgentDefinition;
  runtime: RuntimeDefinition;
  links?: AgentLinks;
  signature?: Signature;

  // Optional fields added for enhanced functionality
  evaluation?: EvaluationFramework;
  planning?: PlanningCapabilities;
}

interface AgentMetadata {
  title: string;
  description: string;
  tags: string[];
  language: string;
  author: "system" | "user" | "agent";
  permissions: {
    read: boolean;
    write: boolean;
    execute: boolean;
  };
}

interface AgentDefinition {
  role: AgentRole;
  objectives: string[];
  prompt: string;
  memory: MemorySpecification;
  tools: ToolDefinition[];
  workflow: WorkflowDefinition;
  reasoning: ReasoningFramework;
  events: EventDefinition[];
}

type AgentRole =
  | "assistant-agent"
  | "expert-agent"
  | "task-runner"
  | "workflow-coordinator"
  | "planner"
  | "analyst"
  | "router";

interface MemorySpecification {
  short_term: any[];
  long_term: any[];
  graph: {
    nodes: GraphNode[];
    edges: GraphEdge[];
  };
}

interface GraphNode {
  id: string;
  type: string;
  data: any;
  metadata?: Record<string, any>;
}

interface GraphEdge {
  from: string;
  to: string;
  type: string;
  weight?: number;
}

interface ToolDefinition {
  name: string;
  input_schema?: JSONSchema;
  output_schema?: JSONSchema;
  runtime?: string;
  description?: string;
}

interface WorkflowDefinition {
  type: "directed_acyclic_graph" | "state_machine";
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  start_node?: string;
  end_nodes?: string[];
}

interface WorkflowNode {
  id: string;
  action: string;
  condition?: string;
  timeout?: number;
  retry?: RetryConfig;
}

interface WorkflowEdge {
  from: string;
  to: string;
  condition?: string;
  priority?: number;
}

interface RetryConfig {
  max_attempts: number;
  backoff: "linear" | "exponential";
  delay_ms: number;
}

interface ReasoningFramework {
  mode: "chain_of_thought" | "supervised" | "hidden" | "visible";
  max_depth: number;
  allow_reflection: boolean;
  reflection_prompts?: string[];
  temperature?: number;
}

interface EventDefinition {
  event: string;
  action: string;
  condition?: string;
  priority?: number;
}

interface RuntimeDefinition {
  engine: "node" | "bun" | "deno" | "rust" | "python" | "wasm" | "microvm";
  sandbox: "browser" | "server" | "microvm" | "tauri";
  requirements: RuntimeRequirement[];
  environment?: Record<string, string>;
  timeouts?: RuntimeTimeouts;
}

interface RuntimeRequirement {
  type: "package" | "permission" | "resource";
  name: string;
  version?: string;
  optional?: boolean;
}

interface RuntimeTimeouts {
  initialization_ms?: number;
  execution_ms?: number;
  cleanup_ms?: number;
}

interface AgentLinks {
  dependencies: string[];
  dependents: string[];
  related_artifacts?: string[];
}

interface Signature {
  hash: string;
  algorithm: string;
  signed_at?: string;
  signer?: string;
}

// Enhanced functionality types
interface EvaluationFramework {
  metrics: {
    accuracy?: number;
    response_time_ms?: number;
    tool_success_rate?: number;
    context_relevance?: number;
    safety_score?: number;
  };
  testing: {
    unit_tests: boolean;
    integration_tests: boolean;
    evals: string[];
  };
  monitoring: {
    telemetry_enabled: boolean;
    alerts: string[];
    logging_level: "debug" | "info" | "warn" | "error";
  };
}

interface PlanningCapabilities {
  enabled: boolean;
  max_steps: number;
  strategy: "single_model_orchestration" | "multi_agent" | "hierarchical";
  fallback_to_human: "never" | "on_uncertainty" | "on_failure";
  confidence_threshold?: number;
}
```

### Knowledge Base Type Definitions

```typescript
interface AgentKnowledgeBase {
  id: string;
  name: string;
  description?: string;
  agent_id: string;
  documents: KnowledgeDocument[];
  embeddings_config: EmbeddingsConfig;
  vector_config: VectorConfig;
  metadata?: Record<string, any>;
}

interface KnowledgeDocument {
  id: string;
  title: string;
  content: string;
  metadata: DocumentMetadata;
  embeddings?: number[];
  chunks: DocumentChunk[];
  created_at: string;
  updated_at: string;
}

interface DocumentMetadata {
  source: string;
  created_at: string;
  tags: string[];
  mime_type?: string;
  size_bytes?: number;
  language?: string;
  author?: string;
}

interface DocumentChunk {
  content: string;
  embedding: number[];
  metadata: ChunkMetadata;
  position: number;
  similarity_score?: number;
}

interface ChunkMetadata {
  start_char: number;
  end_char: number;
  section?: string;
  importance_score?: number;
}

interface EmbeddingsConfig {
  model: string;
  dimensions: number;
  chunk_size: number;
  overlap: number;
  similarity_metric: "cosine" | "euclidean" | "dot_product";
}

interface VectorConfig {
  provider: "supabase" | "pinecone" | "weaviate" | "qdrant" | "chroma";
  index_name: string;
  similarity_metric: "cosine" | "euclidean" | "dot_product";
  dimension: number;
  host?: string;
  api_key?: string;
  additional_config?: Record<string, any>;
}
```

---

# Best Practices for Agent Artifacts

## Observability and Monitoring

Following industry best practices from AWS and Comet.ai, agent artifacts should
implement comprehensive observability:

### Required Observability Features

- **Comprehensive Tracing**: Implement tracing before production deployment to
  illuminate the agent's entire reasoning process, transforming opaque
  chain-of-thoughts into transparent workflows.

- **Performance Metrics**: Track response time, accuracy, and resource
  utilization.

- **Error Handling**: Robust error boundaries with fallback mechanisms.

- **Logging Standards**: Structured logging for all agent activities, tool
  calls, and decision points.

### Evaluation Framework

```json
"evaluation": {
  "metrics": {
    "accuracy": 0.95,
    "response_time_ms": 500,
    "tool_success_rate": 0.98
  },
  "testing": {
    "unit_tests": true,
    "integration_tests": true,
    "evals": ["task_completion", "safety", "performance"]
  },
  "monitoring": {
    "telemetry_enabled": true,
    "alerts": ["error_rate > 5%", "response_time > 1000ms"]
  }
}
```

## Tool Design Guidelines

Based on Anthropic's research on effective tool design:

### Tool Specification Best Practices

- **Clear Input/Output Schemas**: Well-defined JSON schemas for tool inputs and
  outputs.

- **Error Handling**: Tools must handle errors gracefully and provide meaningful
  error messages.

- **Resource Limits**: Define computational and time limits to prevent runaway
  operations.

- **Security Boundaries**: Sandbox execution environments to prevent malicious
  actions.

### Tool Categories

- **Information Retrieval**: Search, database queries, API calls
- **Content Generation**: Writing, code generation, creative tasks
- **System Operations**: File operations, process management
- **External Integrations**: Third-party service interactions

## Autonomy and Planning

Drawing from levels of autonomy frameworks:

### Autonomy Levels

- **Level 1 (Low)**: Binary choices, simple classifications
- **Level 2 (Medium)**: Routine tasks, guided workflows
- **Level 3 (High)**: Complex planning, multi-step reasoning, tool orchestration

### Planning Capabilities

```json
"planning": {
  "enabled": true,
  "max_steps": 10,
  "strategy": "single_model_orchestration",
  "fallback_to_human": "on_uncertainty"
}
```

## Framework Selection

- **Favor Single-Model Orchestration**: Simpler architectures over complex
  multi-agent systems
- **Modular Design**: Break complex agents into focused, composable units
- **Evaluation-First**: Establish objective metrics before scaling
- **Incremental Deployment**: Start with routine tasks, expand autonomy
  gradually

---

# 15. Example Full Agent Artifact

```json
{
  "schema": "https://prometheusags.ai/specs/agent-artifact/1.0.0",
  "id": "pas://agent/97e5ab8a",
  "type": "agent.behavior",
  "subtype": "expert-agent",
  "version": "1.0.0",
  "metadata": {
    "title": "Research Expert",
    "description": "Performs deep research and synthesis.",
    "tags": ["research", "analysis"],
    "language": "none",
    "author": "system",
    "permissions": {
      "read": true,
      "write": true,
      "execute": true
    }
  },
  "agent": {
    "role": "expert-agent",
    "objectives": [
      "Gather information",
      "Analyze sources",
      "Produce accurate summaries"
    ],
    "prompt": "You are a world-class research analyst...",
    "memory": {
      "short_term": [],
      "long_term": []
    },
    "tools": [
      "search.web",
      "prometheus.agui.workspace.update"
    ],
    "workflow": {
      "type": "directed_acyclic_graph",
      "nodes": [
        { "id": "n1", "action": "collect_input" },
        { "id": "n2", "action": "tool:search.web" },
        { "id": "n3", "action": "summarize" }
      ],
      "edges": [
        { "from": "n1", "to": "n2" },
        { "from": "n2", "to": "n3" }
      ]
    },
    "reasoning": {
      "mode": "chain_of_thought",
      "max_depth": 8,
      "allow_reflection": true
    },
    "events": [
      {
        "event": "onStart",
        "action": "announce_ready"
      }
    ]
  },
  "runtime": {
    "engine": "node",
    "sandbox": "browser"
  },
  "links": { "dependencies": [], "dependents": [] },
  "signature": { "hash": "...", "algorithm": "sha256" }
}
```

---

# End of PAS-X Agent Artifact Specification
