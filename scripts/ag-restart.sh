#!/usr/bin/env bash
#
# ag-restart.sh — HARD restart of the agentic-cli (ag) runtime for THIS project.
#
# ag-startup.sh is idempotent: it no-ops any component already running, so it can
# never recover a daemon stuck in a corrupted half-state (e.g. live process but
# removed/stale pidfile, or a watchdog that has lost track of its daemon). This
# script tears everything down in the correct order, frees the metrics port, then
# delegates the bring-up to ag-startup.sh.
#
# Teardown ORDER MATTERS:
#   1. Watchdog FIRST — it respawns the daemon on a stale heartbeat. Kill the
#      daemon while the watchdog lives and you just race it back up.
#   2. Daemon SECOND — graceful `ag cron daemon stop`, then SIGTERM/SIGKILL any
#      surviving `cron daemon start` process, then free the metrics port.
#   3. Verify the metrics port is actually free before handing off to startup.
#
# Usage: bash scripts/ag-restart.sh
set -uo pipefail

export AG_SUPPRESS_BUN_LINK_BANNER=1

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

say()  { printf '\033[1;36m[ag-restart]\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  ⚠\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*"; }

say "Project: $PROJECT_ROOT"

# ---------------------------------------------------------------------------
# Resolve THIS project's metrics port from the registry (same logic as
# ag-startup.sh) so we free the RIGHT port and never touch another tenant's.
# Falls back to scanning known cron processes if the registry lookup fails.
# ---------------------------------------------------------------------------
REGISTRY="${AG_PROJECTS_FILE:-$HOME/.config/ag/projects.json}"
REG_LINE="$(
  PR="$PROJECT_ROOT" python3 - "$REGISTRY" <<'PY' 2>/dev/null
import json, os, sys
reg = sys.argv[1]
root = os.environ["PR"]
try:
    entries = json.load(open(reg))
except Exception:
    sys.exit(0)
for e in entries:
    if os.path.realpath(e.get("path", "")) == os.path.realpath(root):
        print("%s\t%s" % (e.get("metricsPort", ""), e.get("metricsPrefix", ""))); break
PY
)"
METRICS_PORT="${REG_LINE%%$'\t'*}"
if [ -n "$METRICS_PORT" ]; then
  ok "Resolved metrics port for this tenant: :$METRICS_PORT"
else
  warn "Could not resolve metrics port from registry — will skip port-free verification"
fi

# pid(s) currently LISTENING on a given TCP port (space-separated, may be empty).
port_pids() {
  local p="$1"
  { ss -ltnpH "sport = :$p" 2>/dev/null || true; } \
    | grep -oE 'pid=[0-9]+' | cut -d= -f2 | sort -u | paste -sd' ' -
}

# Graceful-then-forceful kill of a set of PIDs.
kill_pids() {
  local what="$1"; shift
  local pids=("$@")
  [ "${#pids[@]}" -eq 0 ] && return 0
  say "Stopping $what (PIDs: ${pids[*]})…"
  kill -TERM "${pids[@]}" 2>/dev/null || true
  for _ in $(seq 1 10); do
    sleep 1
    local alive=()
    for pid in "${pids[@]}"; do kill -0 "$pid" 2>/dev/null && alive+=("$pid"); done
    [ "${#alive[@]}" -eq 0 ] && { ok "$what stopped"; return 0; }
    pids=("${alive[@]}")
  done
  warn "$what did not exit on SIGTERM — escalating to SIGKILL (${pids[*]})"
  kill -KILL "${pids[@]}" 2>/dev/null || true
  sleep 1
}

# ---------------------------------------------------------------------------
# 1. WATCHDOG — stop first so it cannot respawn the daemon mid-teardown.
# ---------------------------------------------------------------------------
say "Tearing down watchdog…"
ag cron watchdog stop >/dev/null 2>&1 || true
mapfile -t wd_pids < <(pgrep -f 'cron watchdog start')
kill_pids "watchdog" "${wd_pids[@]}"
rm -f "$PROJECT_ROOT/.agentic/cron/watchdog.pid" 2>/dev/null || true

# ---------------------------------------------------------------------------
# 2. DAEMON — graceful CLI stop, then kill any surviving process, then free
#    the metrics port (handles the "live process / removed pidfile" desync we
#    have seen: `ag cron daemon stop` can't find a daemon whose pidfile the
#    status command already deleted).
# ---------------------------------------------------------------------------
say "Tearing down cron daemon…"
ag cron daemon stop >/dev/null 2>&1 || true
mapfile -t dm_pids < <(pgrep -f 'cron daemon start')
kill_pids "cron daemon" "${dm_pids[@]}"

if [ -n "$METRICS_PORT" ]; then
  mapfile -t leftover < <(port_pids "$METRICS_PORT" | tr ' ' '\n' | grep -E '.')
  if [ "${#leftover[@]}" -gt 0 ]; then
    warn "Metrics port :$METRICS_PORT still held after daemon stop — freeing it"
    kill_pids "metrics-port :$METRICS_PORT holder" "${leftover[@]}"
  fi
fi

# ---------------------------------------------------------------------------
# 3. Verify clean slate before bring-up.
# ---------------------------------------------------------------------------
if [ -n "$METRICS_PORT" ]; then
  remaining="$(port_pids "$METRICS_PORT")"
  if [ -n "$remaining" ]; then
    err "Metrics port :$METRICS_PORT is STILL held (PIDs: $remaining) — aborting before bring-up."
    err "Investigate manually (ss -ltnp | grep $METRICS_PORT) and re-run."
    exit 1
  fi
  ok "Metrics port :$METRICS_PORT is free"
fi
if pgrep -f 'cron (daemon|watchdog) start' >/dev/null 2>&1; then
  err "A cron daemon/watchdog process survived teardown — aborting before bring-up."
  pgrep -af 'cron (daemon|watchdog) start'
  exit 1
fi
ok "Teardown complete — no daemon/watchdog processes remain"

# ---------------------------------------------------------------------------
# 4. Bring everything back up via the canonical idempotent startup script.
# ---------------------------------------------------------------------------
say "Re-running bring-up (scripts/ag-startup.sh)…"
echo
exec bash "$PROJECT_ROOT/scripts/ag-startup.sh"
