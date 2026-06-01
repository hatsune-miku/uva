<#
.SYNOPSIS
    Install (or overwrite/upgrade) uva from GitHub Releases.

.DESCRIPTION
    Downloads the latest (or a specified) uva Windows release from
    https://github.com/hatsune-miku/uva/releases, verifies its SHA-256,
    and installs uva.exe into a user-local bin directory on PATH —
    overwriting any existing copy.

.PARAMETER Version
    Release tag to install (e.g. "v0.1.0"). Defaults to "latest".
    Also reads $env:UVA_VERSION.

.PARAMETER InstallDir
    Where to place uva.exe. Defaults to "$env:LOCALAPPDATA\uva\bin".
    Also reads $env:UVA_INSTALL_DIR.

.PARAMETER DryRun
    Print what would happen (resolved target, URLs, install dir) and exit
    without downloading or modifying anything.

.EXAMPLE
    irm https://github.com/hatsune-miku/uva/raw/master/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version v0.1.0 -InstallDir C:\tools\uva
#>
[CmdletBinding()]
param(
    [string]$Version = $(if ($env:UVA_VERSION) { $env:UVA_VERSION } else { 'latest' }),
    [string]$InstallDir = $(if ($env:UVA_INSTALL_DIR) { $env:UVA_INSTALL_DIR } else { "$env:LOCALAPPDATA\uva\bin" }),
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue' # faster Invoke-WebRequest downloads

$Repo = 'hatsune-miku/uva'

function Write-Step($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }

# --- Resolve target -------------------------------------------------------
# Only x86_64 Windows binaries are published; they also run on ARM64 Windows
# via emulation, so x86_64-pc-windows-msvc is always the right asset.
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq 'ARM64') {
    Write-Host "注意: Windows ARM64 将通过模拟运行 x86_64 版本。" -ForegroundColor Yellow
}
$target = 'x86_64-pc-windows-msvc'
$asset = "uva-$target.zip"

# --- Build download URLs --------------------------------------------------
if ($Version -eq 'latest') {
    $base = "https://github.com/$Repo/releases/latest/download"
} else {
    $base = "https://github.com/$Repo/releases/download/$Version"
}
$archiveUrl = "$base/$asset"
$shaUrl = "$archiveUrl.sha256"

Write-Step "目标平台: $target"
Write-Step "版本: $Version"
Write-Step "下载地址: $archiveUrl"
Write-Step "安装目录: $InstallDir"

if ($DryRun) {
    Write-Host "(DryRun) 不会下载或修改任何文件。" -ForegroundColor Yellow
    return
}

# --- Enable TLS 1.2 for Windows PowerShell 5.1 ----------------------------
try {
    [Net.ServicePointManager]::SecurityProtocol = `
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("uva-install-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -Force | Out-Null
try {
    $archivePath = Join-Path $work $asset

    Write-Step "正在下载 $asset ..."
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath -UseBasicParsing

    # --- Verify checksum (best-effort: warn but continue if absent) -------
    try {
        $shaText = (Invoke-WebRequest -Uri $shaUrl -UseBasicParsing).Content
        $expected = ([regex]::Match($shaText, '[0-9a-fA-F]{64}')).Value.ToLower()
        if ($expected) {
            $actual = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLower()
            if ($actual -ne $expected) {
                throw "校验和不匹配！期望 $expected，实际 $actual"
            }
            Write-Step "校验和验证通过。"
        }
    } catch [System.Net.WebException] {
        Write-Host "警告: 未找到校验和文件，跳过验证。" -ForegroundColor Yellow
    }

    # --- Extract ----------------------------------------------------------
    Write-Step "正在解压 ..."
    $extractDir = Join-Path $work 'extract'
    Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force
    $exe = Get-ChildItem -Path $extractDir -Filter 'uva.exe' -Recurse | Select-Object -First 1
    if (-not $exe) { throw "压缩包中未找到 uva.exe" }

    # --- Install (overwrite) ---------------------------------------------
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $dest = Join-Path $InstallDir 'uva.exe'
    Copy-Item -Path $exe.FullName -Destination $dest -Force
    Write-Step "已安装到 $dest"

    # --- Ensure InstallDir is on the user PATH ----------------------------
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $parts = @()
    if ($userPath) { $parts = $userPath -split ';' | Where-Object { $_ -ne '' } }
    if ($parts -notcontains $InstallDir) {
        $newPath = (@($parts) + $InstallDir) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Step "已将 $InstallDir 添加到用户 PATH（新开终端生效）。"
    }
    # Update the current session too, so `uva` works right away.
    if (($env:Path -split ';') -notcontains $InstallDir) {
        $env:Path = "$env:Path;$InstallDir"
    }

    # --- Report -----------------------------------------------------------
    Write-Host ""
    Write-Host "uva 安装成功！" -ForegroundColor Green
    try {
        & $dest --version
    } catch {
        Write-Host "（无法运行 uva --version，但文件已就位：$dest）" -ForegroundColor Yellow
    }
    Write-Host "如果当前终端找不到 uva，请重开一个终端窗口。" -ForegroundColor DarkGray
}
finally {
    Remove-Item -Path $work -Recurse -Force -ErrorAction SilentlyContinue
}
