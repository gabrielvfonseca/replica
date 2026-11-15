#!/usr/bin/env bash
# System (host) benchmark: disk throughput (seq/clonefile/latency), CPU (single
# and all-core), and memory bandwidth.
#
# This is NOT a Replica benchmark. It measures the host machine itself so that
# the project benchmark in run.sh can be read against the hardware it ran on.
# Output: benchmarks/results/<date>-<arch>-macos<version>-sys.{md,json}.
#
# Usage:
#   benchmarks/sysbench-host.sh            # defaults
#   benchmarks/sysbench-host.sh --disk-bytes 256 --allow-busy
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="$HERE/results"
DISK_MB=256        # sequential write/read test size
CLONE_MB=256       # clonefile test size (single file)
RUNS=5             # repetitions per op
WARMUP=1
ALLOW_BUSY=0
TMP_MB=256

usage() {
  sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
  cat <<'EOF'

Options:
  --disk-bytes MB   sequential read/write test size (default 256)
  --clone-bytes MB  clonefile test file size (default 256)
  --runs N          repetitions per operation (default 5)
  --out DIR         results directory (default benchmarks/results)
  --allow-busy      run even if the 1-minute load average is high
  -h, --help        show this help
EOF
}
while [ $# -gt 0 ]; do
  case "$1" in
    --disk-bytes) DISK_MB="$2"; shift 2 ;;
    --clone-bytes) CLONE_MB="$2"; shift 2 ;;
    --runs) RUNS="$2"; shift 2 ;;
    --out) OUT_DIR="$2"; shift 2 ;;
    --allow-busy) ALLOW_BUSY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[ "$(uname -s)" = "Darwin" ] || { echo "needs macOS" >&2; exit 1; }

NCPU=$(sysctl -n hw.ncpu)
LOAD_START=$(sysctl -n vm.loadavg | awk '{print $2}')
if [ "$ALLOW_BUSY" -ne 1 ] && awk -v l="$LOAD_START" -v c="$NCPU" 'BEGIN { exit !(l > c / 4) }'; then
  echo "machine is busy: 1-minute load $LOAD_START on $NCPU cores. Close other work and retry, or pass --allow-busy." >&2
  ps -Ao pcpu,comm -r | sed -n '2,6p' >&2
  exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/replica-sys.XXXXXX")"
touch "$SCRATCH/.metadata_never_index"
cleanup() { [ "${KEEP:-0}" = "1" ] || rm -rf "$SCRATCH"; }
trap cleanup EXIT

export NO_COLOR=1
WORK="$SCRATCH/work"; mkdir -p "$WORK"

log() { echo "== $*" >&2; }

# time a command; prints elapsed seconds (float)
t() {
  local s e
  s=$(python3 -c 'import time;print(time.time())')
  "$@" >/dev/null 2>&1
  e=$(python3 -c 'import time;print(time.time())')
  python3 -c "print('%.6f' % ($e - $s))"
}

# ---- disk: sequential write (dd), sequential read (dd), 4k latency (dd), clonefile ----
log "disk: ${DISK_MB}MB sequential write + read, clonefile, 4k latency"
DISK_FILE="$WORK/disk.bin"
CLONE_SRC="$WORK/clone-src.bin"

write_s=""; read_s=""; clone_s=""; lat_s=""
# warm
for _ in $(seq 1 "$WARMUP"); do
  dd if=/dev/zero of="$DISK_FILE" bs=1m count="$DISK_MB" 2>/dev/null
  dd if="$DISK_FILE" of=/dev/null bs=1m 2>/dev/null
done
for i in $(seq 1 "$RUNS"); do
  ws=$(t dd if=/dev/zero of="$DISK_FILE" bs=1m count="$DISK_MB")
  rs=$(t dd if="$DISK_FILE" of=/dev/null bs=1m)
  # 4k random-ish latency: write many small blocks
  lat=$(t dd if=/dev/zero of="$WORK/lat.bin" bs=4k count=2048)
  # clonefile a large file
  cp -c "$DISK_FILE" "$WORK/clone-$i.bin" 2>/dev/null
  cs=$(t cp -c "$DISK_FILE" "$WORK/clone-$i.bin")
  rm -f "$WORK/clone-$i.bin"
  echo "$ws $rs $lat $cs" >>"$WORK/disk.runs"
done
# aggregate: take best (min) for write/read, and for each of write/read/clone/lat
DISK_WRITE=$(awk '{print $1}' "$WORK/disk.runs" | sort -n | head -1)
DISK_READ=$(awk '{print $2}' "$WORK/disk.runs" | sort -n | head -1)
DISK_LAT=$(awk '{print $3}' "$WORK/disk.runs" | sort -n | head -1)
DISK_CLONE=$(awk '{print $4}' "$WORK/disk.runs" | sort -n | head -1)

# ---- CPU: single-thread and all-core FPU/digest loop via openssl/sha256 ----
log "cpu: single-core and all-core sha256 throughput"
CPUFILE="$WORK/cpu.bin"
dd if=/dev/zero of="$CPUFILE" bs=1m count=64 2>/dev/null
# single-thread time for N MB
CPU_N=512
s_single=$(t openssl dgst -sha256 "$CPUFILE" >/dev/null)
# all-core: N parallel digests (one per core), total N*64MB
s_multi=$( { for _ in $(seq 1 "$NCPU"); do openssl dgst -sha256 "$CPUFILE" >/dev/null & done; wait; } )
# We measure a fixed total: single does 64MB in s_single. All-core does 64MB*NCPU in s_multi.
CPU_SINGLE_MBPS=$(python3 -c "print(%.1f % (64 / float('$s_single')))" )
CPU_MULTI_MBPS=$(python3 -c "print(%.1f % (64 * $NCPU / float('$s_multi')))" )

# ---- memory: memcpy bandwidth via a quick python (single core) and parallel ----
log "memory: bandwidth via python memcpy"
MEM_MB=1024
MEM_S=$(python3 - "$MEM_MB" <<'PY'
import sys, time
n = int(sys.argv[1]) * 1024 * 1024
a = bytearray(n)
b = bytearray(n)
t0 = time.time()
# a few passes to be measurable
for _ in range(3):
    b[:] = a
dt = time.time() - t0
# bytes moved = 2 copies (read+write) * n * 3 passes
print('%.6f' % (dt / 3))
PY
)
MEM_MBPS=$(python3 -c "print('%.0f' % (2 * $MEM_MB / float('$MEM_S')))" )

# ---- gather ----
MACHINE="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)"
LOAD_END=$(sysctl -n vm.loadavg | awk '{print $2}')
MACOS="$(sw_vers -productVersion)"; BUILD="$(sw_vers -buildVersion)"; ARCH="$(uname -m)"
DATE="$(date -u +%Y-%m-%d)"
MEMGB=$(python3 -c "print(%.0f % ($(sysctl -n hw.memsize) / 1e9))")
SLUG="${DATE}-${ARCH}-macos${MACOS}-sys"
mkdir -p "$OUT_DIR"
MD="$OUT_DIR/$SLUG.md"; JSON="$OUT_DIR/$SLUG.json"

jq -n \
  --arg date "$DATE" --arg machine "$MACHINE" --arg macos "$MACOS" --arg build "$BUILD" \
  --arg arch "$ARCH" --arg ncpu "$NCPU" --arg memgb "$MEMGB" \
  --argjson load_start "$LOAD_START" --argjson load_end "$LOAD_END" \
  --argjson disk_mb "$DISK_MB" --argjson clone_mb "$CLONE_MB" \
  --argjson runs "$RUNS" \
  --argjson write_s "$DISK_WRITE" --argjson read_s "$DISK_READ" \
  --argjson lat_s "$DISK_LAT" --argjson clone_s "$DISK_CLONE" \
  --argjson cpu_single "$CPU_SINGLE_MBPS" --argjson cpu_multi "$CPU_MULTI_MBPS" \
  --argjson mem_mbps "$MEM_MBPS" \
  '{date:$date, machine:$machine, macos:$macos, build:$build, arch:$arch,
    ncpu:($ncpu|tonumber), mem_gb:($memgb|tonumber),
    load_start:($load_start|tonumber), load_end:($load_end|tonumber),
    disk_mb:($disk_mb|tonumber), runs:($runs|tonumber),
    disk: {sequential_write_s:$write_s, sequential_read_s:$read_s,
           write_mbps:(($disk_mb|tonumber)/$write_s|round),
           read_mbps:(($disk_mb|tonumber)/$read_s|round),
           clonefile_s:$clone_s,
           clonefile_mbps:(($clone_mb|tonumber)/$clone_s|round),
           write_4k_2048blocks_s:$lat_s},
    cpu: {single_core_sha256_mbps:$cpu_single, all_core_sha256_mbps:$cpu_multi},
    memory: {bandwidth_mbps:$mem_mbps}}' >"$JSON"

{
  echo "## System benchmark: $DATE on $MACHINE, macOS $MACOS ($BUILD)"
  echo
  echo "Host-only measurements. Not a Replica number. Read the project results"
  echo "in $(basename "$OUT_DIR") against this hardware."
  echo
  echo "| Metric | Value |"
  echo "|---|---|"
  echo "| CPU | $MACHINE ($ARCH, $NCPU cores) |"
  echo "| Memory | ${MEMGB} GB |"
  echo "| Load (1 min) | $LOAD_START start / $LOAD_END end |"
  echo "| Disk seq write | $(python3 -c "print('%.0f' % ($DISK_MB/$DISK_WRITE))") MB/s |"
  echo "| Disk seq read | $(python3 -c "print('%.0f' % ($DISK_MB/$DISK_READ))") MB/s |"
  echo "| Disk clonefile | $(python3 -c "print('%.0f' % ($CLONE_MB/$DISK_CLONE))") MB/s |"
  echo "| CPU single-core (sha256) | ${CPU_SINGLE_MBPS} MB/s |"
  echo "| CPU all-core (sha256) | ${CPU_MULTI_MBPS} MB/s |"
  echo "| Memory bandwidth | ${MEM_MBPS} MB/s |"
  echo
  echo "Machine-readable: $JSON"
} >"$MD"

cat "$MD"
echo
echo "wrote $MD"
echo "wrote $JSON"
