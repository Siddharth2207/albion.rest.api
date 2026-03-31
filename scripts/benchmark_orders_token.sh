#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  benchmark_orders_token.sh \
    --base-url <url> \
    --api-key <key> \
    --api-secret <secret> \
    --token <address> \
    [--runs <n>] \
    [--output <csv-path>]

Notes:
  - Benchmarks GET /v1/orders/token/{token}
  - Scenarios:
      single_1000 : page=1&pageSize=1000 (server may clamp)
      full_50     : full pagination with pageSize=50
      full_20     : full pagination with pageSize=20
      full_10     : full pagination with pageSize=10
EOF
}

BASE_URL=""
API_KEY=""
API_SECRET=""
TOKEN=""
RUNS=10
OUTPUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url)
      BASE_URL="${2:-}"
      shift 2
      ;;
    --api-key)
      API_KEY="${2:-}"
      shift 2
      ;;
    --api-secret)
      API_SECRET="${2:-}"
      shift 2
      ;;
    --token)
      TOKEN="${2:-}"
      shift 2
      ;;
    --runs)
      RUNS="${2:-}"
      shift 2
      ;;
    --output)
      OUTPUT="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$BASE_URL" || -z "$API_KEY" || -z "$API_SECRET" || -z "$TOKEN" ]]; then
  echo "Missing required arguments." >&2
  usage
  exit 1
fi

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [[ "$RUNS" -lt 1 ]]; then
  echo "--runs must be a positive integer" >&2
  exit 1
fi

if [[ -z "$OUTPUT" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  OUTPUT="/tmp/orders-token-benchmark-${ts}.csv"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

request_once() {
  local page="$1"
  local page_size="$2"
  local out_json="$3"

  curl -sS -u "${API_KEY}:${API_SECRET}" \
    --connect-timeout 10 \
    --max-time 120 \
    -o "$out_json" \
    -w "%{http_code},%{time_total}" \
    "${BASE_URL}/v1/orders/token/${TOKEN}?page=${page}&pageSize=${page_size}"
}

read_json_or_default() {
  local expr="$1"
  local file="$2"
  local default_value="$3"

  if jq -e "$expr" "$file" >/dev/null 2>&1; then
    jq -r "$expr" "$file"
  else
    echo "$default_value"
  fi
}

single_total() {
  local run="$1"
  local page_size="$2"
  local out_json="${tmp_dir}/single-${run}-${page_size}.json"
  local meta code t total_orders returned

  meta="$(request_once 1 "$page_size" "$out_json")"
  code="${meta%%,*}"
  t="${meta##*,}"
  total_orders="$(read_json_or_default '.pagination.totalOrders // .pagination.total_orders // -1' "$out_json" -1)"
  returned="$(read_json_or_default '.orders | length' "$out_json" -1)"

  printf "single_%s,%s,%s,%.6f,%s,%s\n" \
    "$page_size" "$run" "$code" "$t" "$total_orders" "$returned"
}

full_total() {
  local run="$1"
  local page_size="$2"
  local page=1
  local calls=0
  local total_time=0
  local has_more=true
  local total_orders=0
  local returned_sum=0
  local last_code=200

  while [[ "$has_more" == "true" ]]; do
    local out_json="${tmp_dir}/full-${run}-${page_size}-${page}.json"
    local meta code t returned

    meta="$(request_once "$page" "$page_size" "$out_json")"
    code="${meta%%,*}"
    t="${meta##*,}"
    last_code="$code"

    if [[ "$code" != "200" ]]; then
      break
    fi

    calls=$((calls + 1))
    total_time="$(awk -v a="$total_time" -v b="$t" 'BEGIN { printf "%.6f", a + b }')"
    has_more="$(read_json_or_default '.pagination.hasMore // .pagination.has_more // false' "$out_json" false)"
    total_orders="$(read_json_or_default '.pagination.totalOrders // .pagination.total_orders // -1' "$out_json" -1)"
    returned="$(read_json_or_default '.orders | length' "$out_json" -1)"
    if [[ "$returned" == "-1" ]]; then
      break
    fi
    returned_sum=$((returned_sum + returned))
    page=$((page + 1))
  done

  printf "full_%s,%s,%s,%.6f,%s,%s,%s\n" \
    "$page_size" "$run" "$last_code" "$total_time" "$total_orders" "$returned_sum" "$calls"
}

{
  echo "scenario,run,http_code,total_time,total_orders,returned,calls"
  for run in $(seq 1 "$RUNS"); do
    single_total "$run" 1000
    full_total "$run" 50
    full_total "$run" 20
    full_total "$run" 10
  done
} > "$OUTPUT"

echo "wrote ${OUTPUT}"
