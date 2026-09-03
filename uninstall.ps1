# ============================================================
#  TMus 卸载脚本（Windows）——自动探测并清理
#
#  用法：右键 → 使用 PowerShell 运行（或 powershell -ExecutionPolicy Bypass -File .\uninstall.ps1）
#       追加参数 -Yes 可免确认： .\uninstall.ps1 -Yes
#
#  自动处理：
#   - 删除 %LOCALAPPDATA%\tmus（程序 + 需要时才装入的 ffmpeg.exe）
#   - 自动移除用户 PATH 中指向该目录的条目（只删这一项，其它不动）
#   - 删除设置目录 %APPDATA%\tmus
# ============================================================
param([switch]$Yes)

$ErrorActionPreference = 'Stop'
$dest = Join-Path $env:LOCALAPPDATA 'tmus'

Write-Host '==> TMus 卸载开始'

# ---------- 1. 程序目录 ----------
if (Test-Path $dest) {
    if ($Yes -or (Read-Host "删除程序目录 $dest ? [y/N]") -match '^[yY]') {
        Remove-Item -Recurse -Force $dest
        Write-Host "    ✓ 已删除 $dest"
    } else {
        Write-Host '    已取消删除程序目录'
    }
} else {
    Write-Host '    · 未找到程序目录（可能未安装）'
}

# ---------- 2. PATH 中指向该目录的条目 ----------
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -like "*$dest*") {
    $newPath = ($userPath -split ';' | Where-Object { $_ -ne $dest }) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Write-Host "    ✓ 已从用户 PATH 移除 $dest"
} else {
    Write-Host '    · 用户 PATH 中没有该目录'
}

# ---------- 3. 设置 ----------
$cfg = Join-Path $env:APPDATA 'tmus'
if (Test-Path $cfg) {
    Remove-Item -Recurse -Force $cfg
    Write-Host "    ✓ 已删除设置 $cfg"
}

# ---------- 4. 结果 ----------
if (Get-Command tmus -ErrorAction SilentlyContinue) {
    Write-Host "  ! tmus 仍可用：$((Get-Command tmus).Source)（可能被手动装到其它位置）" -ForegroundColor Yellow
} else {
    Write-Host '✅ 卸载完成：tmus 已不可用，PATH 与设置已清理' -ForegroundColor Green
}
