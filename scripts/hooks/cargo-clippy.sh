#!/usr/bin/env bash
# clippy 用独立 target 目录，避免 clippy-driver 产物污染 rustc 缓存（否则 cargo test 全量重编）。
# 依赖 bash 在 PATH 中（Unix 默认有；Windows 用户将 Git Bash 的 bin 目录加入 PATH）。
set -euo pipefail
cd "$(dirname "$0")/../.."
CARGO_TARGET_DIR=target/clippy cargo clippy --fix --allow-dirty --all-targets -- -D warnings
