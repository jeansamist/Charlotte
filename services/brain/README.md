# brain

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** Python · Strands Agents · Amazon Bedrock

## Role
The assistant's mind: it understands requests, keeps the conversation and memory, and acts through skills.

## Planned design
- **Agent:** Strands Agents on Amazon Bedrock models. The model is chosen per use case in settings.
- **Persona:** name, language and tone from the gateway settings (`assistant.name`, `assistant.language`).
- **Memory:** conversation history and long-term memory in PostgreSQL + pgvector.
- **Tools:** an MCP client that connects to every server in `skills/`.
- **Permissions:** what each user (household role) may ask the assistant to do.
- **Offline mode:** a local intent engine handles fixed phrases per language ("turn on the kitchen light",
  "who is home", "stop the music") without an LLM when the internet is down.

## Interfaces
- Consumes `voice.transcript` events (NATS) and publishes `brain.reply` (text for TTS).
- Calls skills over MCP.
- Reads settings and users from the gateway database (read-only role).

## Depends on
PostgreSQL + pgvector, NATS, AWS credentials (Bedrock), the `skills/*` MCP servers.
