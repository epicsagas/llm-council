# 🔮 LLM Council Automation System

**Multi-Model Reasoning + Peer Review + Final Synthesis (Rust MCP + Cursor IDE)**  
End-to-end automated LLM Council pipeline

---

## 📌 Overview

LLM Council is a 3-step method to generate high-quality answers:

1. **Stage1 — First Opinions**  
   Collect independent answers from multiple models (GPT-5, Claude, Gemini, etc.)
2. **Stage2 — Peer Review**  
   Local LLM (Claude/Gemini CLI) compares and ranks the answers
3. **Stage3 — Final Answer**  
   A “Chairman” LLM synthesizes reviews into one expert-level answer

This repo delivers a fully automated workflow using **Cursor IDE Multi-Model + Rust MCP server + local LLM CLI**.

---

## 📁 Repository Structure

```
/
├── mcp-council/             # Rust MCP server (Stage2 + Stage3)
│   ├── src/
│   ├── Cargo.toml
│   └── QUICKSTART.md            # MCP server usage/installation doc
│
.council/                  # All results are stored here
└── {slug}/
    ├── gpt-5-answer.md
    ├── sonnet-answer.md
    ├── gemini-answer.md
    ├── peer-review-by-sonnet.md
    ├── peer-review-by-gemini.md
    └── final-answer-by-sonnet.md

commands/
├── council.md       # Cursor command for Stage1 automation
├── peer_review.md   # Cursor command for Stage2 automation
└── finalize.md      # Cursor command for Stage3 automation
```

---

## 🧠 System Architecture

```
[Cursor IDE]
  ├─ User prompt + title
  ├─ Multi-Model run (GPT-5 / Claude / Gemini)
  └─ Command (council.md) → Stage1 JSONs
                     ▼
[Rust MCP Server (mcp-council)]
  ├─ council.peer_review → Stage2 JSON
  └─ council.finalize    → Stage3 JSON
                     ▼
[Local LLM CLI]
  ├─ claude-cli
  └─ gemini-cli
```

---

## 🚀 Quick Start

For a fast, user-focused walkthrough (Stage1→3), see **Quickstart.md**.

---

## 🛠 Components

### 1) **Stage1: Cursor collection + storage**  
`.cursor/commands/council.md` parses 3 multi-model outputs and saves them automatically (markdown).

Example outputs:  
```
.council/high-res-network-player/gpt-5-answer.md
.council/high-res-network-player/claude-answer.md
.council/high-res-network-player/gemini-answer.md
```

---

### 2) **Stage2: Peer Review (Rust MCP Server)**

Rust MCP Tool:  
```
tools.council.peer_review
```

Input (engine is user-provided; examples: sonnet/gemini/gpt/grok):
```json
{ "title": "high-res", "engine": "sonnet" }  // Claude Sonnet
```

Output:
```
peer-review-by-sonnet.md   # human-friendly
markdown (string field in the response object)
```

---

### 3) **Stage3: Final Answer (Rust MCP Server)**

Rust MCP Tool:
```
tools.council.finalize
```

Input (engine is user-provided; examples: sonnet/gemini/gpt/grok):
```json
{ "title": "high-res", "engine": "sonnet" }  // Claude Sonnet
```

Output:
```
final-answer-by-sonnet.md         # primary human-readable output
markdown (string field in the response object for Cursor UI)
```

---

## 🔧 Installation

### 1) Build Rust MCP Server
```bash
cd mcp-council
cargo build --release
cp target/release/mcp-council ~/.local/bin/
```

### 2) Register MCP in Cursor  
`~/.cursor/mcp.json`:

```json
{
  "servers": {
    "llm-council": {
      "command": "mcp-council",
      "args": []
    }
  }
}
```

### 3) Install Cursor command file

Global (home):
```bash
mkdir -p ~/.cursor/commands
cp mcp-council/commands/* ~/.cursor/commands/
```

Per-project:
```bash
mkdir -p .cursor/commands
cp mcp-council/commands/* .cursor/commands/
```

---

## ✨ Example End-to-End Usage

```
/council "High Res Network Player"
```

→ Stage1 save  
→ Cursor context now has 3 JSON answers  

Stage2:

Compact chat alias:
```
council_peer_review high-res-network-player by claude
```
(slug → title, `by <model>` → self_model exclusion; default engine=sonnet [Claude Sonnet])

The response includes `markdown` and writes (engine passed through):
```
.council/<slug>/peer-review-by-<engine>.md   # e.g., peer-review-by-sonnet.md
```

Stage3:

Compact chat alias:
```
council_finalize high-res-network-player engine=sonnet
```

The response includes `markdown` and writes:
```
.council/<slug>/final-answer-by-<engine>.md   # e.g., final-answer-by-sonnet.md
```

CLI engines supported (actual binaries): `claude` (Claude CLI, e.g., sonnet), `gemini`, `cursor-agent`, `codex`. The `engine` value is passed through to the CLI runner; choose the token that matches your installed CLI.

Done 🎉

---

## 📜 License
MIT License

---

## 🙏 Acknowledgements
- Model Context Protocol (MCP)
- Anthropic Claude CLI
- Google Gemini CLI
- Cursor IDE Multi-Model
# 🔮 LLM Council Automation System

**Multi-Model Reasoning + Peer Review + Final Synthesis (Rust MCP + Cursor IDE)**  
Fully automated LLM Council pipeline

---

## 📌 Overview

LLM Council is a 3-step method to produce high-quality answers:

1. **Stage1 — First Opinions**  
   Collect independent answers from multiple models (GPT-5, Sonnet, Gemini, etc.)
2. **Stage2 — Peer Review**  
   Local LLM (Sonnet/Gemini CLI) compares and evaluates the answers
3. **Stage3 — Final Answer**  
   Chairman LLM synthesizes all reviews into the final expert answer

This repository uses **Cursor IDE multi-model + Rust MCP server + local LLM CLI** to fully automate LLM Council.

---

## 📁 Repository Structure

```
/
├── mcp-council/             # Rust MCP server (Stage2 + Stage3)
│   ├── src/
│   ├── Cargo.toml
│   └── QUICKSTART.md            # MCP server usage/installation doc
│
├── .council/                # All results are stored here
│   └── {slug}/
│       ├── gpt-5-answer.md
│       ├── sonnet-answer.md
│       ├── gemini-answer.md
│       ├── peer-review-by-sonnet.md
│       └── final-answer-by-sonnet.md
│
└── .cursor/
    └── commands/
        └── council.md       # Cursor command for Stage1 automation
```

---

## 🧠 System Architecture

```
[Cursor IDE]
  ├─ User prompt + title
  ├─ Multi-Model run (GPT-5 / Sonnet / Gemini)
  └─ Command (council.md) → Stage1 JSONs
                     ▼
[Rust MCP Server (mcp-council)]
  ├─ council.peer_review → Stage2 JSON
  └─ council.finalize    → Stage3 JSON
                     ▼
[Local LLM CLI]
  ├─ sonnet (via Claude CLI)
  └─ gemini-cli
```

---

## 🚀 Quick Start

For a quick walkthrough, see **Quickstart.md** (Stage1~3 end-to-end).

---

## 🛠 Components

### 1) **Stage1: Cursor capture + save**  
`.cursor/commands/council.md` parses and saves three multi-model outputs automatically (markdown).

Example outputs:  
```
.council/high-res-network-player/gpt-5-answer.md
.council/high-res-network-player/claude-answer.md
.council/high-res-network-player/gemini-answer.md
```

---

### 2) **Stage2: Peer Review (Rust MCP Server)**

Rust MCP Tool:  
```
tools.council.peer_review
```

Input:
```json
{ "title": "high-res", "engine": "sonnet" }
```

Output:
```
peer-review-by-sonnet.md
markdown (string field in the response object)
```

---

### 3) **Stage3: Final Answer (Rust MCP Server)**

Rust MCP Tool:
```
tools.council.finalize
```

Input:
```json
{ "title": "high-res", "engine": "sonnet" }
```

Output:
```
final-answer-by-sonnet.md
markdown (string field in the response object)
```

---

## 🔧 Installation

### 1) Build Rust MCP Server
```bash
cd mcp-council
cargo build --release
cp target/release/mcp-council ~/.local/bin/
chmod +x ~/.local/bin/mcp-council
```

### 2) Register MCP in Cursor  
`~/.cursor/mcp.json`:

```json
{
  "servers": {
    "llm-council": {
      "command": "mcp-council",
      "args": []
    }
  }
}
```

> For details, see `QUICKSTART.md` :contentReference[oaicite:0]{index=0}

---

## ✨ Example End-to-End Usage

```
/council "High Res Network Player"
```

→ Stage1 save  
→ Cursor context now has 3 MD files  

Stage2:

```
council_peer_review high-res-network-player by sonnet
```

The response object includes a `markdown` field for UI-friendly rendering.

Stage3:

```
council_finalize high-res-network-player engine=sonnet
```

Likewise, the response includes `markdown` so you can read the final answer directly.

Done 🎉

---

## 📜 License
MIT License

---

## 🙏 Acknowledgements
- Model Context Protocol (MCP)
- Anthropic Claude CLI
- Google Gemini CLI
- Cursor IDE Multi-Model