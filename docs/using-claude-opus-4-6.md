# Using Claude Opus 4.6 via the Wrapper

This guide shows how to use Claude Opus 4.6 (`claude-opus-4-6`) through the `claude-code-openai-wrapper`. The wrapper exposes an OpenAI-compatible API on `http://localhost:8000`, so any tool or library that speaks the OpenAI protocol can use Opus 4.6 without modification.

## Prerequisites

- The wrapper is running (via `systemctl` or `cargo run`)
- If `API_KEY` is set in the wrapper config, you need it for the `Authorization` header

Verify the wrapper is up and Opus 4.6 is listed:

```bash
curl http://localhost:8000/health
curl http://localhost:8000/v1/models
```

## Basic Request

### cURL

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Explain the CAP theorem with a practical example."}
    ]
  }'
```

If no `API_KEY` is configured in the wrapper, omit the `Authorization` header.

### Response

```json
{
  "id": "chatcmpl-a1b2c3d4",
  "object": "chat.completion",
  "created": 1710849600,
  "model": "claude-opus-4-6",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The CAP theorem states that..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 150,
    "total_tokens": 162
  }
}
```

## Using the OpenAI Python SDK

Point the `base_url` at the wrapper. No Anthropic SDK needed.

```bash
pip install openai
```

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="YOUR_API_KEY",  # or any string if no API_KEY is configured
)

response = client.chat.completions.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[
        {"role": "system", "content": "You are a senior software architect."},
        {"role": "user", "content": "How should I structure a microservices project?"},
    ],
)
print(response.choices[0].message.content)
```

## Using the OpenAI TypeScript SDK

```bash
npm install openai
```

```typescript
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://localhost:8000/v1",
  apiKey: "YOUR_API_KEY",
});

const response = await client.chat.completions.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [
    { role: "system", content: "You are a senior software architect." },
    { role: "user", content: "How should I structure a microservices project?" },
  ],
});
console.log(response.choices[0].message.content);
```

## Streaming

Set `"stream": true` to receive tokens as they are generated via Server-Sent Events (SSE).

### cURL

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 1024,
    "stream": true,
    "messages": [
      {"role": "user", "content": "Write a short story about a robot."}
    ]
  }'
```

Each SSE chunk follows the OpenAI format:

```
data: {"id":"chatcmpl-...","object":"chat.completion.chunk","choices":[{"delta":{"content":"Once"},"index":0,"finish_reason":null}]}

data: {"id":"chatcmpl-...","object":"chat.completion.chunk","choices":[{"delta":{"content":" upon"},"index":0,"finish_reason":null}]}

...

data: [DONE]
```

### Python

```python
stream = client.chat.completions.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    stream=True,
    messages=[
        {"role": "user", "content": "Write a short story about a robot."},
    ],
)
for chunk in stream:
    content = chunk.choices[0].delta.content
    if content:
        print(content, end="", flush=True)
print()
```

### TypeScript

```typescript
const stream = await client.chat.completions.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  stream: true,
  messages: [
    { role: "user", content: "Write a short story about a robot." },
  ],
});
for await (const chunk of stream) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) process.stdout.write(content);
}
console.log();
```

### Stream Usage Tracking

To receive token usage in the final stream chunk, add `stream_options`:

```json
{
  "model": "claude-opus-4-6",
  "stream": true,
  "stream_options": { "include_usage": true },
  "messages": [...]
}
```

## Extended Thinking

Claude can show its internal reasoning process before producing a final answer. To surface this, set `include_thinking` in the request body or via the `X-Claude-Include-Thinking` header.

### Enabling Thinking

```bash
# Via request body
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 16000,
    "include_thinking": true,
    "messages": [
      {"role": "user", "content": "Design a rate limiter supporting fixed-window and sliding-window. Compare the trade-offs."}
    ]
  }'

# Via header
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "X-Claude-Include-Thinking: true" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 16000,
    "messages": [
      {"role": "user", "content": "Design a rate limiter supporting fixed-window and sliding-window."}
    ]
  }'
```

The response includes a `thinking` field on the message object:

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "Here is a comparison of fixed-window vs sliding-window rate limiters...",
      "thinking": "The user wants me to design a rate limiter. Let me think about the two approaches..."
    }
  }]
}
```

### Python

```python
response = client.chat.completions.create(
    model="claude-opus-4-6",
    max_tokens=16000,
    messages=[
        {"role": "user", "content": "Design a rate limiter supporting fixed-window and sliding-window."},
    ],
    extra_body={"include_thinking": True},
)
print("=== Thinking ===")
print(response.choices[0].message.thinking)
print("=== Answer ===")
print(response.choices[0].message.content)
```

### Controlling Thinking Depth

Use `X-Claude-Max-Thinking-Tokens` to set the token budget for reasoning:

```python
response = client.chat.completions.create(
    model="claude-opus-4-6",
    max_tokens=16000,
    messages=[...],
    extra_body={"include_thinking": True},
    extra_headers={"X-Claude-Max-Thinking-Tokens": "10000"},
)
```

You can also set `MAX_THINKING_TOKENS` as an environment variable in the wrapper config for a global default.

### Streaming with Thinking

When streaming with thinking enabled, thinking arrives as a separate delta before content:

```
data: {"choices":[{"delta":{"thinking":"Let me reason about this..."}}]}
data: {"choices":[{"delta":{"content":"The answer is..."}}]}
```

### When to Use Thinking

- **Enable** for complex problems where you want to see the model's reasoning
- **Disable** (default) for simple Q&A, code generation, or when you only need the final answer
- Thinking is omitted entirely from the response when disabled — no performance cost

## Enabling Claude Code Tools

By default, tools are disabled for OpenAI compatibility. Enable them to let Opus 4.6 use Claude Code tools (Read, Write, Edit, Bash, Glob, Grep):

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 4096,
    "enable_tools": true,
    "messages": [
      {"role": "user", "content": "Read the file src/main.rs and explain what it does."}
    ]
  }'
```

### Controlling Which Tools Are Available

Use custom headers to fine-tune tool access:

```bash
# Only allow Read and Grep
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "X-Claude-Allowed-Tools: Read,Grep" \
  -d '{
    "model": "claude-opus-4-6",
    "enable_tools": true,
    "messages": [
      {"role": "user", "content": "Search for TODO comments in the codebase."}
    ]
  }'
```

```bash
# Allow all defaults except Bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "X-Claude-Disallowed-Tools: Bash" \
  -d '{
    "model": "claude-opus-4-6",
    "enable_tools": true,
    "messages": [...]
  }'
```

## Sessions (Multi-Turn Conversations)

The wrapper supports server-side session management. Pass a `session_id` to maintain conversation context across requests:

```python
# First message
r1 = client.chat.completions.create(
    model="claude-opus-4-6",
    messages=[{"role": "user", "content": "What is dependency injection?"}],
    extra_body={"session_id": "my-session-1"},
)
print(r1.choices[0].message.content)

# Follow-up in same session — context is preserved
r2 = client.chat.completions.create(
    model="claude-opus-4-6",
    messages=[{"role": "user", "content": "Show me an example in Python."}],
    extra_body={"session_id": "my-session-1"},
)
print(r2.choices[0].message.content)
```

Manage sessions via the REST API:

```bash
# List sessions
curl http://localhost:8000/v1/sessions

# Get session details
curl http://localhost:8000/v1/sessions/my-session-1

# Delete a session
curl -X DELETE http://localhost:8000/v1/sessions/my-session-1

# Session statistics
curl http://localhost:8000/v1/sessions/stats
```

Sessions expire automatically after 1 hour of inactivity.

## Model Override Header

Override the model per-request without changing the body, useful with clients that hardcode a model:

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -H "X-Claude-Model: claude-opus-4-6" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

The `X-Claude-Model` header takes precedence over the `model` field in the request body.

## Custom Headers Reference

| Header | Type | Description |
|--------|------|-------------|
| `Authorization` | `Bearer <key>` | API key (if configured) |
| `X-Claude-Model` | string | Override request model |
| `X-Claude-Effort` | string | Effort level: `low`, `medium`, `high`, `max` |
| `X-Claude-Max-Turns` | integer | Max agent turns (default: unlimited) |
| `X-Claude-Max-Budget-Usd` | float | Max cost in USD before stopping |
| `X-Claude-Fallback-Model` | string | Auto-switch model on overload |
| `X-Claude-Max-Thinking-Tokens` | integer | Extended thinking token budget |
| `X-Claude-Include-Thinking` | `true`/`false` | Return thinking in response |
| `X-Claude-Append-System-Prompt` | string | Append to system prompt |
| `X-Claude-Allowed-Tools` | comma-separated | Whitelist specific tools |
| `X-Claude-Disallowed-Tools` | comma-separated | Blacklist specific tools |
| `X-Claude-Permission-Mode` | string | `default`, `acceptEdits`, `bypassPermissions`, `plan` |

## Request Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `model` | string | `claude-sonnet-4-5-20250929` | Model to use |
| `messages` | array | required | Conversation messages |
| `max_tokens` | integer | - | Max output tokens |
| `temperature` | float | 1.0 | Sampling temperature (0.0-2.0) |
| `top_p` | float | 1.0 | Nucleus sampling (0.0-1.0) |
| `stream` | boolean | false | Enable streaming |
| `stop` | string/array | - | Stop sequences |
| `enable_tools` | boolean | false | Enable Claude Code tools |
| `include_thinking` | boolean | false | Return Claude's reasoning process |
| `json_schema` | object | - | JSON Schema for structured output |
| `response_format` | object | - | OpenAI-compatible format (`type: "json_schema"`) |
| `session_id` | string | - | Session for multi-turn context |
| `stream_options` | object | - | `{"include_usage": true}` for usage in stream |

## Available Models

| Model | Best For |
|-------|----------|
| `claude-opus-4-6` | Deepest reasoning, agents, complex coding |
| `claude-sonnet-4-6` | Best coding model, balanced performance |
| `claude-opus-4-5-20250929` | Premium intelligence + performance |
| `claude-sonnet-4-5-20250929` | Real-world agents and coding |
| `claude-haiku-4-5-20251001` | Fast responses, simple tasks |

## Troubleshooting

**401 Unauthorized** — Check your `Authorization: Bearer` header matches the `API_KEY` in `/etc/claude-wrapper/config.env`.

**429 Too Many Requests** — Rate limited. Default: 10 chat requests/minute. Adjust `RATE_LIMIT_CHAT_PER_MINUTE` in config.

**Model not found in /v1/models but still works** — The wrapper passes any model name through to Claude CLI. The `/v1/models` list is informational.

**Check logs:**

```bash
journalctl -u claude-code-openai-wrapper -f
```
