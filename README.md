# PKS — Prometheus Knowledge System

Persistent memory and hybrid search for AI agents. PKS indexes your Git repositories, logs commits automatically, and exposes a `search_knowledge_vault` tool via MCP — so your LLM always has context without re-reading files every session.

---

## Overview

PKS runs as a local MCP server (stdio). Once configured, your AI agent can search across all your projects using a single tool call — with results in milliseconds.

### Key Capabilities & MCP Tools

Your AI agent gains a "long-term memory" of all your projects. PKS exposes several tools via MCP:

*   **`search_knowledge_vault`** — Hybrid search (BM25 + optional Semantic) across all indexed repositories in milliseconds.
*   **`pks_add_decision`** — Record architecture decisions (ADRs) directly into the knowledge vault branch.
*   **`pks_add_feature`** — Capture feature requirements and specs for future context retrieval.
*   **`list_knowledge_vaults`** — Discover which repos are currently being managed by PKS.
*   **`pks_execute`** — Execute terminal commands in the context of the indexed projects.

**What it does:**

- **Hybrid search** — BM25 full-text (always on) + optional vector search via Ollama
- **Session journal** — captures tool events (PostToolUse/Stop hooks) and git commits (via `hook-post-commit` + `flush-session`) into daily markdown logs on the `pks-knowledge` branch.
- **Multi-project** — indexes all repos under a root directory simultaneously
- **Offline-capable** — no cloud dependency; everything runs locally

**How it fits in your workflow:**

```
Your IDE / AI Agent
       │
       │  MCP (stdio)
       ▼
  pks --stdio
       │
       ├── search_knowledge_vault("auth architecture")
       │         └── returns ranked markdown chunks from all indexed repos
       │
       ├── pks_add_decision("We are using Rust for the parser")
       │         └── saves ADR to pks-knowledge branch
       │
       └── pks_execute("git log --oneline -10")
                 └── runs commands in vault context
```

PKS indexes all `.md` files it finds under `PKS_VAULTS_DIR`. Each Git repo it finds gets a `pks-knowledge` branch where indexed snapshots and git journal logs are stored — isolated from your main branch history.


---

## Quick Start with your AI Agent

The fastest way to install and configure PKS is to let your AI agent do it.
Copy the prompt below and send it to Claude Code, Antigravity, Cursor, or any MCP-capable agent:

```
Read the README at https://github.com/sbihaiko/pks, follow the
"LLM Agent — Installation Instructions" section, and:

1. Install PKS on my machine (clone, build, cargo install)
2. Add the MCP stdio server config to my project's .mcp.json,
   pointing PKS_VAULTS_DIR to my projects root directory
3. Run `pks init` in the current repo
4. Run a test search to confirm everything is working

Report each step's result before moving to the next.
```

The agent will read this file, execute the steps in the
[LLM Agent — Installation Instructions](#-llm-agent--installation-instructions) section,
and report back at each step.

---

## Requirements

- [Rust (stable)](https://rustup.rs)
- Git
- [Ollama](https://ollama.com) *(optional — for vector/semantic search)*

---

## Install

### 1. Clone and build

```bash
git clone https://github.com/sbihaiko/pks.git
cd pks
cargo build --release
```

The binary will be at `target/release/pks`. Optionally install globally:

```bash
cargo install --path .
```

### 2. Configure your IDE

Add PKS to your `.mcp.json` as a stdio server:

```json
{
  "mcpServers": {
    "pks": {
      "type": "stdio",
      "command": "/path/to/pks/target/release/pks",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "/path/to/your/projects"
      }
    }
  }
}
```

`PKS_VAULTS_DIR` is the root directory PKS scans for Git repositories.

<details open>
<summary><strong>Claude Code</strong></summary>

Add to `.mcp.json` at your project root:

```json
{
  "mcpServers": {
    "pks": {
      "type": "stdio",
      "command": "/Users/<you>/.cargo/bin/pks",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "/Users/<you>/Projects"
      }
    }
  }
}
```

Restart Claude Code. The `search_knowledge_vault` and `pks_execute` tools will appear automatically.

</details>

<details>
<summary><strong>Antigravity</strong></summary>

Add to `.mcp.json` at your project root:

```json
{
  "mcpServers": {
    "pks": {
      "type": "stdio",
      "command": "/Users/<you>/.cargo/bin/pks",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "/Users/<you>/Projects",
        "PKS_EMBEDDING_PROVIDER": "none"
      }
    }
  }
}
```

</details>

<details>
<summary><strong>Cursor / VS Code (Copilot)</strong></summary>

Add to `.vscode/mcp.json`:

```json
{
  "servers": {
    "pks": {
      "type": "stdio",
      "command": "/Users/<you>/.cargo/bin/pks",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "/Users/<you>/Projects"
      }
    }
  }
}
```

</details>

<details>
<summary><strong>Linux / systemd or macOS / launchd (persistent daemon)</strong></summary>

See [`deploy/README.md`](deploy/README.md) for service installation that survives reboots.

</details>

### 3. Initialize a repository

Inside any Git repo you want indexed:

```bash
pks init
```

Expected output:

```
✓ Git root detected: /path/to/your/repo
✓ Config generated: /path/to/your/repo/.pks/config.toml
✓ Branch created: pks-knowledge (com estrutura Obsidian)
✓ Worktree: prometheus/ → pks-knowledge
✓ Daemon registered: your-project (RepoId: /path/to/.git)
PKS active. Search: pks search "<your query>"
```

This creates a `prometheus/` directory as a git worktree linked to the `pks-knowledge` branch. It comes pre-configured with an Obsidian vault structure (`features/`, `decisions/`, `journals/`) and `.obsidian/` settings — open it directly in [Obsidian](https://obsidian.md) to browse your project's knowledge base alongside the code.

To enable semantic search (optional):

```bash
export PKS_EMBEDDING_PROVIDER=ollama
ollama pull nomic-embed-text
```

---

## MCP Tools

| Tool | Description |
|------|-------------|
| `search_knowledge_vault` | Hybrid BM25 + vector search across all indexed repos. Returns ranked markdown chunks. |
| `pks_execute` | Runs a shell command in the context of the active vault. |
| `list_knowledge_vaults` | List all registered Git repository vaults known to PKS. |
| `pks_add_decision` | Record an architecture decision (ADR) in the project's knowledge vault. Writes to decisions/ on pks-knowledge branch. |
| `pks_add_feature` | Record a feature specification in the project's knowledge vault. Writes to features/ on pks-knowledge branch. |

---

## CLI Reference

```bash
pks init [path]                          # Initialize PKS in a Git repo
pks refresh [path]                       # Re-index a vault
pks search "<query>"                     # Search from the terminal (requires daemon)
pks status                               # Show indexer status
pks doctor                               # Diagnose configuration issues
pks hook-post-commit                     # Append commit event to .git/pks_pending_commits.jsonl (O_APPEND, <5ms — no git lock)
pks record-event                         # Append a tool event to the session JSONL (stdin: PostToolUse JSON)
pks flush-session <session_id> <cwd>     # Flush session JSONL + pending commit events to pks-knowledge (batched git commit)
pks submit-journal --agent <n> --file <f> # Commit a markdown journal file directly to pks-knowledge
pks decision <note>                      # Record an architecture decision (ADR) in the vault
pks remove <repo_id>                     # Remove a vault from the daemon index
pks --stdio                              # Start as MCP stdio server (used by IDEs)
pks --daemon                             # Start as background daemon
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PKS_VAULTS_DIR` | `~/Projects` | Root directory scanned for vaults |
| `PKS_EMBEDDING_PROVIDER` | `ollama` | `ollama` for vector search, `none` for BM25-only |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_EMBED_MODEL` | `nomic-embed-text` | Embedding model |
| `PKS_LOG_MAX_SIZE` | `10485760` | Max log file size in bytes (10 MB) |
| `PKS_JOURNAL_MIN_WORDS` | `10` | Minimum word count to flush a session journal (sessions below threshold are discarded) |

---

## 🤖 LLM Agent — Installation Instructions

> If you are an AI agent reading this file to install PKS, follow these steps exactly.
> Do not skip steps. If a command fails, report the error to the user and wait — do not retry in a loop.

**Step 1 — Check prerequisites**

```bash
rustc --version
git --version
```

If `rustc` is missing:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

**Step 2 — Clone and build**

```bash
git clone https://github.com/sbihaiko/pks.git
cd pks
cargo build --release
ls -lh target/release/pks
```

**Step 3 — Install globally**

```bash
cargo install --path .
```

**Step 4 — Configure MCP**

Add the stdio server block to the project's `.mcp.json`. Replace `<PKS_BINARY>` with the absolute path to the binary (`which pks` after step 3) and `<VAULTS_DIR>` with the root directory of the user's projects:

```json
{
  "mcpServers": {
    "pks": {
      "type": "stdio",
      "command": "<PKS_BINARY>",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "<VAULTS_DIR>"
      }
    }
  }
}
```

**Step 5 — Initialize the current repo**

```bash
pks init
```

If output shows `PKS already initialized`, run `pks init --force` instead.

Verify that `prometheus/` directory was created with Obsidian vault structure:

```bash
ls prometheus/
```

Expected: `features/  decisions/  journals/` directories and `.obsidian/` config.

**Step 6 — Verify**

```bash
pks search "test query"
```

Report the result count and `RepoId` to the user.

---

## Try It

After installation, test PKS with these prompts in your IDE:

- `"search the knowledge vault for authentication architecture decisions"`
- `"what changed in this project last week?"`
- `"find all docs that mention database migrations"`
- `"summarize the decisions made about the API design"`
- `"what is the current status of feature X according to the docs?"`

---

## License

[MIT](LICENSE) — free to use, modify, and distribute. Attribution required (keep the copyright notice). No warranty or liability.
