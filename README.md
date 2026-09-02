# TMus

终端（TUI）音乐播放器。进入任意文件夹运行 `tmus`，浏览当前目录与子目录、
播放本地音频（含 32-bit FLAC），播放时显示实时频谱可视化。
发行包内置静态 ffmpeg，普通用户无需安装任何依赖。

## 安装

### macOS 端

1. 打开 GitHub Releases 页，下载对应芯片的安装包：
   - Apple 芯片（M1/M2/M3/M4）：`TMus-macos-arm64.zip`
   - Intel：`TMus-macos-x86_64.zip`
2. 解压后，把 `tmus` 和 `ffmpeg` 放到同一个文件夹（如 `~/bin` 或 `~/.local/bin`）：

   ```bash
   mkdir -p ~/.local/bin
   cp tmus ffmpeg ~/.local/bin/
   ```

3. 新开终端，任意文件夹里运行：

   ```bash
   tmus
   ```

> 想从源码构建：`brew install ffmpeg` 后执行 `cargo build --release`，
> 运行 `./target/release/tmus`。

### Windows 端

1. 打开 GitHub Releases 页，下载 `TMus-windows-x86_64.zip`。
2. 解压到任意目录，双击 `tmus.exe` 即可使用（建议使用 Windows Terminal）。
3. 想全局可用：在该目录右键 `install.ps1` → “使用 PowerShell 运行”，
   程序会装入 `%LOCALAPPDATA%\tmus` 并加入用户 PATH，之后直接运行 `tmus`。

## 使用

```bash
cd ~/Music/专辑名     # 进入任意音乐文件夹
tmus                 # 也可以 tmus ~/Music 指定目录
```

| 按键 | 功能 |
| --- | --- |
| `↑/↓` / `j` `k` | 移动选择 |
| `Enter` | 播放曲目 / 进入目录 |
| `Backspace` / `h` | 返回上级目录 |
| `Space` | 播放 / 暂停 |
| `n` / `p` | 下一首 / 上一首 |
| `←` / `→` | 快退 / 快进 5 秒 |
| `v` | 切换可视方式（频谱柱 / 波形 / 环形频谱） |
| `c` / `C` | 循环配色 / 自定义颜色（`#rrggbb`） |
| `q` | 退出 |

设置自动保存到 `~/.config/tmus/config.json`，删除该文件可恢复默认。
