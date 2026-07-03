#!/bin/bash
# with_lock.sh — serializes access to the agent wallet across concurrent SuperZK deploys.
# Usage: ./with_lock.sh <command> [args...]
# Uses an atomic mkdir-based lock (works even across genuinely concurrent processes
# in the same sandbox). Stale locks older than 5 minutes are auto-cleared so a crashed
# script never permanently deadlocks the pipeline.

LOCK_DIR="/app/kaspa_wrpc/.deploy.lock"
MAX_WAIT=280   # seconds — give up before the 300s bash timeout would hit anyway
STALE_AGE=300  # seconds — auto-clear a lock older than this (crashed holder)

waited=0
while true; do
  if mkdir "$LOCK_DIR" 2>/dev/null; then
    echo "$$" > "$LOCK_DIR/pid"
    date +%s > "$LOCK_DIR/acquired_at"
    break
  fi

  # Check for stale lock
  if [ -f "$LOCK_DIR/acquired_at" ]; then
    now=$(date +%s)
    acquired=$(cat "$LOCK_DIR/acquired_at" 2>/dev/null || echo 0)
    age=$((now - acquired))
    if [ "$age" -gt "$STALE_AGE" ]; then
      echo "Stale lock detected (${age}s old) — clearing it." >&2
      rm -rf "$LOCK_DIR"
      continue
    fi
  fi

  if [ "$waited" -ge "$MAX_WAIT" ]; then
    echo "RESULT_ERROR: Timed out waiting for wallet lock (another deploy is in progress)" 
    exit 1
  fi
  sleep 2
  waited=$((waited + 2))
done

# Release the lock no matter how the wrapped command exits
trap 'rm -rf "$LOCK_DIR"' EXIT

"$@"
exit $?
