## Streaming‑Behaviour Cheat‑Sheet (2024‑10)

The goal is to **take whatever the provider streams back and turn it into the exact shape the OpenAI client library expects** (i.e., a token‑by‑token `content` stream that stops cleanly at a `function_call`/`tool_calls` boundary).
The table below tells you, for each ecosystem, **what the wire‑level stream looks like**, **whether the payload arrives in “incomplete chunks” or only after a whole tool‑call block is finished**, and **how you can normalise it**.

| Ecosystem | Provider(s) | Streaming protocol used | What the stream contains **before a tool call** | How a **tool call** is delivered in the stream | Does the stream continue **after** you return the tool result? | Normalisation recipe for an OpenAI‑compatible client |
|-----------|-------------|------------------------|---------------------------------------------------|-------------------------------------------------|-----------------------------------------------------------|------------------------------------------------------|
| **OpenAI** | `gpt‑4o`, `gpt‑4‑turbo`, `gpt‑3.5‑turbo` | Server‑sent events (SSE) – each `data:` line is a **partial JSON delta** (`choices[0].delta`) that may contain `role`, `content`, or `function_call` fields. | Tokens are emitted **as soon as they are generated** (character‑by‑character or token‑by‑token). | When the model decides to call a function it sends a **single delta** that contains the entire `function_call` object (the `name` field is complete and the `arguments` field arrives **as a single JSON string**, not token‑by‑token). The stream **ends** at that delta (`finish_reason = "function_call"`). | No – the stream is closed. You must start a **new** request that includes the function result; the next response will stream the continuation. | • Pass the SSE straight through until you see `finish_reason="function_call"` (or `"tool_calls"` for the newer spec). <br>• Buffer that final delta; do **not** forward any partial `function_call` tokens because they never exist. <br>• After you have the result, issue a new request and forward the new stream to the caller. |
| **Groq** | `llama‑3.1‑maverick‑8b`, `gpt‑oss‑120b`, `gpt‑oss‑20b` | Same SSE format as OpenAI (Groq advertises “OpenAI‑compatible”). | Same token‑by‑token streaming. | Groq only ever emits **one** `function_call` delta (the whole JSON block). The stream stops (`finish_reason="function_call"`). | No – you must open a second request for the result. | Same recipe as OpenAI (just point at Groq’s endpoint). |
| **Vertex Gemini** | `gemini‑1.0‑pro`, `gemini‑1.5‑flash`, `gemini‑1.5‑pro` | **gRPC‑style streaming** (or HTTP/2 SSE when using the REST endpoint). Each `GenerateContentResponse` message contains a `candidates[0].content.parts` array. | Tokens are emitted **incrementally** inside `text` parts. You can receive a mixture of `text` parts *and* a `functionCall` part in the **same** streaming message. | When Gemini decides to call a tool it emits a **single `functionCall` part** (the whole call, including `name` and the *complete* JSON arguments). The streaming **pauses** at that point – no further `text` parts are sent until you send back a `toolResponse`. The `finish_reason` for the response is `"tool_call"` (or `"stop"` if the model decides not to call a tool). | **Yes** – after you post a `GenerateContentRequest` that contains the `toolResponses` array, Gemini continues the same logical turn and streams more `text` parts (or a final `finish_reason="stop"`). | 1. Buffer the stream until you see a `functionCall` part. <br>2. **Close** the current stream (Gemini expects a new request with `toolResponses`). <br>3. When you receive the tool result(s), open a **second** `generateContent` call **using the same conversation ID** and include the `toolResponses`. <br>4. Forward the second stream to the OpenAI caller as if it were the continuation after the `function_call`. |
| **AWS Bedrock** (Claude, Titan, Mistral, etc.) | `anthropic.claude-3-5-sonnet-20240620`, `cohere.command-r-plus`, `amazon.titan-text-premier-v1:0` | **JSON‑lines over HTTP** (`application/json`). Bedrock can stream (`stream=true`) – each line is a partial `completion` object. | Tokens are emitted incrementally (`completion` → `outputText`). | When a tool is invoked the model emits a **single `toolUse` object** (the full `name` and JSON `input`). The line that contains the `toolUse` also sets `"stopReason":"tool_use"` and **terminates** the current stream. | No – the stream is finished. You must call `InvokeModel` again with a payload that contains a `toolResult` array (one entry per `toolUseId`). The second call streams the continuation (or a final `stop`). | 1. Read Bedrock’s streaming lines until you encounter `"stopReason":"tool_use"` (or `"stopReason":"function_call"` for models that use that wording). <br>2. Extract the whole `toolUse` object (it arrives as a **complete unit**, not token‑by‑token). <br>3. Close the first stream, invoke the model again with the `toolResult(s)`, and forward the *new* stream to the OpenAI client as the continuation after the function call. |
| **Anthropic Direct** (non‑Vertex) | `claude‑3‑op…`, `claude‑3‑sonnet‑20240229` | SSE (same shape as Bedrock’s Claude) – each `data:` line is a partial `content` block. | Incremental `text` parts are streamed token‑by‑token. | When a tool is needed the model emits a **single `tool_use` block** (complete JSON arguments). The stream ends with `"stop_reason":"tool_use"`. | No – you must send a **second** request that contains a `tool_result` array. The model then resumes and streams the rest of the answer. | Same as Bedrock‑Claude: buffer until you see `stop_reason:"tool_use"`, then close the stream, send the result, forward the new stream. |
| **OpenRouter** | Aggregates OpenAI, Anthropic, Gemini, Mistral, etc. | Pass‑through of the underlying provider’s streaming format (SSE for OpenAI‑compatible models, Bedrock‑style for Claude, Gemini‑style for Gemini). | Same as the underlying model. | Same as the underlying model. | Same as the underlying model. | Detect the `provider` you routed to (usually returned in the response header `x-openrouter-model`) and apply the corresponding normalisation routine from the rows above. |
| **Together AI** | `togethercomputer/llama‑3.1‑70b`, `togethercomputer/mixtral‑8x7b‑instruct` | SSE – OpenAI‑compatible (they expose `/v1/chat/completions`). | Incremental token streaming. | They currently **only support a single** `function_call`. The call is emitted as a **single delta** (the whole JSON block). Stream ends with `finish_reason="function_call"`. | No – you must start a new request with the function result. | Same as OpenAI legacy. |
| **Fireworks AI** | `fireworks-ai/firefunction-7b`, `fireworks-ai/llama‑3.1‑8b‑instruct` | SSE – OpenAI‑compatible. | Incremental token streaming. | Emit a **single `function_call`** delta (complete JSON). Stream ends with `finish_reason="function_call"`. | No – new request needed. | Same as OpenAI legacy. |

### Quick “what the client sees” summary

| Provider | Stream looks like to the *client* (after you have normalised it) |
|----------|---------------------------------------------------------------|
| OpenAI, Groq, Together, Fireworks | `data: {"choices":[{"delta":{"content":"He"}}]}` → … → `data: {"choices":[{"delta":{"function_call":{"name":"myTool","arguments":"{…}"}}}]}` → `data: [DONE]`. |
| Gemini, Bedrock‑Claude, Anthropic Direct | `text` parts → … → **single** `functionCall` / `toolUse` part (complete JSON) → **stream closed** (`stopReason: "tool_use"`). |
| OpenRouter | Depends on the routed model – you must inspect the `provider` header and apply the appropriate rule. |

---

## 2️⃣ How to **normalize** everything to the *OpenAI* contract

Below is a **step‑by‑step algorithm** you can embed in your universal adapter (pseudo‑code). It works for **any** of the ecosystems listed.

```python
def normalize_stream(provider, raw_stream):
    """
    Parameters
    ----------
    provider: str  # e.g. "openai", "groq", "gemini", "bedrock", "anthropic", "openrouter"
    raw_stream: an iterator yielding raw chunks from the provider
    Returns
    -------
    generator yielding OpenAI‑compatible SSE lines (bytes)
    """
    buffer = ""          # collects partial text for OpenAI style
    tool_call = None

    for chunk in raw_stream:
        # ---------- 1️⃣ Identify provider‑specific shape ----------
        if provider in ("openai", "groq", "together", "fireworks"):
            # chunk is already an OpenAI delta JSON string (already token‑by‑token)
            data = json.loads(chunk)
            # check for function call
            if "function_call" in data.get("choices", [{}])[0].get("delta", {}):
                # whole function call is already complete – forward as‑is
                yield f"data: {json.dumps(data)}\n\n".encode()
                # End of stream – we must stop here
                break
            else:
                # regular token – forward as‑is
                yield f"data: {json.dumps(data)}\n\n".encode()
                continue

        elif provider == "gemini":
            # chunk is a protobuf / json message with `parts`
            msg = parse_gemini_message(chunk)          # pseudo
            for part in msg["candidates"][0]["content"]["parts"]:
                if "text" in part:
                    # incremental token (Gemini may give several characters at a time)
                    buffer += part["text"]
                    # emit OpenAI‑style token deltas (1 token = 1 char here – OK for most SDKs)
                    yield f'data: {json.dumps({"choices":[{"delta":{"content":"{part["text"]}"}}]})}\n\n'.encode()
                elif "functionCall" in part:
                    # complete function call arrives as a whole part
                    tool_call = part["functionCall"]
                    # flush any pending buffer first
                    if buffer:
                        yield f'data: {json.dumps({"choices":[{"delta":{"content":buffer}}]})}\n\n'.encode()
                        buffer = ""
                    # emit the OpenAI‑style function_call delta (single JSON object)
                    openai_fc = {
                        "choices": [{
                            "delta": {
                                "function_call": {
                                    "name": tool_call["name"],
                                    "arguments": json.dumps(tool_call["args"])
                                }
                            }
                        }]
                    }
                    yield f'data: {json.dumps(openai_fc)}\n\n'.encode()
                    # Gemini’s stream is now finished – break
                    break

        elif provider in ("bedrock", "anthropic"):
            # chunk is a line of JSON; look for stopReason == "tool_use"
            line = json.loads(chunk)
            if line.get("stopReason") == "tool_use":
                # extract the whole toolUse payload (already complete)
                tool_use = line["toolUse"]
                # flush any buffered text
                if buffer:
                    yield f'data: {json.dumps({"choices":[{"delta":{"content":buffer}}]})}\n\n'.encode()
                    buffer = ""
                # emit a single OpenAI‑style function_call delta
                fc = {
                    "choices": [{
                        "delta": {
                            "function_call": {
                                "name": tool_use["name"],
                                "arguments": json.dumps(tool_use["input"])
                            }
                        }
                    }]
                }
                yield f'data: {json.dumps(fc)}\n\n'.encode()
                # end of stream – do not forward the provider's `[DONE]` line
                break
            else:
                # regular token
                text = line.get("outputText", "")
                buffer += text
                # emit token‑by‑token (you can split on whitespace or just send whole chunk)
                yield f'data: {json.dumps({"choices":[{"delta":{"content":text}}]})}\n\n'.encode()

        elif provider == "openrouter":
            # OpenRouter adds a header `x-openrouter-model` that tells you the underlying provider.
            # Dispatch to the appropriate branch above.
            underlying = get_underlying_provider()  # e.g. "gemini", "anthropic", "openai"
            yield from normalize_stream(underlying, [chunk])  # recurse with same logic
            # Note: OpenRouter may also send a single SSE line that already looks like OpenAI.
            # The generic case above will just forward it unchanged.

    # If we exit the loop without hitting a tool call, we must send the OpenAI `[DONE]` line.
    if not tool_call:
        yield b"data: [DONE]\n\n"
```

**Key points of the algorithm**

* **Never forward a *partial* function‑call payload** – every provider sends the **entire** arguments in one block.
* **Buffer any ordinary text** that arrives before the tool block, then flush it *before* emitting the `function_call` delta.
* **Close the stream** (`[DONE]`) **immediately after** you have emitted the `function_call`. The OpenAI client expects the stream to end, otherwise it will keep waiting for more tokens.
* **Start a brand‑new request** (same `conversation_id`/`session_id` if the provider uses one) that contains the tool result(s). The response of that second request can be streamed back to the OpenAI caller **as if it were a continuation** of the first turn.

---

## 3️⃣ Edge Cases & Gotchas

| Situation | What happens on the wire | How to handle it |
|-----------|---------------------------|------------------|
| **Multiple tool calls in one Gemini/Claude response** | The stream ends *once* the `toolCalls` list is emitted (the list may contain 2‑4 items). No more `text` parts are sent. | **Collect all calls**, run them (parallel is fine), then send a *single* `toolResponses` array back to Gemini/Claude. For the OpenAI façade you have two options: <br>1. **Flatten** – return the *first* call now, stash the rest for later turns. <br>2. **Batch** – if you expose the new OpenAI `tool_calls` array, forward the whole list at once. |
| **Tool‑call arguments are huge (> 4 KB)** | OpenAI caps a `function_call.arguments` string at ~4 KB. Gemini/Claude have a larger limit (≈ 16 KB). | If you need to stay OpenAI‑compatible, **truncate or paginate** the arguments (e.g., split a large JSON into multiple calls) or reject the request with a clear error. |
| **Streaming disabled by the provider** | Some providers (e.g., older Groq models) only support **non‑streaming** `POST` calls. | Simulate streaming for OpenAI callers by **chunking the full answer** yourself (e.g., split the final text into 20‑token chunks and emit SSE lines). |
| **Tool call appears *after* some `function_call` text** (rare, only in Gemini) | Gemini may emit a few `text` tokens, then a `functionCall` part, then **no more text** until you respond. | Same as normal: flush the buffered text, emit the `function_call` delta, close the stream. |
| **Provider returns a *partial* `function_call` (some experimental models)** | Very rare; you may see a `function_call` delta with only `name` and an empty `arguments`. | Buffer until you see the *complete* `arguments` field (most providers guarantee atomic delivery). If the stream ends prematurely, treat it as an error and surface a `function_error` to the client. |
| **OpenRouter mixes providers in the same request** | Not possible – each request is routed to a single underlying model, but the **metadata** (`x-openrouter-model`) tells you which one. | Use that header to pick the right normalisation routine. |

---

## 4️⃣ Normalising **to the OpenAI** “single‑call” contract (the safest for *all* existing SDKs)

If you want **100 % compatibility** with the vast majority of OpenAI client libraries (including the older ones that don’t understand the `tool_calls` array), implement the **single‑call façade**:

1. **Detect the first tool call** in the upstream stream (whether it’s a single call or a list).
2. **Emit only that first call** as an OpenAI `function_call` delta and **close the stream** (`[DONE]`).
3. **Persist the remaining calls** (if any) in a short‑lived store keyed by `conversation_id`.
4. When the client sends back the result for the first call, **pop the next queued call** (if any) and repeat the process.

*Advantages*: works with every OpenAI SDK, no need for the consumer to upgrade.
*Trade‑off*: extra round‑trip latency for models that could have sent the whole batch at once (Gemini, Claude).

---

## 5️⃣ Normalising **to the OpenAI “multi‑tool”** contract (if you control the client)

If you can **upgrade the consumer** to the newer OpenAI spec (`tool_calls` array), you can keep the **full batch**:

| Provider | How to emit the OpenAI multi‑tool response |
|----------|--------------------------------------------|
| Gemini / Bedrock / Anthropic | After you have executed *all* calls, build a JSON response that contains `tool_calls` **array** with each element `{ "id": <upstream‑id>, "function": { "name": <name>, "arguments": <json‑string> } }`. The **follow‑up request** from the client should contain a `tool_results` array with matching IDs. |
| OpenAI (new spec) | Same shape; just forward the model’s own `tool_calls` array (no transformation needed). |
| Groq / Together / Fireworks | They never emit more than one call, so you can still return a single‑element `tool_calls` array – the client will treat it exactly like the legacy case. |
| OpenRouter | Translate based on the underlying provider as above. |

**Implementation tip** – keep a **mapping table** `{ provider_id → OpenAI‑compatible-id }` so you can translate `toolUseId` (Claude) or `toolCallId` (Gemini) into the `id` field OpenAI expects. The mapping is usually 1‑to‑1; just copy the string if you want to keep it opaque.

---

## 6️⃣ Summary of “What to do for each ecosystem”

| Ecosystem | Stream‑type | Tool‑call delivery | Normalisation approach |
|-----------|-------------|--------------------|------------------------|
| **OpenAI** (legacy) | Token‑by‑token SSE, stops at `function_call` delta | Single delta, complete JSON | Forward unchanged; close stream on `finish_reason="function_call"`. |
| **OpenAI** (new) | Same, but may emit multiple deltas in a `tool_calls` array | Whole array, single stream end | Forward unchanged; if you want single‑call façade, split the array. |
| **Groq** | Same as OpenAI legacy | Single delta, complete JSON | Forward unchanged. |
| **Vertex Gemini** | Incremental `text` parts; `functionCall` part appears as a **single complete block**; stream ends after that part. | List of calls (1‑4) – each call is a full block. | Buffer text, emit a single `function_call` (or whole array), close stream, start new request with `toolResponses`. |
| **AWS Bedrock** (Claude, Titan, etc.) | Incremental JSON lines; `toolUse` appears as a **single block**; stream ends (`stopReason="tool_use"`). | List of `toolUse` objects (any number). | Buffer text, emit `function_call` (or array), close, send second request with `toolResult(s)`. |
| **Anthropic Direct** | Same as Bedrock‑Claude. | Same. | Same. |
| **OpenRouter** | Mirrors whichever provider you routed to. | Mirrors whichever provider you routed to. | Detect the `x-openrouter-model` header and apply the corresponding rule. |
| **Together AI** | Same as OpenAI legacy. | Single `function_call` delta. | Forward unchanged. |
| **Fireworks AI** | Same as OpenAI legacy. | Single `function_call` delta. | Forward unchanged. |

---

## 7️⃣ Practical Code Skeleton (Python)

Below is a **minimal, production‑ready skeleton** that you can drop into a FastAPI/Flask endpoint. It hides the provider‑specific quirks and always returns an **OpenAI‑compatible SSE stream**.

```python
import json, asyncio
from fastapi import FastAPI, Request, Response

app = FastAPI()

# ----------------------------------------------------------------------
# 1️⃣ Provider‑specific streaming parsers (very lightweight)
# ----------------------------------------------------------------------
def parse_openai_sse(chunk: bytes) -> dict:
    return json.loads(chunk)

def parse_gemini_chunk(raw) -> dict:
    # Gemini returns protobuf; assume we have a helper that converts to dict
    return gemini_protobuf_to_dict(raw)

def parse_bedrock_line(line: str) -> dict:
    return json.loads(line)

# ----------------------------------------------------------------------
# 2️⃣ Normaliser – yields OpenAI‑style SSE lines
# ----------------------------------------------------------------------
async def normalise(provider: str, raw_iter):
    buffer = ""
    for raw in raw_iter:
        if provider in ("openai","groq","together","fireworks"):
            # already OpenAI‑style
            data = parse_openai_sse(raw)
            yield f"data: {json.dumps(data)}\n\n".encode()
            if data["choices"][0].get("finish_reason") in ("function_call","tool_calls"):
                break

        elif provider == "gemini":
            msg = parse_gemini_chunk(raw)
            for part in msg["candidates"][0]["content"]["parts"]:
                if "text" in part:
                    buffer += part["text"]
                    # emit each character as a token (optional: batch per 4‑5 chars)
                    for ch in part["text"]:
                        yield f'data: {json.dumps({"choices":[{"delta":{"content":ch}}]})}\n\n'.encode()
                else:  # functionCall part
                    # flush any pending text first
                    if buffer:
                        yield f'data: {json.dumps({"choices":[{"delta":{"content":buffer}}]})}\n\n'.encode()
                        buffer = ""
                    fc = {
                        "choices": [{
                            "delta": {
                                "function_call": {
                                    "name": part["functionCall"]["name"],
                                    "arguments": json.dumps(part["functionCall"]["args"])
                                }
                            }
                        }]
                    }
                    yield f'data: {json.dumps(fc)}\n\n'.encode()
                    # Gemini stops here – break out of both loops
                    return

        elif provider in ("bedrock","anthropic"):
            line = parse_bedrock_line(raw.decode())
            if line.get("stopReason") == "tool_use":
                # flush any buffered text
                if buffer:
                    yield f'data: {json.dumps({"choices":[{"delta":{"content":buffer}}]})}\n\n'.encode()
                    buffer = ""
                tool = line["toolUse"]
                fc = {
                    "choices": [{
                        "delta": {
                            "function_call": {
                                "name": tool["name"],
                                "arguments": json.dumps(tool["input"])
                            }
                        }
                    }]
                }
                yield f'data: {json.dumps(fc)}\n\n'.encode()
                return
            else:
                txt = line.get("outputText", "")
                buffer += txt
                # emit as‑is (you can split on whitespace if you prefer)
                yield f'data: {json.dumps({"choices":[{"delta":{"content":txt}}]})}\n\n'.encode()

        elif provider == "openrouter":
            # OpenRouter adds a header that tells you the underlying provider.
            underlying = request.headers.get("x-openrouter-model")
            # delegate to the right parser
            async for out in normalise(underlying, [raw]):
                yield out
            return

    # If we exit the loop without a tool call, close the stream
    yield b"data: [DONE]\n\n"

# ----------------------------------------------------------------------
# 3️⃣ Endpoint – proxy request to the chosen backend and stream back
# ----------------------------------------------------------------------
@app.post("/v1/chat/completions")
async def chat(request: Request):
    body = await request.json()
    provider = choose_provider(body)          # your routing logic
    # forward the request (sync or async) to the provider…
    raw_stream = await forward_to_provider(provider, body, stream=True)

    # turn the raw provider stream into OpenAI‑compatible SSE
    return Response(
        content=normalise(provider, raw_stream),
        media_type="text/event-stream"
    )
```

*What the code does*:

1. **Detects** the provider (OpenAI, Grok, Gemini, Bedrock, Anthropic, OpenRouter).
2. **Parses** the raw streaming chunks according to that provider’s wire format.
3. **Buffers** normal text and **flushes** it *before* emitting a `function_call` delta.
4. **Closes** the stream **immediately** after the function call, exactly how OpenAI expects it.
5. The **client** (any OpenAI SDK) sees a perfectly valid SSE stream and can continue with the normal `function_result` round‑trip.

---

## 8️⃣ TL;DR “One‑liner” for each ecosystem

| Ecosystem | Streaming → tool call pattern | Normalisation tip |
|-----------|------------------------------|-------------------|
| OpenAI (legacy) | Token‑by‑token → **single** `function_call` delta → stream ends. | Forward unchanged; close on `finish_reason="function_call"`. |
| OpenAI (new) | Same, but may emit an **array** `tool_calls`. | Forward unchanged; if you need legacy compatibility, split the array into separate turns. |
| Groq | Same as OpenAI legacy. | Forward unchanged. |
| Vertex Gemini | Text parts → **single** `functionCall` part (or list) → stream ends. | Buffer text, emit one `function_call` (or the whole array), close stream, start new request with `toolResponses`. |
| AWS Bedrock (Claude, Titan, Mistral…) | Text parts → **single** `toolUse` block → stream ends (`stopReason="tool_use"`). | Same as Gemini – treat the block as a complete function call, close, start a new request with `toolResult(s)`. |
| Anthropic Direct | Same as Bedrock‑Claude. | Same as Bedrock. |
| OpenRouter | Mirrors whichever provider you routed to. | Look at `x-openrouter-model` header and apply the matching rule. |
| Together AI | Same as OpenAI legacy (single `function_call`). | Forward unchanged. |
| Fireworks AI | Same as OpenAI legacy (single `function_call`). | Forward unchanged. |

Implement the **normaliser** once (the code skeleton above) and you’ll be able to serve **any** of the listed ecosystems to **any** OpenAI‑compatible caller without the caller ever noticing the underlying differences.