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

## 环境足迹（不会污染系统）

- 安装 = 只往 PATH 目录放了 `tmus` 和 `ffmpeg` 两个文件（macOS `~/.local/bin/`，Windows `%LOCALAPPDATA%\tmus`）
- 运行时只读当前文件夹；设置保存在 `~/.config/tmus/config.json`（改了才创建）
- 解码临时文件在系统临时目录且用后即删
- 不使用管理员权限、不写注册表/启动项、无后台进程、无自动更新
- 想零安装：解压后直接运行（macOS 需 `chmod +x tmus`，首次右键→打开）

## 彻底卸载

macOS：

```bash
rm -f ~/.local/bin/tmus ~/.local/bin/ffmpeg   # 若 ffmpeg 是 Homebrew 装的，请保留并改用 brew uninstall ffmpeg
rm -rf ~/.config/tmus                          # 删除设置
rm -f /tmp/tmus-*.wav                          # 清理可能的解码残留
command -v tmus                                # 应无输出 = 卸载干净
```

Windows：删除 `%LOCALAPPDATA%\tmus` 文件夹，并在“系统属性 → 环境变量 → Path”里移除该路径（或重跑 install.ps1 的卸载逻辑）。
