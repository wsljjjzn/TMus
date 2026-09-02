#!/usr/bin/env bash
# ============================================================
#  TMus 安装脚本（macOS）
#
#  两种用法：
#   1) 发行包：解压 zip 后，在该目录执行  ./install.sh
#   2) 源码目录：直接 ./install.sh（自动 cargo build --release）
#
#  ffmpeg 自动检测：
#   - 系统已有 ffmpeg（PATH 或 Homebrew）→ 直接使用系统版，不复制内置文件
#   - 系统没有 → 自动安装发行包内置的静态 ffmpeg
# ============================================================
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST="${TMUS_DEST:-${HOME}/.local/bin}"

echo "==> TMus 安装开始"

# ---------- 1. tmus ----------
TMUS_SRC=""
if [ -x "$DIR/tmus" ]; then
  TMUS_SRC="$DIR/tmus"
elif [ -f "$DIR/Cargo.toml" ]; then
  echo "    未找到现成二进制，从源码编译（需要 Rust 工具链）…"
  (cd "$DIR" && cargo build --release)
  TMUS_SRC="$DIR/target/release/tmus"
else
  echo "✗ 找不到 tmus。本脚本应放在发行包（含 tmus）目录中执行。" >&2
  exit 1
fi

mkdir -p "$DEST"
install -m 755 "$TMUS_SRC" "$DEST/tmus"
echo "    ✓ tmus 已安装到 $DEST/tmus"

# ---------- 2. ffmpeg：先用系统的，没有再用内置 ----------
system_ffmpeg="$(command -v ffmpeg 2>/dev/null || true)"
[ -z "$system_ffmpeg" ] && [ -x /opt/homebrew/bin/ffmpeg ] && system_ffmpeg="/opt/homebrew/bin/ffmpeg"
[ -z "$system_ffmpeg" ] && [ -x /usr/local/bin/ffmpeg ] && system_ffmpeg="/usr/local/bin/ffmpeg"

if [ -n "$system_ffmpeg" ]; then
  echo "    ✓ 检测到系统 ffmpeg: $system_ffmpeg"
  echo "      → 直接使用系统版本（不复制内置文件，避免重复占用磁盘）"
else
  if [ -x "$DIR/ffmpeg" ]; then
    install -m 755 "$DIR/ffmpeg" "$DEST/ffmpeg"
    echo "    ✓ 未检测到系统 ffmpeg，已安装发行包内置静态版到 $DEST/ffmpeg"
  else
    echo "  ! 未检测到系统 ffmpeg，且目录内也没有内置 ffmpeg。" >&2
    echo "    请执行 brew install ffmpeg，或把发行包里的 ffmpeg 与本脚本放在同目录后重试。" >&2
    exit 1
  fi
fi

# ---------- 3. PATH ----------
case ":$PATH:" in
  *":$DEST:"*) echo "    ✓ $DEST 已在 PATH 中" ;;
  *)
    RC=""
    case "${SHELL:-}" in
      *zsh) RC="${HOME}/.zshrc" ;;
      *bash) RC="${HOME}/.bashrc" ;;
      *) RC="${HOME}/.zshrc" ;;
    esac
    if [ -f "$RC" ]; then
      printf '\n# === TMus ===\n# 添加 ~/.local/bin 到 PATH（tmus 在此目录）\nexport PATH="$PATH:%s"\n' "$DEST" >> "$RC"
      echo "    ✓ 已将 $DEST 写入 $RC（新开终端生效）"
    else
      echo "  ! 未找到 shell 配置，请手动把 $DEST 加入 PATH" >&2
    fi
    ;;
esac

echo ""
echo "✅ 安装完成！"
echo "   使用方法：打开终端，进入任意音乐文件夹后输入："
echo "       tmus"
echo "   新开终端窗口使 PATH 生效；首次运行如被系统拦截，请右键 tmus → 打开"
