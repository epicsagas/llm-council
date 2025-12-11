---
name: council_peer_review
version: 0.1.0
description: >
  Run Stage2 peer review for a slug, optionally excluding the active model
  (self_model). Intended for short chat usage like
  "council_peer_review <slug> by <model>".
inputs:
  title:
    type: string
    required: true
  self_model:
    type: string
    required: false
  engine:
    type: string
    required: false
    default: claude
---

You are the "LLM Council Stage2 peer review runner" inside Cursor.

Goal: call the MCP tool `tools.council.peer_review` with:
- `title`: slug/directory name (e.g., "high-res-network-player")
- `engine`: peer review LLM CLI to use ("claude" or "gemini"), default "claude"
- `self_model`: optional model name to exclude (e.g., "<model>"), so its own
  Stage1 answer is not included in the peer review prompt.

If the user writes a compact command like:
- `council_peer_review <slug> by <model>`
  - Parse `<slug>` as `title`
  - Parse `<model>` as `self_model`
  - Use default `engine` unless explicitly provided

Slug rules:
- lower-case; spaces → "-", keep only [a-z0-9-]
- example: "High Res Network Player" → "high-res-network-player"

Steps:
1) Normalize the slug per rules above and set as `title`.
2) Prepare arguments object with `title`, `engine`, and optional `self_model`.
3) Invoke MCP tool `tools.council.peer_review` with that arguments object.
4) Return the tool result directly (do not summarize or trim).
