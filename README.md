# TMus

终端（TUI）音乐播放器。在任意文件夹输入 `tmus`，即可浏览当前目录与子目录并播放
本地音频（mp3 / flac / wav / m4a…，含 32-bit FLAC），播放时显示实时频谱可视化
（频谱柱 / 波形 / 环形频谱，可自定义颜色）。发行包内置静态 ffmpeg，普通用户无需额外安装。

## 演示

点击海报即可在 GitHub 内置播放器中观看演示视频：

<p align="center">
  <a href="videoSample/sample.mp4">
    <img src="videoSample/poster.jpg" alt="TMus 演示视频" width="720">
  </a>
  <br>
  <sub>▲ 点击播放：<code>videoSample/sample.mp4</code></sub>
</p>

---

## 安装

> 发行包下载：GitHub Releases → 选择平台 zip：
> macOS（Apple 芯片 `TMus-macos-arm64.zip` / Intel `TMus-macos-x86_64.zip`）、
> Windows（`TMus-windows-x86_64.zip`）。

### macOS 端

**方式一（推荐）：安装脚本，自动处理一切**

```bash
# 解压后进入该目录
cd ~/Downloads/TMus-macos-arm64          # 换成你的解压路径
./install.sh
```

脚本会自动完成：
1. 把 `tmus` 安装到 `~/.local/bin`（并自动加入 PATH，新开终端生效）
2. **ffmpeg 自动检测**：
   - 系统已有 ffmpeg（PATH 或 Homebrew）→ 直接使用系统版，**不复制**内置文件
   - 系统没有 → 自动安装发行包内置的静态 ffmpeg
3. 首次运行如被 Gatekeeper 拦截：访达右键 `tmus` →「打开」

**方式二：手动安装**

```bash
mkdir -p ~/.local/bin
cp tmus ffmpeg ~/.local/bin/        # 已有系统 ffmpeg 时可只拷 tmus
# 确认 ~/.local/bin 在 PATH 中；没有则在 ~/.zshrc 加一行 export PATH="$PATH:$HOME/.local/bin"
```

### Windows 端

**方式一（推荐）：安装脚本**

解压后，右键 `install.ps1` →「使用 PowerShell 运行」（或在该目录执行
`powershell -ExecutionPolicy Bypass -File .\install.ps1`）。脚本自动完成：
1. 把 `tmus.exe` 装到 `%LOCALAPPDATA%\tmus` 并加入用户 PATH
2. **ffmpeg 自动检测**：系统已有（PATH 中）→ 使用系统版；没有 → 安装内置 `ffmpeg.exe`

**方式二：免安装直接玩**

解压后双击 `tmus.exe` 即可（不会写 PATH，仅当前文件夹可用；建议用 Windows Terminal，
程序已自动把控制台切到 UTF-8）。

### ffmpeg 的处理逻辑（重要）

tmus 找 ffmpeg 的优先级（越高越优先）：

1. 环境变量 `TMUS_FFMPEG=/路径`（强制指定）
2. 与 `tmus` **同目录**的 `ffmpeg`（发行包内置那份）
3. 常见安装位置（Homebrew 等）
4. 系统 PATH 中的 `ffmpeg`

所以：
- **用户已有 ffmpeg**：安装脚本检测到后会直接使用系统版、不复制内置文件（省 ~45MB 磁盘）；手动安装时也只拷 `tmus` 即可
- **用户没有**：脚本自动放一份内置静态 ffmpeg 到同目录
- 两种情况程序都能工作，只是来源不同；想强制用某个 ffmpeg 就设 `TMUS_FFMPEG`

### 从源码构建（可选）

```bash
brew install ffmpeg        # Windows: winget install ffmpeg
cargo build --release
./target/release/tmus
```

---

## 使用方法

```bash
cd ~/Music/专辑名        # 进入任意文件夹
tmus                    # 播放当前目录
tmus ~/Music/某目录     # 或直接指定目录
```

| 按键 | 功能 |
| --- | --- |
| `↑/↓` / `j` `k` | 移动选择 |
| `Enter` | 播放曲目 / 进入目录 |
| `Backspace` / `h` | 返回上级目录 |
| `Space` | 播放 / 暂停 |
| `n` / `p` | 下一首 / 上一首 |
| `←` / `→` | 快退 / 快进 5 秒 |
| `v` | 切换可视方式：频谱柱 / 波形 / 环形频谱 |
| `c` | 循环配色预设（光谱/霓虹/森绿/火焰/冰蓝/云白） |
| `C` | 自定义颜色（输入 `#rrggbb`，Enter 确认） |
| `q` | 退出 |

- 可视化/配色设置自动保存到 `~/.config/tmus/config.json`（Windows：`%APPDATA%\tmus\config.json` 同目录结构），删除该文件即可恢复默认
- 界面：路径 → 状态（样式/配色）→ 文件列表 → 进度条 → 可视化区 → 帮助提示

---

## 环境足迹（不污染系统）

- 安装 = `~/.local/bin` 中最多两个文件（`tmus` + 需要时才有的 `ffmpeg`）；Windows 为 `%LOCALAPPDATA%\tmus`
- 设置文件仅在你改动可视化设置后创建
- 解码临时文件在系统临时目录、用后即删
- 无后台进程、无开机自启、无注册表改动、无需管理员权限
- 运行期间只读当前文件夹

---

## 卸载

推荐直接用发行包里的卸载脚本（会自动探测程序位置、PATH 痕迹、配置，一键清理）：

```bash
# macOS（在解压目录）
./uninstall.sh          # 交互确认；./uninstall.sh -y 全自动
# Windows（在解压目录）
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1    # 或加 -Yes 免确认
```

也可手动删除：

### macOS

```bash
rm -f ~/.local/bin/tmus ~/.local/bin/ffmpeg    # 删程序（Homebrew 装的 ffmpeg 请用 brew uninstall ffmpeg）
rm -rf ~/.config/tmus                          # 删设置
rm -f /tmp/tmus-*.wav                          # 清理解码残留
command -v tmus                                # 应无输出 = 卸载干净
```

如果安装脚本改过 PATH（`~/.zshrc` 里 `export PATH=...tmus` 那两行），一并删除。

### Windows

```powershell
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\tmus"   # 删程序目录
# 移除 PATH 中的 %LOCALAPPDATA%\tmus：
$p = [Environment]::GetEnvironmentVariable('Path','User')
[Environment]::SetEnvironmentVariable('Path', ($p -split ';' | Where-Object { $_ -ne "$env:LOCALAPPDATA\tmus" }) -join ';', 'User')
```

---

## 常见问题

- **找不到 ffmpeg**：把 `ffmpeg`/`ffmpeg.exe` 放到 `tmus` 同目录，或设 `TMUS_FFMPEG`，或安装后重跑安装脚本
- **macOS 首次打不开**：右键 `tmus` →「打开」（个人项目未公证）
- **Windows 提示 SmartScreen**：点「更多信息 → 仍要运行」
- **显示乱码/方块**：请用 Windows Terminal（推荐）；tmus 会自动切换 65001 代码页
- **>90 分钟的长音频**：当前版本整曲载入内存，超长约 90 分钟会提示不支持
