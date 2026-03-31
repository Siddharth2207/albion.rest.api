#!/usr/bin/env python3
import argparse
import csv
from collections import defaultdict


def percentile(sorted_vals, p):
    if not sorted_vals:
        return None
    idx = int((len(sorted_vals) - 1) * p)
    return sorted_vals[idx]


def summarize(path):
    values = defaultdict(list)
    errors = defaultdict(int)
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            scenario = row["scenario"]
            code = row["http_code"]
            t = float(row["total_time"])
            if code != "200":
                errors[scenario] += 1
                continue
            values[scenario].append(t)

    out = {}
    for scenario, vals in values.items():
        vals = sorted(vals)
        out[scenario] = {
            "n": len(vals),
            "errors": errors.get(scenario, 0),
            "avg": sum(vals) / len(vals),
            "p50": percentile(vals, 0.50),
            "p95": percentile(vals, 0.95),
            "min": vals[0],
            "max": vals[-1],
        }

    for scenario, err_count in errors.items():
        if scenario not in out:
            out[scenario] = {
                "n": 0,
                "errors": err_count,
                "avg": None,
                "p50": None,
                "p95": None,
                "min": None,
                "max": None,
            }

    return out


def fmt(v):
    if v is None:
        return "-"
    return f"{v:.3f}"


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--local", required=True)
    p.add_argument("--prod", required=True)
    args = p.parse_args()

    local = summarize(args.local)
    prod = summarize(args.prod)

    scenarios = sorted(set(local.keys()) | set(prod.keys()))

    print(
        "| scenario | local avg (s) | local p50 (s) | local p95 (s) | prod avg (s) | prod p50 (s) | prod p95 (s) | avg ratio (prod/local) | prod errors |"
    )
    print(
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    for s in scenarios:
        l = local.get(s, {})
        pr = prod.get(s, {})
        ratio = "-"
        if l.get("avg") and pr.get("avg"):
            ratio = f"{(pr['avg'] / l['avg']):.2f}x"
        print(
            "| {s} | {lavg} | {lp50} | {lp95} | {pavg} | {pp50} | {pp95} | {ratio} | {perr} |".format(
                s=s,
                lavg=fmt(l.get("avg")),
                lp50=fmt(l.get("p50")),
                lp95=fmt(l.get("p95")),
                pavg=fmt(pr.get("avg")),
                pp50=fmt(pr.get("p50")),
                pp95=fmt(pr.get("p95")),
                ratio=ratio,
                perr=pr.get("errors", 0),
            )
        )


if __name__ == "__main__":
    main()
