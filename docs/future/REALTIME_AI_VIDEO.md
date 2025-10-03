# Universal AI Voice & Real-Time Knowledge Platform

**Project:** llm-supabase-rs (Enhanced)  
**Version:** 3.0 - Voice & Real-Time Features  
**Date:** October 2, 2025

---

## Executive Summary

This document specifies the evolution of the Universal AI Server Proxy into a comprehensive **Voice AI and Real-Time Knowledge Platform** that combines:

1. **Voice AI Assistant** - Real-time conversational AI with any LLM backend
2. **LiveKit Agent Mode** - Joins calls as an AI participant for transcription, Q&A, and real-time knowledge retrieval
3. **Live Stream Enhancement** - AI-powered presentations, podcasts, and events with real-time Q&A from knowledge bases
4. **Omi.me Integration** - Medical transcription backend supporting custom Flutter app for healthcare professionals
5. **Real-Time Vector Embedding** - Continuous knowledge base updates from live streams

**Unique Value Proposition:**

> One Rust Server. Multiple Real-Time AI Capabilities.
>
> - ✅ Voice conversations (user → AI)
> - ✅ AI agent in calls (joins as participant)
> - ✅ Live transcription + embedding
> - ✅ Real-time knowledge Q&A
> - ✅ Any LLM backend (9+ providers)
> - ✅ Omi.me API compatible
> - ✅ Self-hosted option

---

## Use Cases & Market Opportunity

### 1. Live Events & Webinars

**Scenario:** Technical conference presentation with real-time AI Q&A

```mermaid
graph LR
    A[Presenter] -->|Speaks| B[LiveKit Room]
    C[Audience] -->|Watches| B
    B -->|Audio Stream| D[AI Agent]
    D -->|Transcribes| E[Vector DB]
    D -->|Embeds Slides| E
    C -->|Asks Question| F[Chat]
    F -->|Routes to AI| D
    D -->|Searches| E
    D -->|Answers with Context| F
```

**Value:**

- Audience gets instant answers from presentation content
- Presenter doesn't interrupt flow
- Questions answered with citations from slides/transcript
- **Market:** $2B event technology market

### 2. Medical Consultations (Omi.me)

**Scenario:** Doctor-patient consultation with real-time clinical support

```mermaid
sequenceDiagram
    participant D as Doctor (Omi Device)
    participant S as AI Server
    participant K as Knowledge Base
    participant L as LLM
    
    D->>S: Audio stream (consultation)
    S->>S: Transcribe (Deepgram)
    S->>K: Embed symptoms/context
    D->>S: "What's the differential diagnosis?"
    S->>K: Vector search medical knowledge
    K-->>S: Relevant medical info
    S->>L: Generate diagnosis suggestions
    L-->>S: Clinical recommendations
    S-->>D: Answer with citations
    S->>K: Store consultation summary
```

**Value:**

- Real-time clinical decision support
- Automatic consultation documentation
- Patient history knowledge base
- HIPAA-compliant self-hosted option
- **Market:** $10B healthcare AI market

### 3. Podcast Recording & Enhancement

**Scenario:** Podcast with AI co-host and real-time fact-checking

```mermaid
graph TB
    subgraph "Recording Session"
        A[Host 1] -->|Audio| L[LiveKit Room]
        B[Host 2] -->|Audio| L
        C[AI Agent] -->|Joins as Participant| L
    end
    
    subgraph "AI Processing"
        L -->|Streams Audio| D[Transcription]
        D -->|Text| E[Real-time Embedding]
        E -->|Context| F[Knowledge Base]
        L -->|Question Detected| G[AI Response]
        G -->|Searches| F
        G -->|Generates Answer| H[TTS]
        H -->|Audio| L
    end
    
    subgraph "Output"
        L -->|Recording| I[Edited Podcast]
        D -->|Transcript| J[Show Notes]
        F -->|Topics| K[Searchable Archive]
    end
```

**Value:**

- AI participates naturally in conversation
- Automatic show notes and timestamps
- Searchable podcast archive with semantic search
- **Market:** $2B podcast industry

### 4. Customer Support Training

**Scenario:** Training session with live feedback

```mermaid
graph LR
    A[Trainer] -->|Presents| B[LiveKit Room]
    C[Trainees] -->|Attend| B
    D[AI Agent] -->|Monitors| B
    D -->|Transcribes| E[Training Material DB]
    C -->|Questions| F[Q&A Chat]
    D -->|Answers from Material| F
    D -->|Identifies Knowledge Gaps| G[Feedback Report]
    G -->|Suggests Improvements| A
```

**Value:**

- Instant answers from training materials
- Identifies common questions/confusion points
- Continuous improvement of training content
- **Market:** $300B corporate training market

---

## System Architecture

### High-Level Overview

```mermaid
graph TB
    subgraph "Client Applications"
        W[Web Client]
        M[Mobile App - Omi]
        P[Phone System]
        D[Desktop App]
    end
    
    subgraph "LiveKit Infrastructure"
        L[LiveKit SFU]
        R[Rooms & Sessions]
    end
    
    subgraph "Universal AI Server (Rust)"
        direction TB
        A[API Gateway]
        A --> V[Voice Session Manager]
        A --> AG[Agent Manager]
        A --> O[Omi.me API Handler]
        
        V --> ST[STT Engine]
        V --> TT[TTS Engine]
        AG --> ST
        AG --> RT[Real-time Transcription]
        
        RT --> VE[Vector Embedder]
        VE --> VD[Vector Database]
        
        V --> LO[LLM Orchestrator]
        AG --> LO
        O --> LO
        
        LO --> PR[Provider Registry]
        PR --> VP[Vertex AI]
        PR --> BP[Bedrock]
        PR --> OP[OpenAI]
        PR --> X[... 9 providers]
    end
    
    subgraph "Storage"
        VD[Vector DB - pgvector]
        S[Supabase PostgreSQL]
        F[File Storage]
    end
    
    W --> L
    M --> L
    P --> L
    D --> L
    L --> A
    A --> S
    VE --> VD
```

### Data Flow Architecture

```mermaid
flowchart TD
    subgraph "Input Sources"
        U1[User Voice]
        U2[Agent Audio Stream]
        U3[Presentation Slides]
        U4[Document Upload]
    end
    
    subgraph "Processing Pipeline"
        S1[Speech-to-Text]
        S2[Text Processing]
        S3[Chunking & Embedding]
        S4[Vector Storage]
    end
    
    subgraph "Real-Time Query"
        Q1[User Question]
        Q2[Question Embedding]
        Q3[Vector Search]
        Q4[Context Retrieval]
        Q5[LLM Generation]
        Q6[Response]
    end
    
    U1 --> S1
    U2 --> S1
    U3 --> S2
    U4 --> S2
    
    S1 --> S2
    S2 --> S3
    S3 --> S4
    
    Q1 --> Q2
    Q2 --> Q3
    Q3 --> S4
    S4 --> Q4
    Q4 --> Q5
    Q5 --> Q6
    
    style S4 fill:#90EE90
    style Q5 fill:#87CEEB
```

---

## Core Features

### Feature Matrix

| Feature                 | Voice Chat | Agent Mode | Omi.me | Live Stream |
| ----------------------- | ---------- | ---------- | ------ | ----------- |
| Real-time transcription | ✅          | ✅          | ✅      | ✅           |
| Voice synthesis         | ✅          | ✅          | ✅      | ✅           |
| Multi-LLM support       | ✅          | ✅          | ✅      | ✅           |
| Vector search           | ✅          | ✅          | ✅      | ✅           |
| Joins as participant    | ❌          | ✅          | ❌      | ✅           |
| Document embedding      | ✅          | ✅          | ✅      | ✅           |
| Real-time Q&A           | ✅          | ✅          | ✅      | ✅           |
| Omi.me API              | ❌          | ❌          | ✅      | ❌           |
| Medical features        | ❌          | ❌          | ✅      | ❌           |
| Recording               | ✅          | ✅          | ✅      | ✅           |
| Self-hosted             | ✅          | ✅          | ✅      | ✅           |

---

## LiveKit Agent Mode

### Agent Architecture

```mermaid
graph TB
    subgraph "LiveKit Room"
        U1[User 1]
        U2[User 2]
        U3[User 3]
        A[AI Agent]
    end
    
    subgraph "Agent Capabilities"
        L[Listen to All Participants]
        T[Transcribe in Real-time]
        E[Embed Content]
        M[Monitor for Questions]
        R[Respond with Voice]
        S[Summarize Discussion]
    end
    
    subgraph "Knowledge Sources"
        K1[Pre-loaded Docs]
        K2[Presentation Slides]
        K3[Live Transcript]
        K4[External APIs]
    end
    
    A --> L
    L --> T
    T --> E
    E --> K3
    L --> M
    M --> R
    
    R -.->|Searches| K1
    R -.->|Searches| K2
    R -.->|Searches| K3
    R -.->|Searches| K4
    
    U1 -.->|Asks Question| M
    U2 -.->|Asks Question| M
    U3 -.->|Asks Question| M
```

### Agent Modes

```mermaid
stateDiagram-v2
    [*] --> Passive: Agent Joins Room
    
    Passive --> Listening: Start Monitoring
    Listening --> Transcribing: Audio Detected
    Transcribing --> Embedding: Text Available
    Embedding --> Listening: Continue
    
    Listening --> QuestionDetected: Trigger Phrase
    QuestionDetected --> Searching: Vector Search
    Searching --> Generating: Context Retrieved
    Generating --> Speaking: TTS
    Speaking --> Listening: Complete
    
    Listening --> Summarizing: Request Summary
    Summarizing --> Speaking: Generate
    
    Speaking --> [*]: End Session
    
    note right of Passive
        Agent is silent but
        processing audio
    end note
    
    note right of QuestionDetected
        Triggers:
        - "AI, can you..."
        - @mention
        - Hand raise + voice
    end note
```

### Agent Implementation

**Key Components:**

1. **Agent Join Flow**

```mermaid
sequenceDiagram
    participant C as Client/Host
    participant S as AI Server
    participant L as LiveKit
    participant A as AI Agent
    
    C->>S: POST /v1/agent/join
    Note over C,S: {room_name, mode, knowledge_sources}
    
    S->>L: Create agent token
    L-->>S: Token
    
    S->>A: Initialize agent
    A->>L: Join room as participant
    L-->>A: Connected
    
    A->>A: Subscribe to all audio tracks
    A->>S: Start transcription pipeline
    
    S-->>C: {agent_id, status: "joined"}
    
    loop Real-time Processing
        A->>A: Receive audio chunks
        A->>S: Transcribe
        S->>S: Embed transcript
        S->>S: Monitor for questions
        
        alt Question Detected
            S->>S: Vector search
            S->>S: Generate answer
            S->>A: TTS audio
            A->>L: Publish audio track
        end
    end
```

2. **Activation Patterns**

```mermaid
graph LR
    subgraph "Agent Listening"
        A[Audio Stream]
    end
    
    subgraph "Trigger Detection"
        B[Wake Word: 'AI']
        C[At Mention in Chat]
        D[Hand Raise + Voice]
        E[Direct Message]
    end
    
    subgraph "Response Types"
        F[Voice Answer]
        G[Text Answer]
        H[Both]
    end
    
    A --> B
    A --> D
    C --> E
    
    B --> F
    C --> G
    D --> H
    E --> G

```

---

## Real-Time Knowledge System

### Vector Embedding Pipeline

```mermaid
flowchart LR
    subgraph "Content Sources"
        A1[Live Audio]
        A2[Uploaded Docs]
        A3[Slides/PDFs]
        A4[Chat Messages]
    end
    
    subgraph "Processing"
        B1[Transcription]
        B2[Text Extraction]
        B3[Chunking]
        B4[Embedding Model]
    end
    
    subgraph "Storage"
        C1[Vector DB]
        C2[Metadata]
        C3[Original Content]
    end
    
    subgraph "Retrieval"
        D1[Query Embedding]
        D2[Similarity Search]
        D3[Reranking]
        D4[Context Assembly]
    end
    
    A1 --> B1
    A2 --> B2
    A3 --> B2
    A4 --> B3
    
    B1 --> B3
    B2 --> B3
    B3 --> B4
    B4 --> C1
    
    C1 --> D2
    D1 --> D2
    D2 --> D3
    D3 --> D4
    
    style C1 fill:#90EE90
    style D4 fill:#FFB6C1
```

### Knowledge Base Structure

```mermaid
erDiagram
    SESSION ||--o{ CONTENT_CHUNK : contains
    SESSION ||--o{ PARTICIPANT : has
    SESSION ||--o{ QUESTION : receives
    
    SESSION {
        uuid id
        string type
        timestamp start_time
        timestamp end_time
        json metadata
    }
    
    CONTENT_CHUNK {
        uuid id
        uuid session_id
        text content
        vector embedding
        string source_type
        timestamp created_at
        json metadata
    }
    
    PARTICIPANT {
        uuid id
        uuid session_id
        string name
        string role
        timestamp joined_at
    }
    
    QUESTION {
        uuid id
        uuid session_id
        uuid participant_id
        text question
        text answer
        json context_used
        timestamp asked_at
    }
    
    CONTENT_CHUNK ||--o{ QUESTION : informs
```

### Semantic Search Flow

```mermaid
sequenceDiagram
    participant U as User
    participant A as AI Agent
    participant E as Embedder
    participant V as Vector DB
    participant L as LLM
    
    U->>A: "What did they say about pricing?"
    A->>E: Embed query
    E-->>A: Query vector
    
    A->>V: Similarity search
    Note over V: SELECT *, embedding <=> query_vector<br/>ORDER BY similarity<br/>LIMIT 10
    
    V-->>A: Top 10 chunks with context
    
    A->>A: Rerank by relevance
    A->>A: Assemble context
    
    A->>L: Generate answer with context
    Note over L: System: Answer using this context...<br/>Context: [relevant chunks]<br/>Question: What did they say about pricing?
    
    L-->>A: Answer with citations
    A-->>U: "They discussed pricing at 23:45.<br/>Key points: [answer]<br/>Source: Slide 12, Transcript 23:45"
```

---

## Omi.me Integration

### Omi.me Overview

**What is Omi.me?**

- Open-source wearable AI device (necklace)
- Continuous audio recording
- Privacy-focused (user owns data)
- Backend API for transcription & processing
- Flutter mobile app

**Your Integration Goal:**
Build the backend server that the custom Omi Flutter app connects to for medical consultations.

### Omi.me Architecture

```mermaid
graph TB
    subgraph "Omi Device & App"
        D[Omi Necklace]
        F[Custom Flutter App]
        D -->|Bluetooth| F
    end
    
    subgraph "Your AI Server (Omi.me Backend)"
        API[Omi.me API Endpoints]
        T[Transcription Service]
        K[Medical Knowledge Base]
        LLM[LLM Orchestrator]
        S[Storage - Supabase]
    end
    
    subgraph "Medical Features"
        C1[Consultation Transcripts]
        C2[Clinical Decision Support]
        C3[Patient History]
        C4[Medical Knowledge Search]
        C5[Diagnosis Suggestions]
    end
    
    F -->|HTTPS| API
    API --> T
    T --> K
    API --> LLM
    LLM --> K
    
    K --> C1
    K --> C2
    K --> C3
    K --> C4
    LLM --> C5
    
    API --> S
```

### Omi.me API Specification

**Endpoints to Implement:**

```mermaid
flowchart TD
    A[POST /v1/omi/transcribe] --> B{Audio Format?}
    B -->|WAV| C[Process Directly]
    B -->|OPUS| D[Transcode]
    B -->|M4A| D
    
    C --> E[Deepgram/Whisper STT]
    D --> E
    
    E --> F[Store Transcript]
    F --> G[Embed for Search]
    G --> H[Return Response]
    
    I[POST /v1/omi/segment] --> J[Segment by Speaker/Topic]
    J --> K[Medical Entity Extraction]
    K --> L[Return Segments]
    
    M[POST /v1/omi/query] --> N[Vector Search Medical KB]
    N --> O[LLM Generate Answer]
    O --> P[Return with Citations]
```

**API Request/Response Examples:**

1. **Transcribe Consultation**

```json
POST /v1/omi/transcribe
Content-Type: multipart/form-data

{
  "audio": <binary>,
  "format": "opus",
  "metadata": {
    "doctor_id": "uuid",
    "patient_id": "uuid",
    "consultation_id": "uuid",
    "timestamp": "2025-10-02T10:00:00Z"
  }
}

Response:
{
  "transcription": {
    "text": "Patient reports chest pain for 3 days...",
    "segments": [
      {
        "speaker": "doctor",
        "text": "Can you describe the pain?",
        "start": 0.0,
        "end": 2.5
      },
      {
        "speaker": "patient",
        "text": "It's a sharp pain on the left side",
        "start": 3.0,
        "end": 5.5
      }
    ],
    "duration": 180.5,
    "language": "en"
  },
  "entities": {
    "symptoms": ["chest pain"],
    "duration": ["3 days"],
    "characteristics": ["sharp", "left side"]
  },
  "consultation_id": "uuid",
  "stored": true
}
```

2. **Clinical Query**

```json
POST /v1/omi/query
{
  "consultation_id": "uuid",
  "question": "What are the differential diagnoses?",
  "context": {
    "include_history": true,
    "include_guidelines": true,
    "specialty": "cardiology"
  }
}

Response:
{
  "answer": "Based on the symptoms (chest pain, sharp, left side, 3 days duration), differential diagnoses include...",
  "sources": [
    {
      "type": "consultation",
      "timestamp": "2025-10-02T10:05:00Z",
      "content": "Patient reports chest pain...",
      "relevance": 0.95
    },
    {
      "type": "medical_guideline",
      "title": "AHA Chest Pain Guidelines",
      "content": "Sharp chest pain may indicate...",
      "relevance": 0.87
    }
  ],
  "suggested_tests": [
    "ECG",
    "Troponin levels",
    "Chest X-ray"
  ],
  "confidence": 0.82
}
```

3. **Real-time Session**

```json
POST /v1/omi/session/start
{
  "doctor_id": "uuid",
  "patient_id": "uuid",
  "session_type": "consultation"
}

Response:
{
  "session_id": "uuid",
  "websocket_url": "wss://server/v1/omi/ws/uuid",
  "livekit_token": "eyJhbGc...",
  "expires_at": "2025-10-02T12:00:00Z"
}

// WebSocket messages
{
  "type": "transcription",
  "data": {
    "text": "Patient reports...",
    "speaker": "patient",
    "timestamp": 1234567890
  }
}

{
  "type": "suggestion",
  "data": {
    "type": "clinical_note",
    "content": "Consider asking about family history",
    "confidence": 0.75
  }
}
```

### Medical Knowledge Base

```mermaid
graph TB
    subgraph "Knowledge Sources"
        G[Medical Guidelines]
        J[Journal Articles]
        D[Drug Database]
        P[Patient History]
        C[Past Consultations]
    end
    
    subgraph "Processing"
        E[Medical Entity Extraction]
        S[Symptom Mapping]
        V[Vector Embedding]
    end
    
    subgraph "Search & Retrieval"
        Q[Query Analysis]
        VS[Vector Similarity]
        R[Relevance Ranking]
        F[Context Filtering]
    end
    
    subgraph "Clinical Support"
        DD[Differential Diagnosis]
        TR[Treatment Recommendations]
        DR[Drug Interactions]
        AL[Allergy Checks]
    end
    
    G --> E
    J --> E
    D --> E
    P --> E
    C --> E
    
    E --> S
    S --> V
    
    Q --> VS
    VS --> V
    VS --> R
    R --> F
    
    F --> DD
    F --> TR
    F --> DR
    F --> AL
    
    style E fill:#FFB6C1
    style V fill:#90EE90
    style DD fill:#87CEEB
```

---

## Technical Implementation

### Module Structure

```
src/
├── api/
│   ├── handlers/
│   │   ├── voice.rs (existing)
│   │   ├── agent.rs (NEW)
│   │   ├── omi.rs (NEW)
│   │   └── knowledge.rs (NEW)
│   └── websocket/
│       ├── agent_ws.rs (NEW)
│       └── omi_ws.rs (NEW)
│
├── domain/
│   ├── models/
│   │   ├── agent.rs (NEW)
│   │   ├── session.rs (NEW)
│   │   ├── knowledge.rs (NEW)
│   │   └── medical.rs (NEW)
│   └── services/
│       ├── agent_orchestrator.rs (NEW)
│       ├── knowledge_manager.rs (NEW)
│       └── medical_processor.rs (NEW)
│
├── infrastructure/
│   ├── voice/ (existing)
│   ├── agent/ (NEW)
│   │   ├── livekit_agent.rs
│   │   ├── activation_detector.rs
│   │   └── agent_controller.rs
│   ├── knowledge/ (NEW)
│   │   ├── embedder.rs
│   │   ├── vector_store.rs
│   │   ├── chunker.rs
│   │   └── reranker.rs
│   ├── medical/ (NEW)
│   │   ├── entity_extractor.rs
│   │   ├── clinical_knowledge.rs
│   │   └── omi_api.rs
│   └── realtime/ (NEW)
│       ├── transcription_stream.rs
│       └── embedding_stream.rs
```

### Key Dependencies

```toml
[dependencies]
# Existing core dependencies
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
# ... all existing deps

# NEW: LiveKit Agent
livekit = "0.3"
livekit-api = "0.3"

# NEW: Vector Database
pgvector = "0.3"
sqlx = { version = "0.7", features = ["postgres", "uuid", "chrono"] }

# NEW: Embedding Models
fastembed = "3.0"  # Fast local embeddings
# Or use API-based:
# openai-api = "0.1"  # For OpenAI embeddings

# NEW: Audio Processing
symphonia = "0.5"
rubato = "0.15"
hound = "3.5"

# NEW: Medical NLP (optional, for entity extraction)
rust-bert = "0.21"  # For medical entity extraction
```

### Core Implementation: LiveKit Agent

```rust
// src/infrastructure/agent/livekit_agent.rs

pub struct LiveKitAgent {
    agent_id: Uuid,
    room: Room,
    mode: AgentMode,
    capabilities: AgentCapabilities,
    state: Arc<RwLock<AgentState>>,
    
    // Processing components
    transcriber: Arc<dyn SpeechToText>,
    embedder: Arc<KnowledgeEmbedder>,
    llm_orchestrator: Arc<RequestProcessor>,
    knowledge_manager: Arc<KnowledgeManager>,
    
    // Audio
    audio_receiver: AudioStream,
    audio_sender: AudioStream,
}

#[derive(Debug, Clone)]
pub enum AgentMode {
    Passive,        // Only transcribe and embed
    Active,         // Respond to questions
    Proactive,      // Offer suggestions
    Moderator,      // Manage discussion flow
}

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub transcription: bool,
    pub voice_response: bool,
    pub text_response: bool,
    pub knowledge_search: bool,
    pub real_time_embedding: bool,
    pub summarization: bool,
}

impl LiveKitAgent {
    pub async fn join_room(
        room_name: &str,
        mode: AgentMode,
        config: AgentConfig,
    ) -> Result<Self> {
        // Generate agent token
        let token = Self::generate_agent_token(room_name, &config).await?;
        
        // Connect to room
        let room = Room::connect(
            &config.livekit_url,
            &token,
            RoomOptions::default(),
        ).await?;
        
        // Subscribe to all participant audio
        let audio_receiver = Self::subscribe_to_all_audio(&room).await?;
        
        // Create agent audio track
        let audio_sender = room.local_participant()
            .create_audio_track("ai-agent-voice")
            .await?;
        
        let agent = Self {
            agent_id: Uuid::new_v4(),
            room,
            mode,
            capabilities: config.capabilities,
            state: Arc::new(RwLock::new(AgentState::Listening)),
            transcriber: config.transcriber,
            embedder: config.embedder,
            llm_orchestrator: config.llm_orchestrator,
            knowledge_manager: config.knowledge_manager,
            audio_receiver,
            audio_sender,
        };
        
        // Start processing loops
        tokio::spawn(agent.clone().transcription_loop());
        tokio::spawn(agent.clone().embedding_loop());
        tokio::spawn(agent.clone().question_detection_loop());
        
        Ok(agent)
    }
    
    async fn transcription_loop(self) {
        let mut transcript_buffer = String::new();
        
        while let Some(audio_chunk) = self.audio_receiver.recv().await {
            // Transcribe chunk
            let transcript = self.transcriber
                .transcribe_stream(audio_chunk)
                .await
                .unwrap();
            
            for chunk in transcript {
                transcript_buffer.push_str(&chunk.text);
                
                // Emit real-time transcript
                self.emit_transcript_event(&chunk).await;
                
                // Check for sentence completion
                if chunk.is_final && Self::is_sentence_end(&transcript_buffer) {
                    // Pass to embedding pipeline
                    self.embedder
                        .embed_and_store(&transcript_buffer, &self.agent_id)
                        .await
                        .unwrap();
                    
                    transcript_buffer.clear();
                }
            }
        }
    }
    
    async fn question_detection_loop(self) {
        // Monitor transcripts for questions directed at agent
        let mut rx = self.knowledge_manager.transcript_rx.subscribe();
        
        while let Ok(transcript) = rx.recv().await {
            // Check activation patterns
            if Self::is_agent_activated(&transcript.text) {
                // Extract question
                let question = Self::extract_question(&transcript.text);
                
                // Update state
                *self.state.write().await = AgentState::Processing;
                
                // Search knowledge base
                let context = self.knowledge_manager
                    .search(&question, &self.agent_id)
                    .await
                    .unwrap();
                
                // Generate answer
                let answer = self.llm_orchestrator
                    .generate_answer(&question, &context)
                    .await
                    .unwrap();
                
                // Respond based on mode
                match self.mode {
                    AgentMode::Active => {
                        self.speak_answer(&answer).await.unwrap();
                    }
                    _ => {
                        self.send_text_answer(&answer).await.unwrap();
                    }
                }
                
                // Back to listening
                *self.state.write().await = AgentState::Listening;
            }
        }
    }
    
    async fn speak_answer(&self, text: &str) -> Result<()> {
        // Convert text to speech
        let audio = self.tts.synthesize(text).await?;
        
        // Send to LiveKit
        self.audio_sender.send_audio(&audio).await?;
        
        Ok(())
    }
    
    fn is_agent_activated(text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        // Activation patterns
        text_lower.contains("ai,") ||
        text_lower.contains("hey ai") ||
        text_lower.contains("@ai") ||
        text_lower.contains("assistant,")
    }
}
```

### Core Implementation: Real-Time Knowledge

```rust
// src/infrastructure/knowledge/embedder.rs

pub struct KnowledgeEmbedder {
    embedding_model: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<VectorStore>,
    chunker: TextChunker,
}

impl KnowledgeEmbedder {
    pub async fn embed_and_store(
        &self,
        content: &str,
        session_id: &Uuid,
    ) -> Result<Vec<Uuid>> {
        // Chunk content
        let chunks = self.chunker.chunk(content)?;
        
        let mut chunk_ids = Vec::new();
        
        for chunk in chunks {
            // Generate embedding
            let embedding = self.embedding_model
                .generate_embeddings(vec![chunk.text.clone()])
                .await?;
            
            // Store in vector DB
            let chunk_id = self.vector_store
                .store_chunk(ContentChunk {
                    id: Uuid::new_v4(),
                    session_id: *session_id,
                    content: chunk.text,
                    embedding: embedding[0].clone(),
                    source_type: "transcript".to_string(),
                    metadata: json!({
                        "timestamp": Utc::now(),
                        "chunk_index": chunk.index,
                    }),
                })
                .await?;
            
            chunk_ids.push(chunk_id);
        }
        
        Ok(chunk_ids)
    }
}

// src/infrastructure/knowledge/vector_store.rs

pub struct VectorStore {
    pool: PgPool,
}

impl VectorStore {
    pub async fn store_chunk(&self, chunk: ContentChunk) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO content_chunks (
                id, session_id, content, embedding, source_type, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            chunk.id,
            chunk.session_id,
            chunk.content,
            chunk.embedding.as_slice(),
            chunk.source_type,
            chunk.metadata
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    pub async fn search(
        &self,
        query_embedding: &[f32],
        session_id: &Uuid,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        let results = sqlx::query_as!(
            SearchResult,
            r#"
            SELECT 
                id,
                content,
                source_type,
                metadata,
                1 - (embedding <=> $1::vector) as similarity
            FROM content_chunks
            WHERE session_id = $2
            ORDER BY embedding <=> $1::vector
            LIMIT $3
            "#,
            query_embedding,
            session_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(results)
    }
}
```

---

## API Specification

### REST Endpoints

```
# Agent Management
POST   /v1/agent/create        - Create agent configuration
POST   /v1/agent/join          - Join agent to room
GET    /v1/agent/{id}/status   - Get agent status
DELETE /v1/agent/{id}          - Remove agent from room
POST   /v1/agent/{id}/command  - Send command to agent

# Knowledge Base
POST   /v1/knowledge/upload    - Upload documents
POST   /v1/knowledge/embed     - Embed content
POST   /v1/knowledge/search    - Semantic search
GET    /v1/knowledge/session/{id} - Get session knowledge

# Omi.me API
POST   /v1/omi/transcribe      - Transcribe audio
POST   /v1/omi/segment         - Segment by speaker
POST   /v1/omi/query           - Medical query
POST   /v1/omi/session/start   - Start consultation
POST   /v1/omi/session/end     - End consultation
GET    /v1/omi/history/{patient_id} - Get patient history

# WebSocket
WS     /v1/agent/ws/{id}       - Agent real-time events
WS     /v1/omi/ws/{session_id} - Omi real-time session
```

---

## Implementation Roadmap

### Phase 1: Foundation Enhancement (4 weeks)

**Week 1-2: LiveKit Agent Core**

- [ ] Agent join/leave room functionality
- [ ] Multi-participant audio subscription
- [ ] Agent audio publishing
- [ ] Basic activation detection
- [ ] Simple Q&A with existing LLM

**Week 3-4: Vector Knowledge Base**

- [ ] pgvector integration
- [ ] Real-time embedding pipeline
- [ ] Semantic search implementation
- [ ] Context retrieval and ranking
- [ ] Knowledge base CRUD API

**Deliverable:** Agent that joins LiveKit rooms, transcribes, and answers basic questions

### Phase 2: Advanced Agent (4 weeks)

**Week 5-6: Agent Modes & Capabilities**

- [ ] Passive/Active/Proactive modes
- [ ] Activation pattern detection
- [ ] Voice response via TTS
- [ ] Concurrent multi-room support
- [ ] Agent state management

**Week 7-8: Real-Time Processing**

- [ ] Streaming transcription
- [ ] Streaming embedding
- [ ] Real-time context updates
- [ ] Question queue management
- [ ] Response prioritization

**Deliverable:** Full-featured agent with multiple modes and real-time knowledge updates

### Phase 3: Omi.me Integration (4 weeks)

**Week 9-10: Omi.me API**

- [ ] Omi.me endpoint implementation
- [ ] Audio format handling (OPUS, M4A, WAV)
- [ ] Speaker diarization
- [ ] Medical entity extraction
- [ ] Consultation storage

**Week 11-12: Medical Features**

- [ ] Medical knowledge base
- [ ] Clinical decision support
- [ ] Patient history integration
- [ ] Diagnosis suggestions
- [ ] Drug interaction checking

**Deliverable:** Working Omi.me backend for medical consultations

### Phase 4: Production Ready (4 weeks)

**Week 13-14: Performance & Scale**

- [ ] Connection pooling optimization
- [ ] Embedding batch processing
- [ ] Caching layer
- [ ] Load testing
- [ ] Resource optimization

**Week 15-16: Polish & Deploy**

- [ ] Documentation
- [ ] Example applications
- [ ] Deployment guides
- [ ] Security audit
- [ ] HIPAA compliance (Omi.me)

**Deliverable:** Production-ready platform with all features

---

## Success Metrics

### Technical Metrics

- Agent join latency: < 2s
- Transcription delay: < 500ms
- Embedding latency: < 200ms
- Search response time: < 100ms
- End-to-end Q&A latency: < 2s
- Concurrent agents: 100+
- Concurrent rooms: 50+

### Business Metrics

- Medical consultations: 1,000+/month
- Live events enhanced: 100+/month
- Knowledge base size: 1M+ chunks
- Query accuracy: >85%
- User satisfaction: >4.5/5

---

## Conclusion

This platform combines voice AI, real-time knowledge processing, and specialized medical features into one unified Rust server. The architecture enables:

1. **Versatility** - Multiple use cases from one codebase
2. **Performance** - Rust + async for high concurrency
3. **Privacy** - Self-hosted option for sensitive data
4. **Extensibility** - Plugin architecture for new features
5. **Cost Efficiency** - Single server, multiple capabilities

**Market Position:** The only platform combining voice AI, LiveKit agent mode, real-time knowledge, and Omi.me medical integration with multi-LLM support.

**Timeline:** 16 weeks to production-ready platform  
**Investment:** $150K-250K (dev + infrastructure)  
**Revenue Potential:** $2-5M ARR within 18 months