#!/usr/bin/env bash
# Builds the release binary and prints its runtime requirements.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo build --release -p audio-server
BIN=target/release/audio-server
ls -lh "$BIN"
echo "Shared libraries:"; ldd "$BIN" | sed 's/^/  /'
echo "Minimum glibc symbol version:"
objdump -T "$BIN" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1 | sed 's/^/  /'
