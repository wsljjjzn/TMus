#!/usr/bin/env bash
# 在 macOS 本机构建便携 zip：tmus + 静态 ffmpeg（同目录，目标机器无需装 ffmpeg）
# 产物：dist/tmus-macos-<架构>.zip
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release

ARCH="$(uname -m)"
case "$ARCH" in
    arm64)  ASSET="ffmpeg-darwin-arm64.gz" ;;
    x86_64) ASSET="ffmpeg-darwin-x64.gz" ;;
    *) echo "✗ 不支持的架构: $ARCH" >&2; exit 1 ;;
esac

# 下载静态 ffmpeg（只依赖系统库，跨机器可运行）
FF_URL="https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/${ASSET}"
TMP="$(mktemp -d)/ffmpeg"
echo "↓ 下载静态 ffmpeg: $FF_URL"
curl -sL -o "$TMP.gz" "$FF_URL"
gunzip -f "$TMP.gz"
chmod +x "$TMP"

rm -rf dist && mkdir -p dist
cp target/release/tmus dist/
cp "$TMP" dist/ffmpeg
cp README.md dist/
cp install.ps1 dist/ 2>/dev/null || true

(cd dist && zip -rq "TMus-macos-${ARCH}.zip" .)
rm -f "$TMP"

echo "✅ 打包完成: dist/TMus-macos-${ARCH}.zip"
echo "   发送给 macOS 用户：解压后运行 ./tmus（内含 ffmpeg，无需任何安装）"
