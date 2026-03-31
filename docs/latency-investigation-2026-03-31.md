# Orders-By-Token Latency Investigation (2026-03-31)

Target endpoint:
- `GET /v1/orders/token/0xf836a500910453A397084ADe41321ee20a5AAde1`

Benchmarks were executed with the same scenario matrix in both environments:
- `single_1000` (`page=1&pageSize=1000`, server clamps to 50)
- `full_50` (full pagination with `pageSize=50`)
- `full_20` (full pagination with `pageSize=20`)
- `full_10` (full pagination with `pageSize=10`)

Run count: `10` per scenario.

## Side-By-Side Results

| scenario | local avg (s) | local p50 (s) | local p95 (s) | prod avg (s) | prod p50 (s) | prod p95 (s) | avg ratio (prod/local) | prod errors |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| full_10 | 2.552 | 2.635 | 3.449 | 11.954 | 10.972 | 12.332 | 4.68x | 0 |
| full_20 | 1.668 | 1.343 | 2.400 | 10.404 | 10.205 | 10.666 | 6.24x | 1 |
| full_50 | 1.642 | 1.452 | 2.189 | 10.088 | 9.321 | 10.540 | 6.14x | 0 |
| single_1000 | 2.124 | 1.906 | 3.699 | 10.251 | 9.900 | 12.113 | 4.83x | 0 |

Key points:
- Production is consistently slower by roughly `4.7x` to `6.2x` for this endpoint.
- Production has heavy tail jitter (examples from raw runs):
  - `full_50`: `17.52s`
  - `full_10`: `20.15s`
- One production request failed with `504` (`full_20`, run 1).

## Control Endpoint Check

| env | endpoint | avg (s) | p50 (s) | min (s) | max (s) |
|---|---|---:|---:|---:|---:|
| local | /health | 0.001 | 0.001 | 0.001 | 0.001 |
| local | /v1/tokens | 0.244 | 0.244 | 0.243 | 0.246 |
| prod | /health | 0.173 | 0.120 | 0.112 | 0.260 |
| prod | /v1/tokens | 0.251 | 0.294 | 0.143 | 0.373 |

Interpretation:
- General API path/network is not the main issue.
- Slowdown is specific to `orders/token` quote-heavy path.

## Most Likely Cause (Current Evidence)

Based on no-deploy data:
1. The cost scales with quote work in production, and the baseline per quote appears much higher than local.
2. Production shows larger tail spikes and occasional gateway failure (`504`), consistent with upstream quote-path instability (provider latency/retry/backpressure) rather than ingress latency.
3. Prior local server runs showed local DB lock warnings during sync; production may have similar contention, but this requires production log/metric correlation to confirm.

## Reproducibility

Scripts added:
- `scripts/benchmark_orders_token.sh`
- `scripts/summarize_orders_token_bench.py`

Artifacts generated in this run:
- Local raw: `/tmp/local_orders_token_bench.csv`
- Prod raw: `/tmp/prod_orders_token_bench.csv`
- Control raw: `/tmp/control_compare.csv`

## Next Investigation Step (No Deploy)

Correlate slow request windows with production telemetry:
- Pull application logs for sampled `x-request-id` values from slow calls.
- Check for quote retries/timeouts/provider errors around those requests.
- Compare host metrics (CPU, IO wait, DB lock behavior) during the same windows.

If telemetry still cannot isolate stage-level bottleneck, add minimal stage timing instrumentation in `/v1/orders/token`.

## Instrumentation Added (Ready To Deploy)

Code now emits structured timing logs for `GET /v1/orders/token/{address}`:
- Route stage summary:
  - `orders_stage_duration_ms`
  - `quotes_stage_duration_ms`
  - `total_duration_ms`
  - `quote_success_count`
  - `quote_empty_count`
  - `quote_error_count`
  - `returned_orders`
  - `total_orders`
  - `page`, `page_size`
- Handler-level signal:
  - `local_db_path`
  - `local_db_size_bytes`
- Data source-level timings:
  - `queried orders list` with `duration_ms`
  - `queried order quotes` per order with `duration_ms` and `order_hash`

Deployment path is available via GitHub Actions:
- `.github/workflows/cd.yaml` (`workflow_dispatch`)
- deploy step runs `nix run .#deployAll`
