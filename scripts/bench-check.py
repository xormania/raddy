#!/usr/bin/env python3
"""Compare measure JSON to bench/baselines.json. Fail on >10% p50 regression."""

import json
import sys

if len(sys.argv) != 3:
    sys.stderr.write("usage: bench-check.py BASELINE GOT\n")
    sys.exit(2)

base = json.load(open(sys.argv[1], encoding="utf-8"))
got = json.load(open(sys.argv[2], encoding="utf-8"))
fail = False
for key in ("resume_p50_ns", "e2e_hello_warm_p50_ns", "ttfb_p50_ns"):
    b, g = base[key], got[key]
    if b <= 0:
        continue
    ratio = g / b
    print(f"{key}: baseline={b} got={g} ratio={ratio:.3f}")
    if ratio > 1.10:
        print(f"REGRESSION {key} >10%")
        fail = True
sys.exit(1 if fail else 0)
