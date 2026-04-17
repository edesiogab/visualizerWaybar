#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="waybar-audio-visualizer"
TARGET_BIN="${HOME}/.local/bin/${BIN_NAME}"

if [[ -f "${TARGET_BIN}" ]]; then
  rm -f "${TARGET_BIN}"
  echo "Removed ${TARGET_BIN}"
else
  echo "Nothing to remove at ${TARGET_BIN}"
fi
