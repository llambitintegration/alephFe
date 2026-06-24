#!/usr/bin/env bash
#
# ag-shutdown.sh — Stop the agentic-cli (ag) runtime for THIS project.
#
# Cleanly tears down the per-project local processes that ag-startup.sh brings
# up, in the reverse-of-startup / dependency-correct order:
#   1. Heartbeat watchdog    (FIRST — it respawns the daemon on a stale
#                             heartbeat; kill the daemon while the watchdog
#                             lives and you just race it back up)
#   2. Cron daemon           (graceful `ag cron daemon stop`, then SIGTERM/
#                             SIGKILL any surviving process — also frees the
#                             MetricsServer it owns)
#   3. Metrics port           (free any straggler still listening on it)
#
# Out of scope (host-level / shared — NOT touched here, matching ag-startup.sh):
#   - ag-supervisor daemon          (host-wide; manage via `ag host setup`)
#   - Monitoring backends in Docker  (ag-prometheus / ag-grafana / ag-loki / …)
#
# Idempotent: safe to run when nothing is up — it simply reports a clean slate.
# To stop-and-restart instead, use scripts/ag-restart.sh.
# Usage: bash scripts/ag-shutdown.sh
set -uo pipefail

export AG_SUPPRESS_BUN_LINK_BANNER=1

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

say()  { printf '\033[1;36m[ag-shutdown]\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  ⚠\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*"; }

say "Project: $PROJECT_ROOT"

# ---------------------------------------------------------------------------
# Resolve THIS project's metrics port from the registry (same logic as
# ag-startup.sh / ag-restart.sh) so we free the RIGHT port and never touch
# another tenant's. Falls back to scanning known cron processes if the
# registry lookup fails.
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
# 3. Verify clean slate.
# ---------------------------------------------------------------------------
say "Verifying shutdown…"
shutdown_clean=1
if [ -n "$METRICS_PORT" ]; then
  remaining="$(port_pids "$METRICS_PORT")"
  if [ -n "$remaining" ]; then
    err "Metrics port :$METRICS_PORT is STILL held (PIDs: $remaining)."
    err "Investigate manually (ss -ltnp | grep $METRICS_PORT) and re-run."
    shutdown_clean=0
  else
    ok "Metrics port :$METRICS_PORT is free"
  fi
fi
if pgrep -f 'cron (daemon|watchdog) start' >/dev/null 2>&1; then
  err "A cron daemon/watchdog process survived teardown:"
  pgrep -af 'cron (daemon|watchdog) start'
  shutdown_clean=0
fi

# ---------------------------------------------------------------------------
# 4. Summary
# ---------------------------------------------------------------------------
echo
if [ "$shutdown_clean" -eq 1 ]; then
  say "Shutdown complete — no daemon/watchdog processes remain."
  if [ -n "$METRICS_PORT" ]; then
    printf '    Metrics:       :%s  \033[1;32mfree\033[0m\n' "$METRICS_PORT"
  fi
  printf '    Cron daemon:   \033[1;33mstopped\033[0m\n'
  printf '    Watchdog:      \033[1;33mstopped\033[0m\n'
  say "Bring it back up with: bash scripts/ag-startup.sh"
  exit 0
else
  err "Shutdown INCOMPLETE — some processes or ports survived (see above)."
  err "Re-run this script, or escalate manually before restarting."
  exit 1
fi
