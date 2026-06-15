#!/usr/bin/env bash
# Get-Memory — system memory + swap usage (bytes), human table or JSON.
# Source: /proc/meminfo (locale-independent). --json requires jq.
# Usage: get-memory.sh [--json | -o json|table] [-h|--help]
set -euo pipefail

declare -gA MEM   # populated by collect_memory(); all values in bytes

usage() {
  cat <<'EOF'
Get-Memory — system memory + swap usage (bytes), table or JSON.
Usage: get-memory.sh [--json | -o json|table] [-h|--help]
Source: /proc/meminfo. --json requires jq.
EOF
}

# Read /proc/meminfo (reported in kB) and compute bytes the way `free` does:
#   cached = Cached + SReclaimable
#   used   = total - free - buffers - cached
collect_memory() {
  local -A kb
  local key val
  while read -r key val _; do
    [[ -n "$key" ]] && kb["${key%:}"]="${val:-0}"
  done < /proc/meminfo

  local K=1024
  MEM[total]=$(( ${kb[MemTotal]:-0} * K ))
  MEM[free]=$(( ${kb[MemFree]:-0} * K ))
  MEM[available]=$(( ${kb[MemAvailable]:-0} * K ))
  MEM[buffers]=$(( ${kb[Buffers]:-0} * K ))
  MEM[cached]=$(( ( ${kb[Cached]:-0} + ${kb[SReclaimable]:-0} ) * K ))
  MEM[shared]=$(( ${kb[Shmem]:-0} * K ))
  MEM[used]=$(( MEM[total] - MEM[free] - MEM[buffers] - MEM[cached] ))
  MEM[swap_total]=$(( ${kb[SwapTotal]:-0} * K ))
  MEM[swap_free]=$(( ${kb[SwapFree]:-0} * K ))
  MEM[swap_used]=$(( MEM[swap_total] - MEM[swap_free] ))
}

# bytes -> IEC human readable (e.g. 15.3 GiB)
human() {
  awk -v b="$1" 'BEGIN{
    split("B KiB MiB GiB TiB PiB", u, " ");
    i=1; while (b>=1024 && i<6){ b/=1024; i++ }
    printf (i==1 ? "%d %s\n" : "%.1f %s\n"), b, u[i]
  }'
}

emit_table() {
  printf "%-12s %12s\n" "Metric" "Size"
  printf "%-12s %12s\n" "------" "----"
  printf "%-12s %12s\n" "Total"      "$(human "${MEM[total]}")"
  printf "%-12s %12s\n" "Used"       "$(human "${MEM[used]}")"
  printf "%-12s %12s\n" "Free"       "$(human "${MEM[free]}")"
  printf "%-12s %12s\n" "Available"  "$(human "${MEM[available]}")"
  printf "%-12s %12s\n" "Buffers"    "$(human "${MEM[buffers]}")"
  printf "%-12s %12s\n" "Cached"     "$(human "${MEM[cached]}")"
  printf "%-12s %12s\n" "Shared"     "$(human "${MEM[shared]}")"
  printf "%-12s %12s\n" "Swap Total" "$(human "${MEM[swap_total]}")"
  printf "%-12s %12s\n" "Swap Used"  "$(human "${MEM[swap_used]}")"
  printf "%-12s %12s\n" "Swap Free"  "$(human "${MEM[swap_free]}")"
}

# Let jq own JSON construction so escaping/typing are always correct.
emit_json() {
  jq -n \
    --arg     ts        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson total     "${MEM[total]}" \
    --argjson used      "${MEM[used]}" \
    --argjson free      "${MEM[free]}" \
    --argjson available "${MEM[available]}" \
    --argjson buffers   "${MEM[buffers]}" \
    --argjson cached    "${MEM[cached]}" \
    --argjson shared    "${MEM[shared]}" \
    --argjson swt       "${MEM[swap_total]}" \
    --argjson swu       "${MEM[swap_used]}" \
    --argjson swf       "${MEM[swap_free]}" \
    '{
       timestamp: $ts,
       unit: "bytes",
       memory: {
         total: $total, used: $used, free: $free, available: $available,
         buffers: $buffers, cached: $cached, shared: $shared
       },
       swap: { total: $swt, used: $swu, free: $swf }
     }'
}

main() {
  local format="table"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --json)        format="json"; shift ;;
      -o|--output)   format="${2:-}"; shift 2 || true ;;
      -h|--help)     usage; exit 0 ;;
      *) echo "get-memory: unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
  done

  case "$format" in
    table|json) ;;
    *) echo "get-memory: invalid format: '$format' (want json|table)" >&2; exit 2 ;;
  esac

  collect_memory

  if [[ "$format" == "json" ]]; then
    command -v jq >/dev/null 2>&1 || { echo "get-memory: jq is required for JSON output" >&2; exit 1; }
    emit_json
  else
    emit_table
  fi
}

# Run when executed; allow `source get-memory.sh` to import the functions only.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
