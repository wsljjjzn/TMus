# ============================================================
#  TMus 安装脚本（Windows）
#
#  用法（在解压目录）：
#    powershell -ExecutionPolicy Bypass -File .\install.ps1
#   或右键 → 使用 PowerShell 运行
#
#  ffmpeg 自动检测：
#   - 系统已有 ffmpeg（PATH 中）→ 直接使用，不复制内置 ffmpeg.exe
#   - 系统没有 → 自动安装发行包内置的静态 ffmpeg.exe
# ============================================================

$ErrorActionPreference = 'Stop'
$src  = $PSScriptRoot
$dest = Join-Path $env:LOCALAPPDATA 'tmus'

Write-Host '==> TMus 安装开始'

# ---------- 1. 定位 tmus.exe ----------
$tmus = Join-Path $src 'tmus.exe'
if (-not (Test-Path $tmus)) {
    Write-Host '✗ 未找到 tmus.exe，请先解压完整发行包（含 tmus.exe）再运行本脚本。' -ForegroundColor Red
    exit 1
}
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Force $tmus (Join-Path $dest 'tmus.exe')
Write-Host "    ✓ tmus.exe 已安装到 $dest"

# ---------- 2. ffmpeg：先用系统 PATH 的，没有再用内置 ----------
$sysFfmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if ($sysFfmpeg) {
    Write-Host "    ✓ 检测到系统 ffmpeg: $($sysFfmpeg.Source)"
    Write-Host '      → 直接使用系统版本（不复制内置文件）'
} elseif (Test-Path (Join-Path $src 'ffmpeg.exe')) {
    Copy-Item -Force (Join-Path $src 'ffmpeg.exe') (Join-Path $dest 'ffmpeg.exe')
    Write-Host "    ✓ 未检测到系统 ffmpeg，已安装内置静态版到 $dest\ffmpeg.exe"
} else {
    Write-Host '  ! 未检测到系统 ffmpeg，且目录内也没有内置 ffmpeg.exe。' -ForegroundColor Yellow
    Write-Host '    请安装：winget install ffmpeg，或把发行包里的 ffmpeg.exe 与本脚本放同一目录后重试。'
    exit 1
}

# ---------- 3. PATH ----------
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$dest*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dest", 'User')
    Write-Host "    ✓ 已把 $dest 加入用户 PATH（新开终端生效）"
} else {
    Write-Host "    ✓ $dest 已在用户 PATH 中"
}

Write-Host ''
Write-Host '✅ 安装完成！' -ForegroundColor Green
Write-Host "   程序目录: $dest"
Write-Host '   使用方法：打开“Windows Terminal”，进入任意音乐文件夹后输入：'
Write-Host '       tmus'
