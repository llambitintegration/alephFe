#!/usr/bin/env bash
#
# ag-startup.sh — Bring up the agentic-cli (ag) runtime for THIS project.
#
# Scope: starts the per-project pieces that run as local processes:
#   1. MetricsServer        (Prometheus exporter on the project's metrics port)
#   2. Cron daemon          (executes scheduled workflows; emits heartbeat)
#   3. Heartbeat watchdog    (restarts the daemon if its heartbeat goes stale)
#
# Out of scope (host-level / shared — verified but NOT started here):
#   - ag-supervisor daemon          (host-wide; manage via `ag host setup`)
#   - Monitoring backends in Docker  (ag-prometheus / ag-grafana / ag-loki / ag-tempo)
#
# Idempotent: re-running will not double-start a component that is already up.
# Usage: bash scripts/ag-startup.sh
set -uo pipefail

export AG_SUPPRESS_BUN_LINK_BANNER=1

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

LOG_DIR="$PROJECT_ROOT/.agentic/logs"
mkdir -p "$LOG_DIR"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
DAEMON_LOG="$LOG_DIR/cron-daemon-$TS.log"
WATCHDOG_LOG="$LOG_DIR/watchdog-$TS.log"

say()  { printf '\033[1;36m[ag-startup]\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  ⚠\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*"; }

say "Project: $PROJECT_ROOT"

# ---------------------------------------------------------------------------
# TENANCY GUARD — fail closed before touching any service.
#
# `ag` resolves which tenant it acts on PURELY from the current directory's
# path, looked up against ~/.config/ag/projects.json. If this script ever runs
# from the wrong dir (or the registry is stale), `ag` silently falls back to
# the default tenant (port 9091 = agentic-cli/_default) and would start/modify
# the WRONG project. To keep multi-tenancy clean we assert, before starting
# anything, that ag resolves to THIS project's registered metrics port.
# ---------------------------------------------------------------------------
REGISTRY="${AG_PROJECTS_FILE:-$HOME/.config/ag/projects.json}"
# Emits "<metricsPort>\t<metricsPrefix>" for the registry entry whose path == CWD.
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
EXPECTED_PORT="${REG_LINE%%$'\t'*}"
EXPECTED_PREFIX="${REG_LINE#*$'\t'}"
if [ -z "$EXPECTED_PORT" ]; then
  err "TENANCY GUARD: '$PROJECT_ROOT' is not in the registry ($REGISTRY)."
  err "Register it with 'ag init' / 'ag host setup' before starting services. Aborting."
  exit 1
fi
RESOLVED_PORT="$(ag dashboard server status 2>/dev/null | sed -n 's/.*Port:[[:space:]]*\([0-9]\{1,\}\).*/\1/p' | head -1)"
if [ "$RESOLVED_PORT" != "$EXPECTED_PORT" ]; then
  err "TENANCY GUARD: ag resolved tenant port '$RESOLVED_PORT' but this project expects '$EXPECTED_PORT'."
  err "Refusing to start — services would act on the wrong tenant. Aborting."
  exit 1
fi
ok "Tenancy guard passed — ag is scoped to alephone-rust (metrics port $EXPECTED_PORT)"

# ---------------------------------------------------------------------------
# TENANT-SCOPE THE METRICS EXPORTER (must come AFTER the guard).
#
# The cron daemon embeds its own Prometheus exporter that reads METRICS_PORT /
# METRICS_PREFIX from the environment, falling back to the hardcoded defaults
# 9091 / "agentic_cli_" (src/observability/types.ts). Unset, the daemon squats
# on agentic-cli's port 9091 and mislabels THIS project's cron metrics as
# agentic_cli_* — a cross-tenant collision that makes a 2nd project's daemon
# fail with EADDRINUSE. Pin both to this project's registry values so the
# daemon owns the metrics server on its OWN port with its OWN prefix.
#
# Exported here (not earlier) so the guard above still validates pure
# CWD-based tenant resolution with a clean environment.
# ---------------------------------------------------------------------------
export METRICS_PORT="$EXPECTED_PORT"
export METRICS_PREFIX="${EXPECTED_PREFIX:-${PROJECT_ROOT##*/}_}"
ok "Metrics exporter scoped: port $METRICS_PORT, prefix '${METRICS_PREFIX}'"

# ---------------------------------------------------------------------------
# 0. Preconditions: supervisor (host-level) and monitoring backends (Docker)
# ---------------------------------------------------------------------------
say "Checking host-level prerequisites (not started by this script)…"
if pgrep -af 'ag-supervisor' >/dev/null 2>&1; then
  ok "ag-supervisor daemon is running"
else
  warn "ag-supervisor is NOT running — run 'ag host setup' to start it (cron scheduling needs it)"
fi

if command -v docker >/dev/null 2>&1; then
  up_backends="$(docker ps --format '{{.Names}}' 2>/dev/null | grep -E '^ag-(prometheus|grafana|loki|tempo|promtail)$' | sort | paste -sd' ' -)"
  if [ -n "$up_backends" ]; then
    ok "Monitoring backends up: $up_backends"
  else
    warn "No ag-* monitoring containers detected — Grafana/Loki dashboards may be unavailable"
  fi
fi

# ---------------------------------------------------------------------------
# 1. Cron daemon — also OWNS the project MetricsServer.
#
# Per the daemon design ("only the daemon should own MetricsServer"), we do NOT
# start a separate `ag dashboard server` here: with METRICS_PORT/METRICS_PREFIX
# exported above, the daemon's embedded exporter binds this project's port with
# this project's prefix. A standalone server would only collide on that port.
# ---------------------------------------------------------------------------
say "Starting cron daemon (owns MetricsServer on :$METRICS_PORT)…"
if ag cron daemon health >/dev/null 2>&1; then
  ok "Cron daemon already healthy"
else
  nohup ag cron daemon start >"$DAEMON_LOG" 2>&1 &
  # The daemon can take ~30s to report healthy: several MCP stdio bridges
  # crash-loop with backoff during init before the heartbeat goes fresh.
  # Poll health (exit-code based) for up to 45s rather than a fixed sleep.
  for _ in $(seq 1 45); do
    sleep 1
    ag cron daemon health >/dev/null 2>&1 && break
  done
  if ag cron daemon health >/dev/null 2>&1; then
    ok "Cron daemon started (log: ${DAEMON_LOG#$PROJECT_ROOT/})"
  else
    err "Cron daemon did not report healthy — tail: ${DAEMON_LOG#$PROJECT_ROOT/}"
  fi
fi

# ---------------------------------------------------------------------------
# 2. Verify the metrics exporter is tenant-scoped (right port, right prefix).
#    This is the multi-tenancy assertion: the daemon must serve on THIS
#    project's port with THIS project's prefix — never the 9091/agentic_cli_
#    default that would clobber another tenant.
# ---------------------------------------------------------------------------
say "Verifying tenant-scoped metrics on :$METRICS_PORT…"
metrics_sample="$(curl -s --max-time 5 "http://127.0.0.1:${METRICS_PORT}/metrics" 2>/dev/null)"
# Require the RICH daemon metrics (cron_daemon/boot_) under our prefix — not just
# any prefixed line. A leftover standalone server exposes only ${PREFIX}nodejs_*
# and would otherwise pass a naive prefix check while the daemon squats 9091.
if [ -z "$metrics_sample" ]; then
  warn "No response on :$METRICS_PORT/metrics yet (daemon exporter may still be warming up)"
elif printf '%s' "$metrics_sample" | grep -qE "^${METRICS_PREFIX}(cron_daemon|boot_)"; then
  ok "Daemon metrics serving '${METRICS_PREFIX}cron_*' on :$METRICS_PORT (tenant-isolated)"
elif printf '%s' "$metrics_sample" | grep -q "^${METRICS_PREFIX}"; then
  err "Only generic '${METRICS_PREFIX}nodejs_*' on :$METRICS_PORT — this is the standalone server, NOT the daemon."
  err "The daemon is likely still on default 9091. Tear down and re-run to rebind it (see below)."
else
  wrong="$(printf '%s' "$metrics_sample" | grep -oE '^[a-z_]+_' | sort -u | head -1)"
  err "Metrics on :$METRICS_PORT use prefix '${wrong}', expected '${METRICS_PREFIX}' — TENANT LEAK"
fi
# Cross-tenant collision guard: NO daemon should be serving cron metrics on the
# shared default 9091 (regardless of prefix — a squat shows as agentic_cli_* there).
if curl -s --max-time 3 "http://127.0.0.1:9091/metrics" 2>/dev/null | grep -qE 'cron_daemon|_boot_'; then
  err "A cron daemon is serving metrics on default port 9091 — cross-tenant squat detected."
  err "If that's THIS project's daemon, it ignored METRICS_PORT (started before this script). Tear down and re-run."
fi

# ---------------------------------------------------------------------------
# 3. Heartbeat watchdog (restarts daemon on stale heartbeat)
# ---------------------------------------------------------------------------
say "Starting heartbeat watchdog…"
# Detect liveness by reading the watchdog PID file directly, NOT by shelling out
# to `ag cron watchdog status`. Two reasons:
#   1. Each `ag` invocation is a Bun cold start (~2-3s + deprecation banner);
#      polling it 8x races against the watchdog's OWN cold start (the PID file
#      isn't written until ~7s after launch) and gives a false "not running".
#   2. `ag cron watchdog status` prints "NOT RUNNING" whose 'running' substring
#      fools a naive grep anyway.
# The watchdog writes .agentic/cron/watchdog.pid as JSON {"pid":N,...}; reading
# it + `kill -0` is instant and authoritative (see HeartbeatWatchdog.start()).
WATCHDOG_PIDFILE="$PROJECT_ROOT/.agentic/cron/watchdog.pid"
wd_running() {
  [ -f "$WATCHDOG_PIDFILE" ] || return 1
  local pid
  pid="$(grep -oE '"pid"[[:space:]]*:[[:space:]]*[0-9]+' "$WATCHDOG_PIDFILE" 2>/dev/null | grep -oE '[0-9]+$')"
  [ -n "$pid" ] || return 1
  kill -0 "$pid" 2>/dev/null && return 0
  return 1
}
if wd_running; then
  ok "Watchdog already running"
else
  # setsid + </dev/null: give the watchdog its own session and a clean stdin so
  # it can't be reaped when this script exits moments after backgrounding it.
  setsid nohup ag cron watchdog start </dev/null >"$WATCHDOG_LOG" 2>&1 &
  # The watchdog's `ag` cold start writes its PID file ~7s after launch. With
  # the cheap pidfile-based wd_running we can poll generously (20s) at no cost.
  for _ in $(seq 1 20); do
    sleep 1
    wd_running && break
  done
  if wd_running; then
    ok "Watchdog started (log: ${WATCHDOG_LOG#$PROJECT_ROOT/})"
  else
    warn "Watchdog status unconfirmed — tail: ${WATCHDOG_LOG#$PROJECT_ROOT/}"
  fi
fi

# ---------------------------------------------------------------------------
# 4. Summary
# ---------------------------------------------------------------------------
say "Startup complete. Health summary:"
# NOTE: prefer heartbeat-based 'daemon health' over PID-file 'daemon status',
# which can report "Stopped" for a live daemon when the pid file is empty/stale.
if ag cron daemon health >/dev/null 2>&1; then
  printf '    Cron daemon:   \033[1;32mhealthy\033[0m (heartbeat)\n'
else
  printf '    Cron daemon:   \033[1;31munhealthy\033[0m\n'
fi
if wd_running; then
  printf '    Watchdog:      \033[1;32mrunning\033[0m\n'
else
  printf '    Watchdog:      \033[1;33mnot running\033[0m\n'
fi
printf '    Metrics:       :%s  (prefix %s)\n' "$METRICS_PORT" "$METRICS_PREFIX"
say "Live dashboard:  ag dashboard         (or: ag dashboard status)"
say "Cron jobs:        ag cron list"
