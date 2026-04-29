#!/usr/bin/env bash
# check-sandwich-prereqs.sh — Verify host can run the Vestige Cognitive Sandwich.
set -u

ok()   { printf '  \033[1;32m[ OK ]\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m[WARN]\033[0m %s\n' "$*"; FAIL=1; }
miss() { printf '  \033[1;31m[MISS]\033[0m %s\n' "$*"; FAIL=1; }

FAIL=0
DASHBOARD_PORT="${VESTIGE_DASHBOARD_PORT:-3927}"
MLX_ENDPOINT="${MLX_ENDPOINT:-http://127.0.0.1:8080/v1/chat/completions}"
MLX_ENDPOINT="${MLX_ENDPOINT%/}"
MLX_MODELS_URL="${MLX_ENDPOINT%/chat/completions}/models"

echo "Vestige Cognitive Sandwich — Prereq Check"
echo

# Platform
if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then
  ok "Apple Silicon macOS ($(sw_vers -productVersion 2>/dev/null || echo darwin))"
else
  miss "Apple Silicon Mac required (M1+). Detected $(uname -s) $(uname -m)."
fi

# Python
if command -v python3 >/dev/null; then
  PY="$(python3 -c 'import sys;print(".".join(map(str,sys.version_info[:2])))' 2>/dev/null)"
  case "$PY" in
    3.1[0-9]|3.[2-9]*) ok "Python $PY" ;;
    *) warn "Python $PY (need 3.10+)" ;;
  esac
else
  miss "python3 not found"
fi

# CLI tools
command -v jq            >/dev/null && ok "jq"            || miss "jq missing — brew install jq"
command -v uv            >/dev/null && ok "uv"            || miss "uv missing — brew install uv"
command -v mlx_lm.server >/dev/null && ok "mlx-lm"        || miss "mlx-lm — uv tool install mlx-lm"
command -v hf            >/dev/null && ok "huggingface_hub CLI" || miss "hf — uv tool install 'huggingface_hub[cli]'"
command -v claude        >/dev/null && ok "claude CLI"    || miss "claude CLI — install Claude Code"
command -v vestige-mcp   >/dev/null && ok "vestige-mcp"   || miss "vestige-mcp — cargo install vestige-mcp"

# Model on disk — HF cache uses `models--<org>--<name>` (double-dash separators).
MODEL="${VESTIGE_SANDWICH_MODEL:-mlx-community/Qwen3.6-35B-A3B-4bit}"
HF_HOME_DEFAULT="${HF_HOME:-$HOME/.cache/huggingface}"
ENC_MODEL="models--$(printf '%s' "$MODEL" | sed 's|/|--|g')"
if [ -d "$HF_HOME_DEFAULT/hub/$ENC_MODEL" ]; then
  ok "Model cached: $MODEL"
else
  printf '  \033[1;33m[INFO]\033[0m Model not yet downloaded — first run will fetch ~19GB\n'
  printf '         hf download %s\n' "$MODEL"
  # NOT a failure — first-run download is expected.
fi

# Vestige MCP HTTP API
if curl -fsS -m 2 "http://127.0.0.1:${DASHBOARD_PORT}/api/health" >/dev/null 2>&1; then
  ok "vestige-mcp dashboard responding on :$DASHBOARD_PORT"
else
  warn "vestige-mcp dashboard not responding on :$DASHBOARD_PORT"
fi

# OpenAI-compatible local/remote model endpoint
if curl -fsS -m 2 "$MLX_MODELS_URL" >/dev/null 2>&1; then
  ok "model endpoint responding at $MLX_MODELS_URL"
else
  warn "model endpoint not responding at $MLX_MODELS_URL — install + load launchd plist or set MLX_ENDPOINT"
fi

# launchd plist
if [ -f "$HOME/Library/LaunchAgents/com.vestige.mlx-server.plist" ]; then
  ok "launchd plist installed"
else
  warn "launchd plist missing — run: install-sandwich.sh"
fi

# Settings hook wiring
if [ -f "$HOME/.claude/settings.json" ] && \
   jq -e '.hooks.UserPromptSubmit and .hooks.Stop' "$HOME/.claude/settings.json" >/dev/null 2>&1; then
  ok "settings.json hooks block present"
else
  warn "settings.json missing hooks block — run: install-sandwich.sh"
fi

echo
if [ $FAIL -eq 0 ]; then
  echo "  Ready. Cognitive Sandwich will fire on next Claude Code prompt."
  exit 0
else
  echo "  Fix the items above, then re-run."
  exit 1
fi
