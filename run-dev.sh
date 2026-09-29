#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1 && [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
fi

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# jemalloc cannot build in a directory whose path contains spaces.
# Keep the generated files on E: when running this checkout in WSL.
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
    if [[ "$PWD" == /mnt/e/* ]]; then
        export CARGO_TARGET_DIR=/mnt/e/codex-build/solochain-template-node
    else
        export CARGO_TARGET_DIR="$HOME/.cache/solochain-template-node"
    fi
fi
mkdir -p "$CARGO_TARGET_DIR/tmp"
export TMPDIR="$CARGO_TARGET_DIR/tmp"

# RocksDB 8.1.1 needs an explicit integer-types header with GCC 15.
export CXXFLAGS="${CXXFLAGS:+$CXXFLAGS }-include cstdint"

# Limit memory usage on smaller WSL installations.
cargo build --locked --release --jobs "${CARGO_BUILD_JOBS:-2}" -p solochain-template-node
exec "$CARGO_TARGET_DIR/release/solochain-template-node" --dev "$@"
