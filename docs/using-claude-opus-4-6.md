# Using Claude Opus 4.6

Claude Opus 4.6 (`claude-opus-4-6`) is Anthropic's most intelligent model, optimized for building agents and coding. It offers the deepest reasoning capabilities in the Claude model family.

## Model ID

| Alias | Model ID |
|-------|----------|
| Latest | `claude-opus-4-6` |

Use `claude-opus-4-6` in API requests. This always points to the latest Opus 4.6 release.

## When to Use Opus 4.6

- Complex architectural decisions requiring deep reasoning
- Multi-step agent workflows with tool use
- Advanced coding tasks (refactoring, debugging, system design)
- Problems requiring extended thinking with large token budgets
- Research and analysis requiring maximum accuracy

For simpler tasks where speed matters more than depth, consider `claude-sonnet-4-5` or `claude-haiku-4-5`.

## Quick Start

### Prerequisites

1. An Anthropic API key from [console.anthropic.com](https://console.anthropic.com)
2. Set the key as an environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### cURL

```bash
curl https://api.anthropic.com/v1/messages \
  -H "content-type: application/json" \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Explain the CAP theorem with a practical example."}
    ]
  }'
```

### Python

Install the SDK:

```bash
pip install anthropic
```

Basic message:

```python
from anthropic import Anthropic

client = Anthropic()  # reads ANTHROPIC_API_KEY from env

message = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[
        {"role": "user", "content": "Hello, Claude"}
    ],
)
print(message.content[0].text)
```

### TypeScript

Install the SDK:

```bash
npm install @anthropic-ai/sdk
```

Basic message:

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic(); // reads ANTHROPIC_API_KEY from env

const message = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [
    { role: "user", content: "Hello, Claude" },
  ],
});
console.log(message.content[0].text);
```

## Streaming

Streaming returns tokens as they are generated, improving perceived latency for long responses.

### Python (async)

```python
import asyncio
from anthropic import AsyncAnthropic

client = AsyncAnthropic()

async def main():
    async with client.messages.stream(
        model="claude-opus-4-6",
        max_tokens=1024,
        messages=[{"role": "user", "content": "Write a short story about a robot."}],
    ) as stream:
        async for text in stream.text_stream:
            print(text, end="", flush=True)
        print()

    final_message = await stream.get_final_message()
    print(f"Output tokens: {final_message.usage.output_tokens}")

asyncio.run(main())
```

### TypeScript

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();

const stream = client.messages
  .stream({
    model: "claude-opus-4-6",
    max_tokens: 1024,
    messages: [
      { role: "user", content: "Write a short story about a robot." },
    ],
  })
  .on("text", (text) => {
    process.stdout.write(text);
  });

const message = await stream.finalMessage();
console.log(`\nOutput tokens: ${message.usage.output_tokens}`);
```

### cURL (SSE)

```bash
curl https://api.anthropic.com/v1/messages \
  -H "content-type: application/json" \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-opus-4-6",
    "max_tokens": 1024,
    "stream": true,
    "messages": [
      {"role": "user", "content": "Write a short story about a robot."}
    ]
  }'
```

## Extended Thinking

Extended thinking lets Opus 4.6 reason through complex problems step by step before producing a final answer. You allocate a `budget_tokens` for the thinking process; these tokens count toward `max_tokens`.

### Python

```python
from anthropic import Anthropic

client = Anthropic()

response = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=16000,
    thinking={
        "type": "enabled",
        "budget_tokens": 10000,
    },
    messages=[
        {
            "role": "user",
            "content": "Design a rate limiter that supports both fixed-window and sliding-window algorithms. Compare the trade-offs.",
        }
    ],
)

for block in response.content:
    if block.type == "thinking":
        print("=== Thinking ===")
        print(block.thinking)
        print()
    elif block.type == "text":
        print("=== Answer ===")
        print(block.text)
```

### TypeScript

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();

const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 16000,
  thinking: {
    type: "enabled",
    budget_tokens: 10000,
  },
  messages: [
    {
      role: "user",
      content: "Design a rate limiter that supports both fixed-window and sliding-window algorithms. Compare the trade-offs.",
    },
  ],
});

for (const block of response.content) {
  if (block.type === "thinking") {
    console.log("=== Thinking ===");
    console.log(block.thinking);
    console.log();
  } else if (block.type === "text") {
    console.log("=== Answer ===");
    console.log(block.text);
  }
}
```

**Notes on extended thinking:**
- `budget_tokens` must be at least 1,024
- `budget_tokens` must be less than `max_tokens`
- The thinking blocks appear before the text block in the response
- Higher budgets let the model reason more deeply but use more tokens

## Tool Use (Function Calling)

Opus 4.6 excels at deciding when and how to use tools, making it ideal for agent workflows.

### Python

```python
from anthropic import Anthropic

client = Anthropic()

tools = [
    {
        "name": "get_weather",
        "description": "Get the current weather for a given location.",
        "input_schema": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City and state, e.g. San Francisco, CA",
                }
            },
            "required": ["location"],
        },
    }
]

message = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    tools=tools,
    messages=[
        {"role": "user", "content": "What's the weather in San Francisco?"}
    ],
)

if message.stop_reason == "tool_use":
    tool_use = next(b for b in message.content if b.type == "tool_use")
    print(f"Tool: {tool_use.name}")
    print(f"Input: {tool_use.input}")
```

### TypeScript

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();

const message = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  tools: [
    {
      name: "get_weather",
      description: "Get the current weather for a given location.",
      input_schema: {
        type: "object" as const,
        properties: {
          location: {
            type: "string",
            description: "City and state, e.g. San Francisco, CA",
          },
        },
        required: ["location"],
      },
    },
  ],
  messages: [
    { role: "user", content: "What's the weather in San Francisco?" },
  ],
});

if (message.stop_reason === "tool_use") {
  const toolUse = message.content.find((b) => b.type === "tool_use");
  console.log(`Tool: ${toolUse.name}`);
  console.log(`Input: ${JSON.stringify(toolUse.input)}`);
}
```

## System Prompts

Guide Opus 4.6's behavior with a system prompt:

```python
message = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    system="You are a senior software architect. Be concise and provide code examples.",
    messages=[
        {"role": "user", "content": "How should I structure a microservices project?"}
    ],
)
```

## Multi-Turn Conversations

Pass the full conversation history in the `messages` array:

```python
messages = [
    {"role": "user", "content": "What is dependency injection?"},
    {"role": "assistant", "content": "Dependency injection (DI) is a design pattern..."},
    {"role": "user", "content": "Show me an example in Python."},
]

message = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=messages,
)
```

## Using via This Wrapper

The `claude-code-openai-wrapper` translates OpenAI-compatible requests to Claude. After starting the wrapper, send requests using the OpenAI format:

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-opus-4-6",
    "messages": [
      {"role": "user", "content": "Hello, Claude"}
    ]
  }'
```

Or with any OpenAI-compatible client library by pointing the base URL to the wrapper:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="your-wrapper-api-key",  # if API_KEY is set in wrapper config
)

response = client.chat.completions.create(
    model="claude-opus-4-6",
    messages=[
        {"role": "user", "content": "Hello, Claude"}
    ],
)
print(response.choices[0].message.content)
```

## Model Comparison

| Model | Best For | Speed | Depth |
|-------|----------|-------|-------|
| `claude-opus-4-6` | Agents, coding, deep reasoning | Slower | Deepest |
| `claude-sonnet-4-5` | General coding, real-world agents | Balanced | High |
| `claude-haiku-4-5` | Fast responses, simple tasks | Fastest | Moderate |

## Further Reading

- [Anthropic API Reference](https://docs.anthropic.com/en/api/messages)
- [Extended Thinking Guide](https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)
- [Tool Use Guide](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- [Prompt Engineering](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering)
