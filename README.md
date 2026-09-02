# tmus — 终端音乐播放器

纯终端（TUI）音乐播放器：在任意文件夹里运行，浏览当前目录与子目录、
播放本地音频（mp3 / flac / m4a / wav …，经 **ffmpeg** 解码，兼容 32-bit FLAC），
播放时用符号块实时显示频谱。无桌面 GUI、无 Electron/Tauri 依赖。

## 安装

前置：**ffmpeg**（唯一外部依赖）

```bash
brew install ffmpeg
```

安装命令：

```bash
cd /Users/juzhaonan/WORK/PROJECT/MusicPlayer
cargo build --release
install -m 755 target/release/tmus ~/.local/bin/   # ~/.local/bin 需在 PATH
# 或直接运行一键脚本
./install.sh
```

`.zshrc` / `.bashrc` 里已配置别名（若未配置可自行添加）：

```bash
alias music="tmus"
```

## 使用

```bash
cd ~/Music/周杰伦        # 进入任意音乐文件夹
music                    # 播放当前文件夹（tmus 也行，或 tmus ~/Music 指定目录）
```

**快捷键**

| 按键 | 功能 |
| --- | --- |
| `↑/↓` 或 `j`/`k` | 移动选择 |
| `Enter` | 播放选中曲目 / 进入目录 |
| `Backspace` 或 `h` | 返回上级目录 |
| `Space` | 播放 / 暂停 |
| `n` / `p` | 下一首 / 上一首 |
| `←` / `→` | 快退 / 快进 5 秒 |
| `g` / `G` 或 `Home`/`End` | 列表首 / 末 |
| `x` | 停止 |
| `r` | 刷新列表 |
| `v` | 切换可视方式：频谱柱 / 波形 / 环形频谱 |
| `c` | 循环配色预设（光谱/霓虹/森绿/火焰/冰蓝/云白） |
| `C` | 自定义颜色：输入 `#rrggbb`（如 `#00ff88`） |
| `q` / `Esc` | 退出 |

**可视化设置**：三种可视方式 + 六套预设配色 + 任意自定义颜色，改动即保存到
`~/.config/tmus/config.json`（也可手工编辑该文件）。自定义色会自动生成
从暗到亮的同色系渐变；波形/文字等强调色取配色最亮端。

界面布局：顶部当前路径 → 播放状态（含当前 样式/配色）→ 文件列表
（`D` 目录、`♪` 音频、`●` 正在播放）→ 进度条 → **可视化区** → 两行帮助提示。

## 技术架构

```
┌ 界面：ratatui + crossterm（列表/进度/频谱，~30fps）────────┐
│                                                            │
│  ┌ 音频：cpal 常驻输出流 ─ 回调按播放位置写声卡 ┐           │
│  └ 样本源：整曲 i16 立体声样本（Arc 共享）◄───────── 频谱单点DFT
│                                                            │
│  ffmpeg（子进程）→ 设备采样率 / 立体声 16bit WAV → 解析进内存 │
└────────────────────────────────────────────────────────────┘
```

- `src/audio.rs` — 解码（ffmpeg → wav16 → 内存样本）、cpal 播放引擎、
  频谱分析（每频段单点 DFT + 峰值平滑）
- `src/main.rs` — TUI 界面与交互
- `examples/smoke.rs` — 音频设备冒烟（`cargo run --example smoke`）
- `examples/decode.rs` — 解码冒烟（`cargo run --example decode -- 文件`）

## 说明与边界

- 播放/可视化共用同一份解码样本，符号频谱与声音严格同步；
- 整曲载入内存（上限约 90 分钟立体声，超出会提示），换歌/快进都即时；
- 需在真实终端（TTY）运行，终端需支持 UTF-8 与等宽字体；
- 单/双声道自动适配设备；多声道由 ffmpeg 自动降混。

## 更新重装

```bash
cd /Users/juzhaonan/WORK/PROJECT/MusicPlayer && ./install.sh
```

## 发布与部署（Windows / macOS）

程序会自动优先使用**同目录下的 ffmpeg**，所以发行包里带上静态 ffmpeg 后，
普通用户**零依赖**：解压即用，无需装任何东西。

### 给用户的最小分发包

```
tmus-macos-arm64.zip  /  tmus-macos-x86_64.zip      # macOS（Apple 芯片 / Intel）
tmus-windows-x86_64.zip                              # Windows x64
└── tmus(.exe)        # 主程序
    ffmpeg(.exe)      # 静态 ffmpeg（与程序同目录，自动被找到）
    README.md
    install.ps1       # Windows 可选：加入 PATH
```

- **macOS 用户**：解压后终端运行 `./tmus`，或在文件夹里执行 `install.sh`-风格命令放入 PATH
- **Windows 用户**：解压后直接双击 `tmus.exe`，或在 Windows Terminal 里运行；
  想全局可用就运行 `install.ps1`（加入用户 PATH）；tmus 启动时会自动把控制台切到 UTF-8
  （建议使用 Windows Terminal 以获得最佳字符显示）

### 一键发布（GitHub Actions）

仓库已带 `.github/workflows/release.yml`，两种触发方式：

1. **手动**：GitHub 页面 → Actions → “build-release” → Run workflow
2. **打标签自动发版**：`git tag v0.1.0 && git push origin v0.1.0`

会自动在 3 个平台（macOS Apple Silicon / macOS Intel / Windows x64）编译，
下载静态 ffmpeg 打包成 zip，并附加到对应的 GitHub Release。

### 本地打包（macOS）

```bash
./pack.sh        # 产物 dist/tmus-macos-<架构>.zip（含静态 ffmpeg）
```

### 常见部署问题

- 程序找不到 ffmpeg：把 `ffmpeg`/`ffmpeg.exe` 与 `tmus`/`tmus.exe` 放同一目录即可；
  也可设环境变量 `TMUS_FFMPEG=/path/to/ffmpeg`
- Windows 上汉字/方块显示异常：请用 Windows Terminal（tmus 已自动切换 65001 代码页）
- ffmpeg 版本：发行版内置静态 ffmpeg 6.x，功能足够；32-bit FLAC 等均支持
- 未签名提示：macOS 首次运行下载的程序若被 Gatekeeper 拦截，右键 tmus →“打开”；
  Windows SmartScreen 提示时点“更多信息 → 仍要运行”（个人项目暂未做签名/公证）
