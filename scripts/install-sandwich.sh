#!/usr/bin/env bash
# install-sandwich.sh — One-command installer for the Vestige Cognitive Sandwich.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/samvallad33/vestige/v2.1.0/scripts/install-sandwich.sh | sh
#   # or, from a checkout:
#   ./scripts/install-sandwich.sh [--force] [--no-launchd] [--include-memory-loader]
#
# What it does:
#   1. Verifies required local tools
#   2. Stages ~/.claude/hooks/, ~/.claude/agents/, ~/Library/LaunchAgents/
#   3. Copies sanitized hooks + agents
#   4. Renders launchd plist template with $HOME and chosen MODEL
#   5. Merges hooks block into ~/.claude/settings.json (preserves existing keys)
#   6. launchctl load com.vestige.mlx-server (auto-starts mlx_lm.server with Qwen3.6-35B-A3B)
#   7. Prints next-steps for model download

set -euo pipefail

VERSION="${VESTIGE_SANDWICH_VERSION:-v2.1.0}"
REPO="samvallad33/vestige"
MODEL_ID="${VESTIGE_SANDWICH_MODEL:-mlx-community/Qwen3.6-35B-A3B-4bit}"
DASHBOARD_PORT="${VESTIGE_DASHBOARD_PORT:-3927}"
MLX_ENDPOINT="${MLX_ENDPOINT:-http://127.0.0.1:8080/v1/chat/completions}"
MLX_ENDPOINT="${MLX_ENDPOINT%/}"
MLX_MODELS_URL="${MLX_ENDPOINT%/chat/completions}/models"

HOOKS_DIR="$HOME/.claude/hooks"
AGENTS_DIR="$HOME/.claude/agents"
LAUNCHD_DIR="$HOME/Library/LaunchAgents"
SETTINGS="$HOME/.claude/settings.json"

FORCE=0
NO_LAUNCHD=0
INCLUDE_MEMORY_LOADER=0
SRC=""

for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    --no-launchd) NO_LAUNCHD=1 ;;
    --include-memory-loader) INCLUDE_MEMORY_LOADER=1 ;;
    --src=*) SRC="${arg#--src=}" ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
  esac
done

say()  { printf '\033[1;36m[sandwich]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[sandwich]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[sandwich]\033[0m %s\n' "$*" >&2; exit 1; }

# --- Platform check (honors --no-launchd for Linux/Intel users) ---
if [ "$(uname -s)" != "Darwin" ]; then
  if [ "$NO_LAUNCHD" -eq 0 ]; then
    die "macOS required for the launchd auto-start of mlx_lm.server. Re-run with --no-launchd to install hooks only and run mlx_lm.server manually."
  fi
  warn "Non-Darwin platform — installing hooks/agents only (no launchd). Run an OpenAI-compatible model endpoint and set MLX_ENDPOINT if it is not $MLX_ENDPOINT."
elif [ "$(uname -m)" != "arm64" ]; then
  warn "Apple Silicon recommended (M1+). Detected $(uname -m). The local Qwen3.6 model requires arm64 + Metal."
fi

# --- Prereqs (warnings only, install proceeds) ---
command -v jq      >/dev/null || die "jq required: brew install jq"
command -v python3 >/dev/null || die "python3 required (3.10+)"
command -v claude  >/dev/null || warn "'claude' CLI not found — install Claude Code first."
command -v vestige-mcp >/dev/null || warn "'vestige-mcp' not found — install with: cargo install vestige-mcp"
command -v uv      >/dev/null || warn "'uv' not found — install with: brew install uv"
command -v mlx_lm.server >/dev/null || warn "mlx-lm not installed — run: uv tool install mlx-lm"
command -v hf      >/dev/null || warn "'hf' not found — run: uv tool install 'huggingface_hub[cli]'"

# --- Resolve source: local checkout or release tarball ---
if [ -n "$SRC" ]; then
  SCRIPT_DIR="$SRC"
elif [ -f "$(dirname "$0")/../hooks/sanhedrin.sh" ]; then
  SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
  say "Using local checkout: $SCRIPT_DIR"
else
  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT
  say "Fetching Vestige Sandwich $VERSION..."
  curl -fsSL "https://github.com/$REPO/archive/refs/tags/$VERSION.tar.gz" \
    | tar xz -C "$TMPDIR"
  SCRIPT_DIR="$(ls -d "$TMPDIR"/vestige-*)"
fi

[ -d "$SCRIPT_DIR/hooks" ] || die "hooks/ not found in $SCRIPT_DIR — wrong source?"

# --- Stage directories ---
mkdir -p "$HOOKS_DIR" "$AGENTS_DIR" "$LAUNCHD_DIR"

# --- Copy hooks ---
copied=0; skipped=0
for f in "$SCRIPT_DIR/hooks"/*.sh "$SCRIPT_DIR/hooks"/*.py; do
  [ -f "$f" ] || continue
  base="$(basename "$f")"
  # load-all-memory.sh dumps every memory MD — opt-in only
  if [ "$base" = "load-all-memory.sh" ] && [ "$INCLUDE_MEMORY_LOADER" -eq 0 ]; then
    say "skip $base (use --include-memory-loader to install)"
    continue
  fi
  if [ -e "$HOOKS_DIR/$base" ] && [ "$FORCE" -eq 0 ]; then
    skipped=$((skipped + 1))
    continue
  fi
  install -m 0755 "$f" "$HOOKS_DIR/$base"
  copied=$((copied + 1))
done
say "hooks: $copied installed, $skipped skipped (use --force to overwrite)"

# --- Copy agents ---
for f in "$SCRIPT_DIR/agents"/*.md; do
  [ -f "$f" ] || continue
  base="$(basename "$f")"
  if [ -e "$AGENTS_DIR/$base" ] && [ "$FORCE" -eq 0 ]; then
    continue
  fi
  install -m 0644 "$f" "$AGENTS_DIR/$base"
done
say "agents installed to $AGENTS_DIR"

# --- Render launchd plist (macOS only) ---
if [ "$NO_LAUNCHD" -eq 0 ]; then
  PLIST="$LAUNCHD_DIR/com.vestige.mlx-server.plist"
  TEMPLATE="$SCRIPT_DIR/launchd/com.vestige.mlx-server.plist.template"
  [ -f "$TEMPLATE" ] || die "launchd template missing: $TEMPLATE"
  sed -e "s|__HOME__|$HOME|g" -e "s|__MODEL__|$MODEL_ID|g" "$TEMPLATE" > "$PLIST"
  launchctl unload "$PLIST" 2>/dev/null || true
  launchctl load "$PLIST"
  say "launchd loaded: com.vestige.mlx-server (model: $MODEL_ID)"
fi

# --- Merge hooks fragment into settings.json ---
[ -f "$SETTINGS" ] || echo '{}' > "$SETTINGS"
if [ -f "$HOME/.claude/settings.json.bak.pre-sandwich" ]; then
  say "settings.json backup already exists at .bak.pre-sandwich — not overwriting"
else
  cp "$SETTINGS" "$HOME/.claude/settings.json.bak.pre-sandwich"
fi
TMP_MERGE="$(mktemp)"
jq -s '.[0] * .[1]' "$SETTINGS" "$SCRIPT_DIR/hooks/settings.fragment.json" > "$TMP_MERGE"
mv "$TMP_MERGE" "$SETTINGS"
say "merged hooks block into $SETTINGS (backup at .bak.pre-sandwich)"

# --- Next steps ---
cat <<EOF

  ┌──────────────────────────────────────────────────────────────┐
  │  Cognitive Sandwich installed.                                │
  └──────────────────────────────────────────────────────────────┘

  Next steps:
    1. Download the local model (~19 GB, one-time):
         hf download $MODEL_ID
    2. Restart Claude Code so it picks up the new hooks.
    3. Verify the install:
         vestige health                 # if vestige CLI installed
         curl http://127.0.0.1:$DASHBOARD_PORT/api/health
         curl $MLX_MODELS_URL
    4. Try a prompt — the Sanhedrin Stop hook will fire and judge
       Claude's draft against your Vestige memory before delivery.

  To uninstall:
    launchctl unload $LAUNCHD_DIR/com.vestige.mlx-server.plist
    rm $LAUNCHD_DIR/com.vestige.mlx-server.plist
    cp $HOME/.claude/settings.json.bak.pre-sandwich $HOME/.claude/settings.json

EOF
