# Tool‑Calling Landscape (2024‑10)

Below is a **complete, side‑by‑side analysis** of the eight major “LLM‑as‑a‑service” ecosystems that expose a **function / tool calling** capability today:

| Ecosystem | Representative providers / models | Public API style | How a tool is **declared** in the request | How a tool is **returned** in the response | # of tool calls a model can emit in **one assistant turn** | Does the model **pause** generation while the tool runs? | Parallel‑execution support (from the orchestrator’s point of view) | Error‑handling contract | Official client libraries |
|-----------|-----------------------------------|------------------|-------------------------------------------|---------------------------------------------|-----------------------------------------------------------|------------------------------------------------------------|-------------------------------------------------------------|------------------------|---------------------------|
| **OpenAI** | `gpt‑4o`, `gpt‑4‑turbo`, `gpt‑3.5‑turbo` | **OpenAI Chat Completion** (REST) | `tools: [{type:"function", function:{name, description, parameters}}]` | *Legacy*: `function_call` (single)  <br>*New (Oct 2023+)*: `tool_calls` array (each with `id`, `function.name`, `function.arguments`) | **Legacy** – 1 call only. <br>**New spec** – multiple calls (array) but the spec still expects **one** follow‑up request that bundles all results. | Yes – the model stops emitting tokens at the call boundary. You must send a new request that includes the result(s) before any more tokens are generated. | The spec does **not** prescribe parallelism. You *can* run the calls concurrently on your side, but the client sees a single round‑trip. | Return `function_error` (legacy) or a `tool_result` with `"is_error": true`. The model may retry, ask for clarification, or fall back to plain text. | Official SDKs (Python, Node, Go, Java, Ruby, .NET, etc.) – all expect the JSON shapes above. |
| **Groq** | `llama‑3.1‑maverick‑8b`, `gpt‑oss‑120b`, `gpt‑oss‑20b` | **Groq Chat Completion** – a **drop‑in OpenAI‑compatible** endpoint (`https://api.groq.com/openai/v1/chat/completions`) | **Exactly the same** JSON as OpenAI (`tools` array). | **Exactly the same** JSON (`function_call` or `tool_calls`). | **Effectively 1** – Groq currently emits at most **one** `function_call`. The `tool_calls` array is accepted but Groq never populates more than one element. | Yes – the model halts at the call token. You must post a second request with the result. | Not applicable (single call). | Same as OpenAI (`function_error` or `tool_result.is_error`). | Because Groq advertises “OpenAI‑compatible”, you can reuse the **official OpenAI SDKs** unchanged. |
| **Vertex AI Gemini** | `gemini‑1.0‑pro`, `gemini‑1.5‑flash`, `gemini‑1.5‑pro` | **Vertex AI Generative AI** (`projects.locations.publishers.models.predict` or `gemini.googleapis.com/v1/models/...:generateContent`) | `tools: [{functionDeclarations: [{name, description, parameters}]}]` (Google‑style JSON). | `toolCalls` list in `GenerateContentResponse`. Each entry contains `functionName` and the **arguments** that the model generated. | **Multiple** – Gemini is built to emit **a list** of tool calls in one response (current limit ≈ 4 calls per turn, but the limit is token‑budget‑driven). | Yes – the model can emit *some* free‑form text **before** the list, then the list, then (after you return results) it can continue with more text. | The API expects you to send **one** follow‑up request that contains *all* `toolResponses` (you may resolve them concurrently). | Return a `ToolResponse` object with `status: "ERROR"` and `errorMessage`. Gemini decides whether to retry that tool or continue. | Google Cloud client libraries (`google‑cloud‑aiplatform` for Python, Java, Node, Go, Ruby). |
| **AWS Bedrock** | `anthropic.claude-3-5-sonnet-20240620`, `meta.llama3-2-70b`, `cohere.command-r-plus`, `amazon.titan-text-premier-v1:0` | **Bedrock Runtime** (`InvokeModel` – JSON payload) | `toolSpec` (array) with `name`, `description`, `inputSchema` (JSON Schema). | `toolResult` array inside the response payload. Each result is paired with the `toolUseId` you sent. | **Multiple** – Bedrock’s spec allows a **list** of `toolUse` objects in a single assistant turn. | The model stops generating at the point it emits the `toolUse` list. You must call `InvokeModel` again with the `toolResult`s. | You may run the calls in parallel; Bedrock does not enforce order. | `toolResult` may contain `"status":"ERROR"` and an `errorMessage`. The model can ask for a retry or fallback. | AWS SDKs (`boto3` for Python, AWS SDK for JavaScript/TypeScript, Java, Go, .NET, etc.). |
| **OpenRouter** | Aggregates OpenAI, Anthropic, Google, Mistral, Llama, etc. | **Unified OpenRouter Chat Completion** (OpenAI‑compatible JSON) | Same as OpenAI `tools` field. | Same as OpenAI `function_call` / `tool_calls`. | **Depends on the underlying model**: <br>• If the routed model is OpenAI‑compatible → same limits (legacy 1, new multi‑call). <br>• If the routed model is Claude, Gemini, etc. → the underlying limits apply (multiple allowed). | The pause behaviour follows the **underlying model**. OpenRouter simply forwards the payload. | Same as the underlying model – you can parallelise if the model emitted multiple calls. | Same as underlying model (`function_error`, `tool_result.is_error`, etc.). | OpenRouter provides its own Python/JS SDKs that are thin wrappers around the OpenAI‑compatible endpoint. |
| **Together AI** | `togethercomputer/llama-3.1-70b`, `togethercomputer/CodeLlama-34b`, `togethercomputer/mixtral-8x7b` | **Together AI Chat Completion** (OpenAI‑compatible) | Same `tools` JSON as OpenAI. | Same `function_call` / `tool_calls`. | **Currently 1 call per turn** – the public API only returns a single `function_call`. The `tool_calls` array is accepted but not populated with >1 element. | Yes – generation stops at the function call token. | No parallelism needed (single call). | `function_error` or `tool_result.is_error` – identical to OpenAI. | Official Python/JS SDKs are thin wrappers around the OpenAI‑compatible endpoint. |
| **Fireworks AI** | `fireworks-ai/firefunction-7b`, `fireworks-ai/mixtral-8x22b`, `fireworks-ai/llama-3.1-8b-instruct` | **Fireworks AI Chat Completion** (OpenAI‑compatible) | Same `tools` field. | Same `function_call` / `tool_calls`. | **Single call** – the service currently emits only one `function_call`. The `tool_calls` array is accepted but never contains more than one element. | Yes – model halts at the function call. | No parallelism required. | `function_error` or `tool_result.is_error`. | Fireworks Python SDK (`fireworks-ai`) and the generic OpenAI SDK work out‑of‑the‑box. |
| **Anthropic Direct** (self‑hosted/managed, not via Vertex) | `claude-3-5-sonnet-20240620`, `claude-3-opus-20240229` | **Anthropic HTTP v1** (`/messages`) – almost identical to Vertex’s Claude endpoint but without the Vertex wrapper. | `tools: [{name, description, input_schema}]` (JSON Schema). | `tool_results` list in the response. Each result includes `tool_use_id`, `content`, and optional `is_error`. | **Multiple** – Claude can emit **any number** of `tool_use` blocks in a single assistant turn (the spec does not impose a hard ceiling). | Yes – generation stops at the list of `tool_use`s. You return all `tool_result`s in a *single* follow‑up request and the model may continue in the same turn. | You are free to run the calls in parallel; the model does not care about order. | `tool_result.is_error: true` + `error` string. Claude can retry the same tool, ask for clarification, or continue with prose. | Anthropic SDKs (`anthropic` Python/Node) and generic HTTP. |

---

## 2️⃣ Deep‑Dive Comparison of Key Dimensions

| Dimension | OpenAI | Groq | Vertex Gemini | AWS Bedrock | Anthropic Direct (Claude) | OpenRouter | Together AI | Fireworks AI |
|-----------|--------|------|---------------|------------|---------------------------|------------|-------------|--------------|
| **Request schema name** | `tools` (OpenAI) | Same | `tools` → `functionDeclarations` (Google) | `toolSpec` | `tools` (Claude) | Same as OpenAI | Same as OpenAI | Same as OpenAI |
| **Response field name** | `function_call` (legacy) / `tool_calls` (array) | Same | `toolCalls` (list) | `toolResult` (list) | `tool_results` (list) | Same as OpenAI | Same as OpenAI | Same as OpenAI |
| **Multiple calls per turn** | **New spec** allows an array; most SDKs still expect one. | No (single). | **Yes** – list of calls. | **Yes** – list of calls. | **Yes** – list of calls. | Inherited from underlying model (usually 1). | No (single). | No (single). |
| **Streaming behaviour** | Stream ends at the call token; you must open a new stream for the result. | Same. | Stream can contain text before the call list; after you send results the model may resume the same stream. | Same as Gemini – can stream pre‑call text. | Same as Gemini – can stream before/after. | Same as underlying model. | Same as OpenAI. | Same as OpenAI. |
| **Parallel execution** | Not part of spec; you may run calls concurrently but must bundle results into a single follow‑up request. | N/A (single). | **Native** – you can resolve the whole `toolCalls` list in parallel and then send a single `toolResponses` payload. | Same as Gemini – parallel is encouraged. | Same – parallel is encouraged. | Same as underlying model. | N/A. | N/A. |
| **Error‑return shape** | `function_error` (legacy) or `tool_result.is_error`. | Same. | `ToolResponse.status = "ERROR"` + `errorMessage`. | `toolResult.status = "ERROR"` + `errorMessage`. | `tool_result.is_error = true` + `error` field. | Same as underlying model. | Same as OpenAI. | Same as OpenAI. |
| **Typical client usage pattern** | 1️⃣ Send chat request → 2️⃣ Receive `function_call` → 3️⃣ Call your back‑end → 4️⃣ Send a **new** chat request with the `function` result. | Same as OpenAI. | 1️⃣ Send `generateContent` → 2️⃣ Receive `toolCalls` (maybe several) → 3️⃣ Run them (parallel ok) → 4️⃣ Call `generateContent` again with **all** `toolResponses` → 5️⃣ Generation continues in same turn. | Same as Gemini. | Same as Gemini. | Same as OpenAI unless you route to a model that supports multiple calls. | Same as OpenAI. | Same as OpenAI. |

---

## 3️⃣ Model‑Level Nuances Within an Ecosystem

| Ecosystem | Model / version | Notable difference in tool‑calling behaviour |
|-----------|-----------------|-----------------------------------------------|
| **OpenAI** | `gpt‑4o-mini` (2024‑06) | Fully supports the **new `tool_calls` array** (multiple calls). |
| | `gpt‑4‑turbo` (2024‑03) | Still limited to a **single** `function_call` (legacy). |
| **Groq** | `llama‑3.1‑maverick‑8b` | No multi‑tool support; returns a single `function_call`. |
| | `gpt‑oss‑120b` | Same limitation. |
| **Vertex Gemini** | `gemini‑1.0‑pro` | Can emit up to **4** tool calls per turn (token‑budget limited). |
| | `gemini‑1.5‑flash` / `gemini‑1.5‑pro` | Same contract; larger context windows allow **more** calls to fit. |
| **AWS Bedrock** | `anthropic.claude-3-5-sonnet-20240620` (via Bedrock) | Same as native Claude – unlimited list (subject to token budget). |
| | `meta.llama3-2-70b` | Supports the Bedrock `toolSpec` but currently only emits **one** call (model‑specific). |
| **Anthropic Direct** | `claude‑3‑opus‑20240229` | Unlimited `tool_use` list; you can run them all in parallel. |
| **OpenRouter** | Depends on routed model – e.g., `google/gemini-pro` | If routed to Gemini you get multi‑call; if routed to OpenAI you get the OpenAI limit. |
| **Together AI** | `togethercomputer/llama‑3.1‑70b` | Single `function_call` only (the service has not yet exposed multi‑tool). |
| **Fireworks AI** | `fireworks-ai/firefunction-7b` | Single call only. |

> **Take‑away:** *Only Gemini, Bedrock (when using Claude), and Anthropic Direct guarantee true multi‑tool semantics.* All other providers either restrict you to a single call or rely on the “new OpenAI `tool_calls` array” which is still **one round‑trip** for the whole batch.

---

## 4️⃣ What “parallel execution” really means

| Ecosystem | Can the LLM *itself* run calls in parallel? | Does the **API contract** let you return results in **any order**? |
|-----------|--------------------------------------------|-------------------------------------------------------------------|
| OpenAI (legacy & new) | **No** – the model stops at the call token. | You must return a **single** `tool_results` payload (or a single `function` result). Order is irrelevant because there is only one entry. |
| Groq | Same as OpenAI. | N/A |
| Gemini | **Yes** – the model emits a *list* of calls and then **waits** for a *single* `toolResponses` array. | Order **does not matter**; each response is matched by the `toolCallId`. You can resolve them concurrently. |
| Bedrock (Claude) | **Yes** – same list semantics as Gemini. | Order irrelevant; matched via `toolUseId`. |
| Anthropic Direct | **Yes** – list semantics. | Same as Gemini. |
| OpenRouter | Mirrors the underlying model. | Same as underlying. |
| Together / Fireworks | **No** – single call. | N/A |

---

## 5️⃣ Practical Guidance for Building a **Universal Adapter**

### 5.1. Normalised internal model

```python
class NormalizedToolCall:
    def __init__(self, uid: str, name: str, args: dict):
        self.id   = uid          # OpenAI id / Gemini toolCallId / Bedrock toolUseId
        self.name = name
        self.args = args
```

*All incoming responses (OpenAI, Groq, Gemini, Bedrock, Anthropic) can be mapped onto this class.*

### 5.2. Execution pipeline

1. **Receive a request** (your public endpoint advertises “OpenAI‑compatible”).
2. **Detect the target provider** (e.g., based on a query parameter `provider=bedrock` or `provider=gemini`).
3. **Translate the request** to the provider’s schema (mostly a 1‑to‑1 field rename).
4. **Forward the request** to the provider (Claude via Vertex, Bedrock, OpenAI, etc.).
5. **Parse the provider response** into a list of `NormalizedToolCall`.
6. **Run the calls**:
   * If the list length > 1 **and** the caller expects legacy OpenAI behaviour → **queue** the extra calls and return the first one only (see step 7a).
   * If the caller supports the newer multi‑tool spec → keep the list intact.
   * You may execute the calls **concurrently** (thread pool, asyncio, etc.) because the result order does not matter.
7. **Build the response** that you will send back to the *original* caller:

   - **Legacy OpenAI** (single‑call)
     ```json
     {
       "choices": [{
         "message": {
           "role": "assistant",
           "function_call": {
             "name": call.name,
             "arguments": json.dumps(call.args)
           }
         }
       }]
     }
     ```
   - **OpenAI “new spec”** (multiple calls)
     ```json
     {
       "choices": [{
         "message": {
           "role": "assistant",
           "tool_calls": [
             {"id": c.id, "function": {"name": c.name, "arguments": json.dumps(c.args)}}
             for c in calls
           ]
         }
       }]
     }
     ```
   - **Gemini / Bedrock / Anthropic** (native) – you would **forward** the exact list unchanged if you ever expose those endpoints directly.

8. **Error handling** – if a call fails, embed the error in the appropriate field (`is_error` / `status:"ERROR"` / `function_error`).

9. **Streaming** – pause the stream at the point you emit the tool call(s). Once you have the result(s) you can **resume** the same stream (Gemini/Claude) or start a **new** stream (OpenAI legacy).

### 5.3. Feature‑detection flag

Expose a health‑check endpoint:

```json
{
  "supports_multi_tool": true,          // true for Gemini, Bedrock, Claude Direct
  "supports_parallel_execution": true,  // same flag – indicates you can resolve calls concurrently
  "max_tool_calls_per_turn": 4          // Gemini/Bedrock limit (soft)
}
```

Clients can query this endpoint to decide whether to request the *new* spec or stay on the legacy single‑call flow.

### 5.4. Testing matrix

| Test case | Expected behaviour |
|-----------|--------------------|
| **OpenAI → single function** | Model stops, you return result, next request continues. |
| **OpenAI (new spec) → 3 calls** | You return a `tool_calls` array with 3 entries; a single follow‑up request contains 3 `tool_results`. |
| **Gemini → 2 calls** | Model returns a list of 2 `toolCalls`; you resolve both in parallel, return a single `toolResponses` array, and generation continues in the same turn. |
| **Bedrock (Claude) → 4 calls** | Same as Gemini. |
| **Groq → single call** | Same as OpenAI legacy. |
| **Together AI → single call** | Same as OpenAI legacy. |
| **Anthropic Direct (non‑Vertex) → 5 calls** | Same as Gemini/Bedrock. |
| **OpenRouter routing to Gemini** | Behaves like Gemini (multiple calls). |
| **OpenRouter routing to OpenAI** | Behaves like OpenAI (single or new spec). |

---

## 6️⃣ Quick Reference Matrix (One‑liner per ecosystem)

| Ecosystem | Multi‑tool support? | Pause‑generation? | Parallel‑friendly? | Typical client SDK |
|-----------|--------------------|-------------------|-------------------|--------------------|
| **OpenAI** | ✅ (new spec) – but many clients still use 1 call | ✅ – stops at call token | ✖️ (you can parallelise behind the scenes) | Official OpenAI SDKs |
| **Groq** | ❌ (single call) | ✅ | ✖️ | OpenAI SDK (compatible) |
| **Vertex Gemini** | ✅ (list) | ✅ – list marks pause | ✅ (run list in parallel) | Google Cloud SDK |
| **AWS Bedrock** | ✅ (list) | ✅ – list marks pause | ✅ (run list in parallel) | AWS SDK (`boto3`, etc.) |
| **Anthropic Direct** | ✅ (list) | ✅ – list marks pause | ✅ (run list in parallel) | Anthropic SDK |
| **OpenRouter** | Depends on routed model | Mirrors routed model | Mirrors routed model | OpenRouter SDK (thin wrapper) |
| **Together AI** | ❌ (single) | ✅ | ✖️ | OpenAI‑compatible SDK |
| **Fireworks AI** | ❌ (single) | ✅ | ✖️ | OpenAI‑compatible SDK |

---

## 7️⃣ Bottom‑Line Answer to Your Original “Will All Those Tool‑Call Semantics Be the Same?”

* **No.** The *core idea*—the model emits a name + JSON arguments and you feed a result back—is shared, but the **exact contract** (field names, ability to emit multiple calls, streaming semantics, and error format) **varies**:

| Ecosystem | Does it **always** stop generation after a tool call? | Can it emit **more than one** tool call in a single turn? |
|-----------|--------------------------------------------------------|-----------------------------------------------------------|
| OpenAI (legacy) | **Yes** – stops; you must send a new request. | **No** (single). |
| OpenAI (new `tool_calls`) | **Yes** – stops at the end of the `tool_calls` array. | **Yes**, but you still need **one** follow‑up request that contains *all* results. |
| Groq | **Yes** – same as OpenAI legacy. | **No**. |
| Vertex Gemini | **Yes** – stops at the list of `toolCalls`. | **Yes** – can return several calls; you can resolve them in parallel and send a single `toolResponses` payload. |
| AWS Bedrock | **Yes** – stops at the `toolUse` list. | **Yes** – list may contain several calls; parallel resolution is encouraged. |
| Anthropic Direct | **Yes** – stops at the `tool_use` list. | **Yes** – unlimited list (subject to token budget). |
| OpenRouter | **Depends** on the routed model (mirrors its behaviour). | **Depends** on the routed model. |
| Together AI | **Yes** – stops at the single `function_call`. | **No**. |
| Fireworks AI | **Yes** – stops at the single `function_call`. | **No**. |

*If your public endpoint advertises **OpenAI‑compatible** semantics (the vast majority of third‑party SDKs expect that), you **must** either:*

1. **Flatten** any batch you receive from Gemini/Bedrock/Anthropic into the **single‑call** flow that OpenAI legacy clients understand, **or**
2. **Upgrade** your public contract to the **new OpenAI `tool_calls` array** and make sure **every** downstream consumer uses an SDK version that supports that spec.

If you ignore this distinction, callers that rely on the legacy OpenAI contract will hit parsing errors, time‑outs, or unexpected “missing result” failures.

---

### Final Recommendation for a **single, universal server**

| Goal | Recommended mode |
|------|-------------------|
| **Maximum compatibility (all existing OpenAI SDKs)** | **Emit a single `function_call` per response**. If the upstream model gave you a batch, return the *first* call now, stash the remainder in a short‑lived queue, and serve them on subsequent turns. |
| **Leverage native multi‑tool capabilities (lower latency, parallelism)** | **Expose a new endpoint** (`/v2/chat`) that returns the **OpenAI `tool_calls` array** (or the native Gemini/Bedrock format). Publish a version flag so clients can opt‑in. |
| **Pure Gemini / Bedrock / Claude Direct** | Forward the request unchanged and return the provider‑native response; no translation needed. |

Implementing the **queue‑and‑replay** layer lets you safely support **all eight ecosystems** with a single OpenAI‑compatible façade while still taking advantage of parallel execution under the hood for the models that allow it.