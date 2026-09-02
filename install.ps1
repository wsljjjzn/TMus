# Windows 一键安装：把 tmus + ffmpeg 装到 %LOCALAPPDATA%\tmus 并加入 PATH
# 用法：解压 zip 后，右键 install.ps1 → “使用 PowerShell 运行”，
#       或在该目录打开终端执行：  powershell -ExecutionPolicy Bypass -File .\install.ps1

$ErrorActionPreference = 'Stop'
$src  = $PSScriptRoot
$dest = Join-Path $env:LOCALAPPDATA 'tmus'

if (-not (Test-Path (Join-Path $src 'tmus.exe'))) {
    Write-Host '未找到 tmus.exe，请先解压完整压缩包再运行本脚本。' -ForegroundColor Red
    exit 1
}

New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Force (Join-Path $src 'tmus.exe')  (Join-Path $dest 'tmus.exe')
Copy-Item -Force (Join-Path $src 'ffmpeg.exe') (Join-Path $dest 'ffmpeg.exe')

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$dest*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dest", 'User')
    Write-Host "已把 $dest 加入用户 PATH" -ForegroundColor Green
}

Write-Host ''
Write-Host '✅ 安装完成！' -ForegroundColor Green
Write-Host "   程序目录: $dest"
Write-Host '   请打开新的“终端 / Windows Terminal”，进入音乐文件夹运行：'
Write-Host '       tmus'
