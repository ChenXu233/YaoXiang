#!/usr/bin/env python3
"""Cargo runner forwarder: find the workspace-level z3_runner.py from the bench package root.

Cargo sets the target runner cwd to the package directory. The workspace-root
.cargo/config.toml uses the relative path 'scripts/dev/z3_runner.py'; that path does
not exist under benches/shootout/runner. This file only resolves and forwards; it
does not copy the z3 lookup logic. The real runner remains <repo>/scripts/dev/z3_runner.py.
"""
import os
import runpy
import sys


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    self_path = os.path.abspath(__file__)
    probe = here
    while True:
        candidate = os.path.join(probe, "scripts", "dev", "z3_runner.py")
        if os.path.abspath(candidate) != self_path and os.path.isfile(candidate):
            sys.argv[0] = candidate
            runpy.run_path(candidate, run_name="__main__")
            return
        parent = os.path.dirname(probe)
        if parent == probe:
            raise SystemExit("z3_runner.py: workspace root not found from " + here)
        probe = parent


if __name__ == "__main__":
    main()
