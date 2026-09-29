# Multisig development setup

Your assignment is roadmap section 3, Module 16: Multisig & Corporate Treasury.
Verify the base development chain before implementing the pallet.

## Environment checked

Ubuntu on WSL 2 has Rust stable 1.98.1, Cargo, rust-src, Clippy, rustfmt and the
wasm32-unknown-unknown target installed. The installed build packages include
build-essential, Git, Clang, libclang-dev, CMake, pkg-config, libssl-dev and
protobuf-compiler.

This checkout specifies stable Rust. The pasted notes mention nightly; keep
the checkout's toolchain unless a verified build failure requires a change.

## Run from PowerShell in the project directory

```powershell
wsl -d Ubuntu -- bash ./run-dev.sh
```

The launcher builds locked dependencies with two jobs and starts the development
chain on success. Build output goes to `E:\codex-build\solochain-template-node`
because jemalloc rejects spaces in build paths. An explicit `cstdint` include
addresses the RocksDB 8.1.1 error with GCC 15.

First builds are lengthy. Avoid cargo clean: it removes reusable build files.
After successful startup, connect to `ws://127.0.0.1:9944` and confirm block
numbers advance. Use Ctrl+C to stop a foreground node. Development mode uses
test accounts and temporary state.

## Verify the template pallet in Ubuntu

```bash
source "$HOME/.cargo/env"
CARGO_TARGET_DIR=/mnt/e/codex-build/solochain-template-node \
  cargo test --locked --release --jobs 2 -p pallet-template
```

## Before pallet development

Do not copy workspace entries for other teams' pallets from the pasted notes
until those source directories exist. Preserve this checkout's SDK versions.

Module 16 covers weighted signer thresholds, proposals and approvals, treasury
policies, expiry and veto, time-locked administration and batch execution.
Coordinate the credential-verification runtime trait with the Module 15 identity
team before integrating credential-gated signer roles.

Successful node startup and passing tests must still be verified after the build.
