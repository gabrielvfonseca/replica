#!/usr/bin/env bash
# Unattended bench runner: wait for the machine to idle (1-min load < threshold
# for two consecutive samples), then run the project benchmark suite followed by
# the system benchmark. All output goes to benchmarks/results/bench-run.log.
# Self-terminates after MAXWAIT seconds.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
LOG="$HERE/results/bench-run.log"
mkdir -p "$HERE/results"

THR=2.5          # 1-min load must be under this (a quarter of 10 cores)
MAXWAIT=3600     # give up after 1 hour
IDLE=0           # consecutive sub-threshold samples needed
IDLE_MAX=3       # sub-threshold samples required before we call it idle
SAMPLE_S=20      # seconds between load samples

export PATH="$PWD/target/release:$PATH"
export NO_COLOR=1

say() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$LOG"; }

say "waiting for machine to idle (1-min load < $THR, $IDLE sub-threshold samples, every ${SAMPLE_S}s)"
deadline=$(( $(date +%s) + MAXWAIT ))
while :; do
  LOAD=$(sysctl -n vm.loadavg | awk '{print $2}')
  NOW=$(date +%s)
  if [ "$NOW" -gt "$deadline" ]; then
    say "gave up waiting; load still $(sysctl -n vm.loadavg | awk '{print $2}'). Not running benchmarks. Close other work and re-run."
    exit 2
  fi
  if awk -v l="$LOAD" -v t="$THR" 'BEGIN{exit !(l < t)}'; then
    IDLE=$((IDLE+1))
    say "  load=$LOAD (< $THR) [$IDLE/$IDLE_MAX]"
    if [ "$IDLE" -ge "${IDLE_MAX:-3}" ]; then say "machine idle; starting benchmarks"; break; fi
  else
    IDLE=0
    say "  load=$LOAD (>= $THR) -- waiting; top: $(ps -Ao pcpu,comm -r | sed -n '2p' | awk '{print $2" "$1"%"}')"
  fi
  sleep "$SAMPLE_S"
done

say "=== machine info ==="
sysctl -n machdep.cpu.brand_string hw.ncpu hw.memsize
sw_vers
say "load at start: $(sysctl -n vm.loadavg)"
echo

say "=== [1/2] PROJECT benchmark (benchmarks/run.sh, all methods, N=1/5/10) ==="
( cd "$HERE" && ./run.sh --concurrency 1,5,10 ) 2>&1 | tee -a "$LOG"
RC1=${PIPESTATUS[0]}
echo
say "project benchmark exit: $RC1"
echo

say "=== [2/2] SYSTEM benchmark (benchmarks/sysbench-host.sh) ==="
( cd "$HERE" && ./sysbench-host.sh ) 2>&1 | tee -a "$LOG"
RC2=${PIPESTATUS[0]}
echo
say "system benchmark exit: $RC2"
say "load at end: $(sysctl -n vm.loadavg)"
say "DONE (project=$RC1 system=$RC2)"
