#!/usr/bin/env bash
# macOS 本地打发行包：tmus + 静态 ffmpeg + 安装脚本
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release

ARCH="$(uname -m)"
case "$ARCH" in
    arm64)  ASSET="ffmpeg-darwin-arm64.gz" ;;
    x86_64) ASSET="ffmpeg-darwin-x64.gz" ;;
    *) echo "✗ 不支持的架构: $ARCH" >&2; exit 1 ;;
esac

TMP="$(mktemp -d)/ffmpeg"
echo "↓ 下载静态 ffmpeg"
curl -sL -o "$TMP.gz" "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/${ASSET}"
gunzip -f "$TMP.gz"; chmod +x "$TMP"

rm -rf dist && mkdir -p dist
cp target/release/tmus dist/
cp "$TMP" dist/ffmpeg
cp README.md install.sh install.ps1 dist/
(cd dist && zip -rq "TMus-macos-${ARCH}.zip" .)
rm -f "$TMP"

echo "✅ 打包完成: dist/TMus-macos-${ARCH}.zip"
