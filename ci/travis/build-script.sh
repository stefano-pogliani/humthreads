#!/usr/bin/env bash
set -ex

cargo build --verbose
cargo test --verbose
cargo clippy --verbose -- -D warnings
cargo fmt --verbose -- --check
