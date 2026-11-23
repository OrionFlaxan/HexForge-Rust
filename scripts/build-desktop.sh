#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --workspace --release
ls -lh "$ROOT_DIR/target/release/hexforge" "$ROOT_DIR/target/release/hexforge-bench"
