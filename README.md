# Vestige — Cognitive Memory System for AI Agents

Vestige gives your AI coding assistants (Claude Code, Cursor, etc.) persistent long-term memory with neuroscience-inspired knowledge management. It remembers what you tell it across sessions, discovers connections between your memories, and naturally forgets what's no longer relevant.

**Built with:** Rust (backend) · SvelteKit (dashboard) · SQLite (storage) · FSRS-6 (spaced repetition) · Nomic Embed v1.5 (local embeddings)

## Why Vestige?

Every time you start a new AI coding session, you re-explain your architecture, preferences, and past decisions. Vestige fixes this:

- Tell Claude "remember that we use CDK not Terraform" → it remembers forever
- Ask "what did we decide about auth?" → it retrieves the decision with full context
- Switch projects → it loads only that project's memories
- Over time → important knowledge strengthens, irrelevant info fades naturally

No cloud dependency. Everything runs locally from a single binary.

## Quick Start

### Prerequisites

- macOS (Apple Silicon or Intel) or Linux
- [Rust](https://rustup.rs/) (for building from source)
- [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) v1.23+
- Node.js 20+ and pnpm (for the dashboard)

### Install

```bash
# Clone
git clone https://github.com/Sri01728/vestige.git
cd vestige

# Download ONNX Runtime (macOS ARM example)
curl -L "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-arm64-1.23.2.tgz" -o /tmp/onnxruntime.tgz
cd /tmp && tar -xzf onnxruntime.tgz
sudo mkdir -p /usr/local/lib/onnxruntime
sudo cp /tmp/onnxruntime-osx-arm64-1.23.2/lib/*.dylib /usr/local/lib/onnxruntime/

# Build
cd /path/to/vestige
ORT_LIB_LOCATION=/usr/local/lib/onnxruntime ORT_PREFER_DYNAMIC_LINK=1 cargo build --release

# Install binaries
sudo cp target/release/vestige-mcp /usr/local/bin/
sudo cp target/release/vestige /usr/local/bin/
```

### Connect to Claude Code

```bash
claude mcp add vestige /path/to/vestige-mcp-wrapper -s user
```

Create a wrapper script at `~/.local/bin/vestige-mcp-wrapper`:
```bash
#!/bin/bash
export DYLD_LIBRARY_PATH="/usr/local/lib/onnxruntime"
export FASTEMBED_CACHE_DIR="$HOME/.fastembed_cache"
export HF_HOME="$HOME/.fastembed_cache"
exec /usr/local/bin/vestige-mcp "$@"
```
```bash
chmod +x ~/.local/bin/vestige-mcp-wrapper
```

### Connect to Cursor

Add to `~/.cursor/mcp.json`:
```json
{
  "mcpServers": {
    "vestige": {
      "command": "/path/to/.local/bin/vestige-mcp-wrapper",
      "args": []
    }
  }
}
```

### Run as HTTP Server (recommended for multiple sessions)

Instead of each session spawning its own process, run one server:

```bash
vestige serve --dashboard --port 3928 --dashboard-port 3927
```

Then configure Claude Code / Cursor to connect via HTTP:
```bash
claude mcp add vestige --transport http http://localhost:3928/mcp -s user
```

### Dashboard

```bash
# Option 1: Built-in (basic)
vestige serve --dashboard

# Option 2: Dev dashboard (full features)
cd apps/dashboard
pnpm install
pnpm add 3d-force-graph chart.js graphology graphology-communities-louvain
pnpm dev
```

Open http://localhost:5173/dashboard

## Features

### Memory Ingestion
- **Prediction Error Gating** — Auto-decides create / update / supersede / merge based on semantic similarity to existing memories
- **Auto-tagging** — Detects tools (docker, aws, redis), domains (auth, devops, testing), languages (typescript, rust, python) from content
- **Entity extraction** — Identifies CamelCase types, snake_case identifiers, file paths
- **Workspace auto-tagging** — Active workspace automatically tags all new memories with `project:<name>`
- **Conflict detection** — When superseding, returns old content and reason for the change

### Memory Retrieval
- **Hybrid search** — BM25 keyword matching + semantic vector similarity
- **FSRS-6 decay** — 21-parameter spaced repetition. Important memories strengthen, irrelevant ones fade
- **Trigger tags** — Tag a memory with `trigger:deploy` and it auto-surfaces whenever deployment is discussed
- **Pinned core memories** — Always loaded at session start, never decay

### Knowledge Graph
- **Dream consolidation** — Discovers connections between memories via pairwise embedding comparison
- **Typed edges** — semantic, shared_concepts, caused_by, supersedes (not just "related")
- **0.65 similarity threshold** — Only genuinely related memories get connected
- **Multi-hop traversal** — Deep explore finds memories 1, 2, 3+ hops away

### Workspaces
- **Project isolation** — `workspace new my-project` creates an independent knowledge graph
- **Auto-tagging** — All ingested memories get `project:<name>` while workspace is active
- **Dashboard filtering** — View only one project's memories on the graph
- **Cross-project connections** — Dream still discovers genuine cross-project links

### Dashboard (7 pages)
- **Graph** — 3D force-directed visualization with click-to-reveal, community detection, particle flow, workspace filter, freeze/unpin, edge type legend
- **Memories** — List, search, filter all memories
- **Timeline** — GitHub-style heatmap calendar + chronological view
- **Explore** — Knowledge trail (step-by-step) + deep multi-hop traversal
- **Intentions** — Create/complete/snooze reminders with priority and triggers
- **Command Center** — Health score, charts (memory growth, retention distribution, workspace breakdown, connection types, activity), dream history
- **Settings** — Configuration

### Decay Tuning
- **Permanent** — Never decays (user preferences, critical facts)
- **Normal** — Standard FSRS-6 decay
- **Ephemeral** — 4x faster decay (temporary context)

### Active Forgetting
- **Suppress** — Compounding retrieval penalties, reversible within 24h
- **Demote** — Ranks memory lower in search without deleting
- **Dream filtering** — Noisy edge types (complementary, temporal) are never persisted

### CLI
```bash
vestige search "query"              # Search memories
vestige add "content" --tags "a,b"  # Quick add
vestige workspace new my-project    # Create workspace
vestige workspace set my-project    # Switch workspace
vestige workspace list              # Show all projects
vestige stats                       # Memory statistics
vestige health                      # System health check
vestige serve --dashboard           # Run HTTP server + dashboard
```

## MCP Tools (27)

| Tool | Purpose |
|------|---------|
| `smart_ingest` | Intelligent memory creation with prediction error gating |
| `search` | Hybrid BM25 + semantic search |
| `memory` | Get/delete/promote/demote/pin/unpin/set_decay/edit memories |
| `workspace` | Create/switch/list project workspaces |
| `session_context` | One-call session initialization (loads core + triggers + at-risk) |
| `dream` | Consolidation cycle — discover connections, strengthen memories |
| `intention` | Set/check/complete/snooze reminders |
| `deep_reference` | 8-stage cognitive reasoning across memories |
| `suppress` | Active forgetting with Rac1 cascade |
| `list_tags` | Browse all tags with counts |
| `export_claude_md` | Generate shareable CLAUDE.md from project memories |
| `memory_health` | Retention dashboard |
| `memory_graph` | Subgraph export for visualization |
| + 14 more | Changelog, timeline, consolidate, predict, explore, etc. |

## Architecture

```
┌─────────────────────────────────────────────────┐
│ Claude Code / Cursor / Any MCP Client           │
└──────────────────┬──────────────────────────────┘
                   │ MCP (stdio or HTTP)
┌──────────────────▼──────────────────────────────┐
│ vestige-mcp (Rust binary, ~7.6 MB)              │
│ ├── 27 MCP tools                                │
│ ├── Autopilot (14 autonomous cognitive modules) │
│ ├── Dashboard REST API + WebSocket              │
│ └── CLI (13 commands)                           │
├─────────────────────────────────────────────────┤
│ vestige-core (Rust library)                     │
│ ├── FSRS-6 (21-parameter spaced repetition)     │
│ ├── Hybrid search (BM25 + vector)               │
│ ├── Prediction Error Gating                     │
│ ├── Dream consolidation                         │
│ ├── Spreading activation                        │
│ ├── Synaptic tagging & capture                  │
│ ├── Active forgetting (Anderson 2025 + Rac1)    │
│ └── 29 neuroscience modules                     │
├─────────────────────────────────────────────────┤
│ SQLite + FTS5 + Nomic Embed v1.5 (local)        │
│ Database: ~/Library/Application Support/         │
│           com.vestige.core/vestige.db            │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│ SvelteKit Dashboard (7 pages)                   │
│ ├── 3d-force-graph (graph visualization)        │
│ ├── Chart.js (analytics)                        │
│ ├── Graphology (community detection)            │
│ └── Tailwind CSS                                │
└─────────────────────────────────────────────────┘
```

## Runtime Requirements

| Component | Size |
|-----------|------|
| vestige-mcp binary | 7.6 MB |
| vestige CLI binary | 7.4 MB |
| ONNX Runtime | 35 MB |
| Nomic Embed model (auto-downloaded) | 670 MB |
| SQLite database | ~1 MB (grows with usage) |
| **Total** | **~720 MB** |

## Credits

- Original Vestige by [samvallad33](https://github.com/samvallad33/vestige)
- Enhanced fork with custom features by [Sri01728](https://github.com/Sri01728)
- FSRS-6 algorithm by [open-spaced-repetition](https://github.com/open-spaced-repetition)
- Neuroscience foundations: Anderson 2025 (active forgetting), Bjork & Bjork 1992 (dual-strength model), Davis Rac1 (memory cascade)

## License

MIT
