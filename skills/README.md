# skills

> **Status: planned, not implemented.**

Every capability of the assistant is a **skill**, and every skill is an **MCP server** (Model Context Protocol).
The `brain` is the MCP client: it lists each skill's tools and calls them when a request needs them.
To add a capability, add a skill. Nothing else changes.

| Skill | Purpose |
|---|---|
| [`home-assistant`](home-assistant/README.md) | Control Home Assistant entities and scenes |
| [`audio`](audio/README.md) | Control the audio-server (volume, output, pairing) |
| [`camera`](camera/README.md) | Snapshots and clips |
| [`presence`](presence/README.md) | Who is home |
| [`_template`](_template/README.md) | Starting point for custom skills |

Planned conventions:
- One folder per skill, in any language (MCP SDKs exist for Python, TypeScript and Rust).
- A manifest declares the name, description, tools and required permissions (which roles may use it).
- Skills are registered in the gateway (enable or disable from the GUI) and supervised like the other services.
