# ⚡ Quickstart: LLM Council Automation

This guide shows the fastest way to run the 3-step LLM Council workflow.

---

# 🎯 Goal

In about 2 minutes you will:

1. Generate Stage1 with multiple models  
2. Run Stage2 Peer Review via Rust MCP server  
3. Produce the Stage3 Final Answer

---

# 1️⃣ Stage1 — Collect multi-model answers

## Step 1: Write your question
Create `prompt.txt` in the Cursor workspace and add your question:

```
List the key technical considerations for a high-end network audio player.
```

## Step 2: Run Stage1

In Cursor Chat:

```
/council "High Res Network Player"
```

This command reads answers from:

- GPT-5  
- Claude-4.5  
- Gemini  

And saves them to:

```
.council/high-res-network-player/
```

Example outputs:

```
gpt-5-answer.md
claude-answer.md
gemini-answer.md
```

✨ Only one line of user input needed.

---

# 2️⃣ Stage2 — Peer Review (MCP)

Once Stage1 is ready, run Peer Review:

```
council_peer_review high-res-network-player by sonnet
```
(`by <model>` sets `self_model` to exclude that model’s own answer; default engine=sonnet [Claude Sonnet]. CLI engines available: claude, gemini, cursor-agent, codex.)

Cursor automatically calls the Rust MCP tool:

```
tools.council.peer_review
```

Result (human-facing):

```
peer-review-by-sonnet.md    # primary readable output
```

Cursor responses include `markdown` for direct rendering. Asking “show it in markdown” returns the same `markdown` content.

---

# 3️⃣ Stage3 — Final Answer (MCP)

```
council_finalize high-res-network-player engine=sonnet
```

Cursor automatically calls:

```
tools.council.finalize
```

Output (human-facing):

```
final-answer-by-sonnet.md    # primary readable output
```

Response includes `markdown` so the final answer renders nicely.

You get a single best-quality answer that incorporates all model opinions and reviews.

---

# 📁 Result file layout

```
.council/high-res-network-player/
  ├── gpt-5-answer.md
  ├── sonnet-answer.md
  ├── gemini-answer.md
  ├── peer-review-by-sonnet.md
  ├── peer-review-by-gemini.md
  ├── final-answer-by-sonnet.md
  └── final-answer-by-sonnet.md
```

---

# 🔧 Verify the MCP server is on PATH

```bash
which mcp-council
```
