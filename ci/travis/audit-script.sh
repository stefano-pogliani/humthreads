#!/usr/bin/env bash
set -ex

# Need a lock file to audit.
cargo update
cargo audit
