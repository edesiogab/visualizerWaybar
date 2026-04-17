#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_NAME="waybar-audio-visualizer"
TARGET_BIN="${HOME}/.local/bin/${BIN_NAME}"

warn_missing() {
	local cmd="$1"
	local reason="$2"
	if ! command -v "$cmd" >/dev/null 2>&1; then
		echo "[warn] '$cmd' não encontrado: $reason"
	fi
}

echo "[1/3] Building release binary..."
cargo build --release --manifest-path "${PROJECT_ROOT}/Cargo.toml"

mkdir -p "${HOME}/.local/bin"
cp "${PROJECT_ROOT}/target/release/${BIN_NAME}" "${TARGET_BIN}"
chmod +x "${TARGET_BIN}"

echo "[2/3] Binary installed to ${TARGET_BIN}"
warn_missing "cava" "sem cava o visualizador usa fallback por nível de volume"
warn_missing "wpctl" "necessário para controle de volume no PipeWire"
warn_missing "playerctl" "necessário para click direito play/pause"
echo "[3/3] Next steps:"
echo "  1) Add the module snippet from config/waybar-module.jsonc into your ~/.config/waybar/config.jsonc"
echo "  2) Add styles from config/style-additions.css into your ~/.config/waybar/style.css"
echo "  3) Restart Waybar: omarchy-restart-waybar"
