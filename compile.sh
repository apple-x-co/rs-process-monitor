#!/bin/bash -eu

cd "$(dirname "$0")" || exit

# cargo --version
# cargo 1.89.0 (c24e10642 2025-06-23)

# on macos m2
# cargo check
# cargo build # dev profile
cargo build --release

# for linux (x86_64)
# cargo install cross
# docker pull ghcr.io/cross-rs/x86_64-unknown-linux-musl:0.2.5 --platform=linux/amd64
cross build --release --target=x86_64-unknown-linux-musl