#!/bin/bash
if [ -z "$1" ]; then
  echo "Usage: $0 <repo_url> <options>"
  exit 1
fi
cargo install --path . && ~/.cargo/bin/orchestrator-loop-for-ai --repo $@
