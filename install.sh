#!/bin/sh
# Install (or overwrite/upgrade) uva from GitHub Releases on Linux/macOS.
#
# Usage:
#   curl -fsSL https://github.com/hatsune-miku/uva/raw/main/install.sh | sh
#   curl -fsSL https://github.com/hatsune-miku/uva/raw/main/install.sh | sh -s -- --version v0.1.0
#   ./install.sh --install-dir ~/.local/bin --dry-run
#
# Env overrides: UVA_VERSION, UVA_INSTALL_DIR
set -eu

REPO="hatsune-miku/uva"
VERSION="${UVA_VERSION:-latest}"
INSTALL_DIR="${UVA_INSTALL_DIR:-$HOME/.local/bin}"
DRY_RUN=0

usage() {
    cat <<'EOF'
uva 安装脚本（Linux / macOS）

用法:
  install.sh [--version <tag>] [--install-dir <dir>] [--dry-run]

选项:
  --version <tag>       要安装的版本（默认 latest）；也可用 $UVA_VERSION
  --install-dir <dir>   安装目录（默认 ~/.local/bin）；也可用 $UVA_INSTALL_DIR
  --dry-run             只打印将要执行的操作，不下载或修改任何文件
  -h, --help            显示本帮助
EOF
}

step() { printf '==> %s\n' "$1"; }
warn() { printf '警告: %s\n' "$1" >&2; }
die() {
    printf 'uva: %s\n' "$1" >&2
    exit 1
}

# --- Parse arguments ------------------------------------------------------
while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="${2:?--version 需要一个值}"; shift 2 ;;
        --version=*) VERSION="${1#*=}"; shift ;;
        --install-dir) INSTALL_DIR="${2:?--install-dir 需要一个值}"; shift 2 ;;
        --install-dir=*) INSTALL_DIR="${1#*=}"; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "未知参数: $1（用 --help 查看用法）" ;;
    esac
done

# --- Resolve target -------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
    Linux) os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *) die "不支持的操作系统: $os（Windows 请用 install.ps1，其它平台请用 cargo 从源码安装）" ;;
esac
case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    arm64|aarch64) arch_part="aarch64" ;;
    *) die "不支持的架构: $arch（请用 cargo 从源码安装）" ;;
esac
target="${arch_part}-${os_part}"
asset="uva-${target}.tar.gz"

# --- Build download URLs --------------------------------------------------
if [ "$VERSION" = "latest" ]; then
    base="https://github.com/$REPO/releases/latest/download"
else
    base="https://github.com/$REPO/releases/download/$VERSION"
fi
archive_url="$base/$asset"
sha_url="$archive_url.sha256"

step "目标平台: $target"
step "版本: $VERSION"
step "下载地址: $archive_url"
step "安装目录: $INSTALL_DIR"

if [ "$DRY_RUN" -eq 1 ]; then
    step "(DryRun) 不会下载或修改任何文件。"
    exit 0
fi

# --- Helpers --------------------------------------------------------------
download() { # download <url> <dest>; returns nonzero on HTTP/network error
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        die "需要 curl 或 wget"
    fi
}

sha256_of() { # sha256_of <file> -> lowercase hex on stdout (empty if no tool)
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

# --- Download -------------------------------------------------------------
step "正在下载 $asset ..."
download "$archive_url" "$tmp/$asset" || die "下载失败：$archive_url（该版本/平台可能尚无产物）"

# --- Verify checksum (best-effort) ----------------------------------------
if download "$sha_url" "$tmp/$asset.sha256" 2>/dev/null; then
    expected="$(grep -oE '[0-9a-fA-F]{64}' "$tmp/$asset.sha256" | head -n1 | tr 'A-F' 'a-f')"
    actual="$(sha256_of "$tmp/$asset" | tr 'A-F' 'a-f')"
    if [ -n "$expected" ] && [ -n "$actual" ]; then
        [ "$expected" = "$actual" ] || die "校验和不匹配！期望 $expected，实际 $actual"
        step "校验和验证通过。"
    fi
else
    warn "未找到校验和文件，跳过验证。"
fi

# --- Extract --------------------------------------------------------------
step "正在解压 ..."
tar -xzf "$tmp/$asset" -C "$tmp"
bin="$(find "$tmp" -type f -name uva ! -name '*.tar.gz' | head -n1)"
[ -n "$bin" ] || die "压缩包中未找到 uva"

# --- Install (overwrite) --------------------------------------------------
mkdir -p "$INSTALL_DIR"
dest="$INSTALL_DIR/uva"
if ! install -m 0755 "$bin" "$dest" 2>/dev/null; then
    cp "$bin" "$dest"
    chmod 0755 "$dest"
fi
[ "$os" = "Darwin" ] && xattr -d com.apple.quarantine "$dest" 2>/dev/null || true
step "已安装到 $dest"

# --- Ensure InstallDir is on PATH -----------------------------------------
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;; # already on PATH
    *)
        case "$(basename "${SHELL:-sh}")" in
            zsh) profile="$HOME/.zshrc" ;;
            bash) [ "$os" = "Darwin" ] && profile="$HOME/.bash_profile" || profile="$HOME/.bashrc" ;;
            *) profile="$HOME/.profile" ;;
        esac
        if [ -f "$profile" ] && grep -qF "$INSTALL_DIR" "$profile" 2>/dev/null; then
            :
        else
            printf '\n# added by uva installer\nexport PATH="%s:$PATH"\n' "$INSTALL_DIR" >> "$profile"
            step "已把 $INSTALL_DIR 写入 PATH（$profile）。"
        fi
        step "重开终端，或执行: export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

# --- Report ---------------------------------------------------------------
printf '\n'
step "uva 安装成功！"
"$dest" --version || warn "无法运行 uva --version，但文件已就位：$dest"
