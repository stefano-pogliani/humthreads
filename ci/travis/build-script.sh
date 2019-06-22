#!/usr/bin/env bash
set -ex

cargo build --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo doc --all-features
cargo fmt --verbose -- --check
