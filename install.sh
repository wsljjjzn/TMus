#!/usr/bin/env bash
# 重新编译并安装 tmus 到 ~/.local/bin（已在 PATH 中）
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release

DEST="${HOME}/.local/bin"
mkdir -p "$DEST"
install -m 755 "target/release/tmus" "$DEST/tmus"

echo "✅ 已安装: $DEST/tmus"
echo "  在任意文件夹运行  mus（或 tmus）即可播放当前目录"
