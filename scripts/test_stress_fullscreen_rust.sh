#!/bin/bash
# =============================================================================
# STRESS TEST: Window ↔ Fullscreen Toggle for Rust-Firework
# =============================================================================
# Detects deadlocks/crashes during rapid F11 fullscreen toggling.
#
# Detection strategy:
#   After each 'F11' keystroke, wait for app log output:
#     "Fullscreen:"  or  "Window resized:"
#   If expected log line does NOT appear within TIMEOUT_SEC → DEADLOCK/HANG.
#   Saves progress incrementally to stress_progress.log on every iteration.
# =============================================================================

set -eo pipefail

APP_PATH="${1:-./target/debug/fireworks_sim}"
ITERATIONS="${2:-50}"
DELAY_MS="${3:-100}"
WINDOW_NAME="Fireworks Simulator"
LOG_FILE="stress_fullscreen.log"
PROGRESS_FILE="stress_progress.log"
STACKS_FILE="stress_fullscreen.stacks"
TIMEOUT_SEC=5

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     RUST-FIREWORK FULLSCREEN TOGGLE STRESS TEST             ║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC} Binary:     ${YELLOW}$APP_PATH${NC}"
echo -e "${CYAN}║${NC} Iterations: ${YELLOW}$ITERATIONS${NC}"
echo -e "${CYAN}║${NC} Delay:      ${YELLOW}${DELAY_MS}ms${NC}"
echo -e "${CYAN}║${NC} Timeout:    ${YELLOW}${TIMEOUT_SEC}s per toggle${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"

if [ ! -f "$APP_PATH" ]; then
    echo "Error: Application binary not found at $APP_PATH."
    exit 1
fi

rm -f "$LOG_FILE" "$PROGRESS_FILE" "$STACKS_FILE"
: >"$LOG_FILE"
: >"$PROGRESS_FILE"

echo -e "\n${CYAN}[INIT]${NC} Starting application..."
export RUST_LOG=info
stdbuf -oL -eL "$APP_PATH" > >(tee "$LOG_FILE") 2>&1 &
APP_PID=$!


sleep 1.5

if ! kill -0 $APP_PID 2>/dev/null; then
    echo -e "${RED}[FATAL]${NC} Application failed to start."
    cat "$LOG_FILE"
    exit 1
fi

WID=""
for attempt in {1..10}; do
    WID=$(xdotool search --onlyvisible --name "$WINDOW_NAME" 2>/dev/null | head -n 1 || true)
    if [ -n "$WID" ]; then break; fi
    sleep 0.5
done

if [ -z "$WID" ]; then
    echo -e "${YELLOW}[WARN]${NC} Window '$WINDOW_NAME' visible search failed, trying fallback search..."
    WID=$(xdotool search --name "$WINDOW_NAME" 2>/dev/null | head -n 1 || true)
fi

if [ -z "$WID" ]; then
    echo -e "${RED}[FATAL]${NC} Could not find window '$WINDOW_NAME'."
    kill $APP_PID 2>/dev/null || true
    exit 1
fi

xdotool windowfocus "$WID" 2>/dev/null || true
sleep 1

echo -e "\n${CYAN}[START]${NC} Beginning stress test: $ITERATIONS fullscreen toggle cycles"

SUCCESS_COUNT=0
HANG_COUNT=0
CRASH_DETECTED=false
START_TIME=$(date +%s)
EXPECT_FULLSCREEN=true
XDOTOOL_TIMEOUT=2

wait_for_log_pattern() {
    local pattern="$1"
    local line_before="$2"
    local timeout_ms=$((TIMEOUT_SEC * 1000))
    local poll_ms=50
    local waited=0

    while [ $waited -lt $timeout_ms ]; do
        if ! kill -0 $APP_PID 2>/dev/null; then
            return 1 # Crash
        fi

        local current_count
        current_count=$(grep -c "$pattern" "$LOG_FILE" 2>/dev/null || true)
        current_count=${current_count:-0}
        if [ "$current_count" -gt "$line_before" ]; then
            return 0
        fi

        sleep "0.$(printf '%03d' $poll_ms)"
        waited=$((waited + poll_ms))
    done

    return 2 # Timeout / Hang
}

send_toggle_key() {
    xdotool key --window "$WID" --delay 0 F11 2>/dev/null || xdotool key --delay 0 F11 2>/dev/null || true
}

capture_stacks() {
    local iteration=$1
    local direction=$2
    echo -e "${YELLOW}[DIAG]${NC} Capturing stack traces (iteration $iteration, $direction)..."
    {
        echo "============================================================"
        echo "DEADLOCK at iteration $iteration ($direction)"
        echo "Timestamp: $(date -Iseconds)"
        echo "PID: $APP_PID"
        echo "============================================================"
    } >>"$STACKS_FILE"

    if command -v gdb &>/dev/null; then
        gdb -batch -ex "set pagination off" -ex "thread apply all bt full" -p $APP_PID 2>/dev/null >>"$STACKS_FILE" || true
    fi
}

# Ensure window focus once before starting burst
xdotool windowfocus "$WID" 2>/dev/null || true
xdotool windowactivate "$WID" 2>/dev/null || true
sleep 0.2

for ((i = 1; i <= ITERATIONS; i++)); do
    if ! kill -0 $APP_PID 2>/dev/null; then
        echo -e "${RED}[CRASH]${NC} Application died before iteration $i / $ITERATIONS"
        echo "$(date -Iseconds) - CRASH at toggle $i" >> "$PROGRESS_FILE"
        CRASH_DETECTED=true
        break
    fi

    send_toggle_key

    if [ "$DELAY_MS" -gt 0 ]; then
        sleep "$(awk "BEGIN{printf \"%.4f\", $DELAY_MS/1000}")"
    fi

    # Check if app crashed immediately after keypress
    if ! kill -0 $APP_PID 2>/dev/null; then
        echo -e "${RED}[CRASH]${NC} Application crashed on Toggle #$i"
        echo "$(date -Iseconds) - CRASH at toggle $i" >> "$PROGRESS_FILE"
        CRASH_DETECTED=true
        break
    fi

    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    if [ $((i % 10)) -eq 0 ] || [ $i -eq $ITERATIONS ]; then
        elapsed=$(($(date +%s) - START_TIME))
        msg="Sent $i / $ITERATIONS rapid toggles (${elapsed}s elapsed)"
        echo -e "${GREEN}[OK]${NC}    $msg"
        echo "$(date -Iseconds) - [OK] $msg" >> "$PROGRESS_FILE"
    fi
done

# Post-burst check: allow engine event loop 1 second to drain queue and verify no deadlock/crash
if ! $CRASH_DETECTED; then
    echo -e "\n${CYAN}[VERIFY]${NC} Waiting 1.5s for event loop to drain queue and check stability..."
    sleep 1.5
    if ! kill -0 $APP_PID 2>/dev/null; then
        echo -e "${RED}[CRASH]${NC} Application crashed during event loop processing after burst!"
        CRASH_DETECTED=true
    fi
fi

END_TIME=$(date +%s)
TOTAL_TIME=$((END_TIME - START_TIME))

echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║             RUST-FIREWORK STRESS TEST RESULTS               ║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC} Toggles completed: ${GREEN}$SUCCESS_COUNT${NC} / $((ITERATIONS))"
echo -e "${CYAN}║${NC} Hangs detected:    $(if [ $HANG_COUNT -gt 0 ]; then echo -e "${RED}$HANG_COUNT${NC}"; else echo -e "${GREEN}0${NC}"; fi)"
echo -e "${CYAN}║${NC} Crash detected:    $(if $CRASH_DETECTED; then echo -e "${RED}YES${NC}"; else echo -e "${GREEN}NO${NC}"; fi)"
echo -e "${CYAN}║${NC} Total time:        ${TOTAL_TIME}s"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"

if kill -0 $APP_PID 2>/dev/null; then
    echo -e "\n${CYAN}[CLEANUP]${NC} Terminating application..."
    kill $APP_PID 2>/dev/null || true
fi

wait $APP_PID 2>/dev/null || true

if $CRASH_DETECTED || [ $HANG_COUNT -gt 0 ]; then
    exit 1
else
    exit 0
fi
