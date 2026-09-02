#!/usr/bin/env bash
# ============================================================
#  TMus 卸载脚本（macOS）——自动探测安装痕迹并清理
#
#  用法：./uninstall.sh          # 交互确认
#        ./uninstall.sh -y      # 全部自动确认（脚本化）
#
#  自动处理：
#   - 删除 tmus（默认安装目录 ~/.local/bin，或 TMUS_DEST 自定义目录；
#     若在其它路径也找到 tmus 会提示）
#   - 删除我们装入同目录的 ffmpeg（只删安装目录里那份，绝不动 Homebrew/系统 ffmpeg）
#   - 删除配置 ~/.config/tmus 与 /tmp 解码残留
#   - 自动清理安装脚本写入 shell 配置的 PATH 块（只删带 # === TMus === 标记的行）
# ============================================================
set -euo pipefail

AUTO=0
[ "${1:-}" = "-y" ] && AUTO=1

DEST="${TMUS_DEST:-${HOME}/.local/bin}"
echo "==> TMus 卸载开始"

confirm() { # $1 提示
    if [ "$AUTO" = 1 ]; then return 0; fi
    local ans
    read -r -p "$1 [y/N] " ans
    case "$ans" in y|Y) return 0;; *) return 1;; esac
}

# ---------- 1. tmus ----------
GONE=1
if [ -x "$DEST/tmus" ]; then
    rm -f "$DEST/tmus"
    echo "    ✓ 已删除 $DEST/tmus"
    GONE=0
fi

# 其它位置的 tmus（例如曾手动拷贝到别处）
OTHER="$(command -v tmus 2>/dev/null || true)"
if [ -n "$OTHER" ] && [ "$OTHER" != "$DEST/tmus" ]; then
    if confirm "    ? 还发现 $OTHER，是否一并删除？"; then
        rm -f "$OTHER"
        echo "    ✓ 已删除 $OTHER"
    fi
fi

# ---------- 2. 我们安装的 ffmpeg（仅限安装目录内那份） ----------
if [ -f "$DEST/ffmpeg" ]; then
    if confirm "    ? 是否删除我们装入 $DEST 的内置 ffmpeg？"; then
        rm -f "$DEST/ffmpeg"
        echo "    ✓ 已删除 $DEST/ffmpeg"
    else
        echo "    · 保留 $DEST/ffmpeg"
    fi
fi

# ---------- 3. 配置与临时文件 ----------
if [ -d "${HOME}/.config/tmus" ]; then
    rm -rf "${HOME}/.config/tmus"
    echo "    ✓ 已删除配置 ~/.config/tmus"
fi
rm -f /tmp/tmus-*.wav 2>/dev/null || true

# ---------- 4. shell 配置里的 PATH 痕迹（只删我们加的块） ----------
for RC in "${HOME}/.zshrc" "${HOME}/.bashrc"; do
    [ -f "$RC" ] || continue
    if grep -q "# === TMus ===" "$RC"; then
        awk '
            BEGIN { skip = 0 }
            /# === TMus ===/ { skip = 1; next }
            skip && /export PATH/ { skip = 0; next }
            skip { next }
            { print }
        ' "$RC" > "${RC}.tmus-tmp"
        mv "${RC}.tmus-tmp" "$RC"
        echo "    ✓ 已从 $RC 移除 TMus 的 PATH 配置"
    fi
done

# ---------- 5. 结果 ----------
if command -v tmus >/dev/null 2>&1; then
    echo "  ! tmus 仍可用：$(command -v tmus)（可能被手动装到其它位置）"
else
    echo "✅ 卸载完成：tmus 已不可用，配置与 PATH 痕迹已清理"
fi
