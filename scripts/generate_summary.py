#!/usr/bin/env python3
"""Generate benchmark summary markdown from comparison results."""

import json
import sys
from pathlib import Path

def main():
    input_file = Path("compare_results.json")
    output_file = "benchmark_summary.md"

    if not input_file.exists():
        print("Warning: compare_results.json not found", file=sys.stderr)
        return

    with open(input_file, 'r') as f:
        data = json.load(f)

    with open(output_file, 'a') as f:
        for bench in data.get('benchmarks', []):
            name = bench.get('name', 'Unknown')
            yaoxiang = bench.get('yaoxiang', 0)
            python = bench.get('python', 0)
            rust = bench.get('rust', 0)
            cpp = bench.get('cpp', 0)
            go = bench.get('go', 0)
            f.write(f'| {name} | {yaoxiang:.3f}ms | {python:.3f}ms | {rust:.3f}ms | {cpp:.3f}ms | {go:.3f}ms |\n')

if __name__ == "__main__":
    main()
