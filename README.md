# uva — uv automations

像使用 `yarn` 那样使用 Python 项目。

- 忘记 `venv` 概念
- 忘记 activate 命令
- 一键搞定依赖、让项目赶快跑起来
- 一键全局切清华源

## 前置要求

系统中需要先装有 `uv`。还没装？运行：

```
uva how-to-install-uv
```

它会输出安装地址：<https://docs.astral.sh/uv/getting-started/installation/>

## 安装 uva

**Windows 一键安装（从 GitHub Releases 覆盖安装最新版）：**

```powershell
irm https://github.com/hatsune-miku/uva/raw/main/install.ps1 | iex
```

会下载最新发行版、校验 SHA-256，并把 `uva.exe` 装进 `%LOCALAPPDATA%\uva\bin`
（自动加入用户 PATH，已存在则直接覆盖升级）。想装指定版本或自定义目录：

```powershell
# 先下载脚本再带参运行
irm https://github.com/hatsune-miku/uva/raw/main/install.ps1 -OutFile install.ps1
.\install.ps1 -Version v0.1.0 -InstallDir C:\tools\uva
.\install.ps1 -DryRun        # 只打印将要执行的操作，不下载
```

**macOS / Linux 一键安装：**

```sh
curl -fsSL https://github.com/hatsune-miku/uva/raw/main/install.sh | sh
```

会自动识别平台（x86_64 / arm64）、下载最新发行版、校验 SHA-256，并把 `uva` 装进
`~/.local/bin`（已存在则覆盖升级；不在 PATH 时写入对应 shell 的 profile）。带参运行：

```sh
# 指定版本 / 目录
curl -fsSL https://github.com/hatsune-miku/uva/raw/main/install.sh | sh -s -- --version v0.1.0
UVA_INSTALL_DIR=~/bin sh install.sh           # 也支持环境变量
sh install.sh --dry-run                       # 只打印将要执行的操作
```

**从源码构建（任意平台）：**

```
cargo install --path .
# 或
cargo build --release   # 产物在 target/release/uva
```

## 命令

| 命令 | 类比 | 作用 |
| --- | --- | --- | 
| `uva` | `yarn` (`yarn install`) | 装依赖 |
| `uva run [文件] [参数...]` | `yarn start ...` | 运行 |
| `uva start [文件] [参数...]` | 与 `uva run` 完全一样 | 运行 |
| `uva repl` | `node` | 启动 Python REPL（项目环境优先，否则全局） |
| `uva <文件> [参数...]` | 同上，与 `uva run` 完全一样 | 运行 |
| `uva add <包>... [-g\|--save]` | `yarn add <包>... [-g]` | 装包（`-g` 装到全局） |
| `uva remove <包>... [-g\|--save]` | `yarn remove <包>... [-g]` | 卸包（`-g` 从全局卸） |
| `uva cn` | - | 一键全局切清华源 |
| `uva unset-base-url` | - | 一键全局恢复官方源 |

### `uva run` 的默认入口查找

不传文件名时，按以下优先级查找（命中即用）：

1. 当前目录下**唯一**的 `.py` 文件；
2. `src/` 下**唯一**的 `.py` 文件；
3. `main.py`；
4. `src/main.py`；
5. 都没有则报错，并列出尝试过的位置。

文件名不强求以 `.py` 结尾。文件名后面的参数会原样转发给脚本，例如
`uva run app.py --port 8000` 会运行 `uv run app.py --port 8000`。

### `uva add` / `uva remove`

往当前环境里装包或卸包。可以一次指定多个，包名用**空格或逗号**分隔：

```bash
# 只装依赖、不动依赖文件
uva add requests flask

# 装依赖并写入依赖文件
uva add requests flask --save 
```

### `uva repl`（按上下文选环境，类似 `node`）

`uva repl` 会按你所在的位置选择 Python 环境，就像在项目目录里跑 `node` 会用本地
`node_modules` 一样：

- **在项目里**（有 `pyproject.toml` / `uv.lock`）→ 跑 `uv run python`，REPL 能直接
  `import` 该项目的依赖（必要时自动同步）。
- **只有本地 `.venv`**（例如 requirements.txt 项目，已 `uva install` 过）→ 用该 `.venv`。
- **不在任何项目里** → 用 uva 的**全局环境**（见下）。

### `uva add -g`（全局环境）

uva 维护一个**全局 Python 环境**（venv）。`uva add -g` 把包装进它，在项目外运行
`uva repl` 时即可直接 `import`：

```bash
cd ~                       # 不在任何项目里
uva add -g requests        # 装到全局环境
uva repl                   # 全局 REPL
>>> import requests        # 直接可用
uva remove -g requests     # 从全局环境卸载
```

全局环境位置：Windows 为 `%LOCALAPPDATA%\uva\venv`，macOS/Linux 为
`$XDG_DATA_HOME/uva/venv`（默认 `~/.local/share/uva/venv`），首次使用时自动创建。
`-g` 优先于 `--save`，且不依赖当前目录是否为项目。

> 说明：这里的 `-g` 是「全局**可导入**的库」，比 npm 的 `-g`（只装**命令行工具**、
> 不可 `require`）更贴近 Python/pip 的习惯；而 `uva repl` 选环境的方式则与 `node` 一致。

### `uva cn`：一键全局切清华源

```
uva cn
```

把下面这段写入**全局** `uv.toml`（`%APPDATA%\uv\uv.toml`，或 macOS/Linux 的
`~/.config/uv/uv.toml`），让 uv 默认走[清华 TUNA 镜像](https://mirrors.tuna.tsinghua.edu.cn/help/pypi/)：

```toml
[[index]]
url = "https://pypi.tuna.tsinghua.edu.cn/simple"
default = true
```

幂等可重复执行；会替换已有的 `[[index]]`，但保留 `uv.toml` 里的其它设置。想撤销时：

```
uva unset-base-url      # 清除全局 uv.toml 的 [[index]] 设置
```

## 设计原则

- **透明**：执行前，`uva` 会把对应的 `uv` 命令以 `$ uv ...` 的形式打印到
  stderr，方便你了解底层在做什么。
- **不碰 Python 版本**：`uva` 始终使用 uv 当前激活的 Python 版本。要切换版本，
  请直接使用 `uv`。
- **退出码**：用法/参数错误返回 `2`；“不是 Python 项目”/“未指定源文件”返回 `1`；
  其余情况原样透传所封装的 `uv` 命令的退出码。

## 平台支持

Windows、Linux、macOS 均受支持。CI（`.github/workflows/ci.yml`）在
`windows-latest`、`ubuntu-latest`、`macos-latest` 三个平台上同时构建、lint 与测试。

- 全局 `uv.toml` 的位置按 uv 自身的规则解析：Windows 为 `%APPDATA%\uv\uv.toml`，
  macOS / Linux 为 `$XDG_CONFIG_HOME/uv/uv.toml`（否则 `~/.config/uv/uv.toml`）；
  若设置了 `UV_CONFIG_FILE`，则以它为准。**macOS 与 Linux 走同一套逻辑，无需特殊处理。**
- 改写 `requirements.txt` / `uv.toml` 时会保留文件原有的换行风格（CRLF 或 LF）。

## 发布（GitHub Releases）

发布完全自动：**CI 自己创建 tag 和 Release**，无需手动打标签。

`.github/workflows/release.yml` 在每次推送到 `main` 时运行——读取 `Cargo.toml` 里的
版本号，若对应的 `v<版本>` 标签尚不存在，就在各原生 runner 上构建产物，并自动创建该
标签与 Release（连同 `.sha256` 校验文件）：

| 平台 | target | 产物 |
| --- | --- | --- |
| Windows | `x86_64-pc-windows-msvc` | `uva-x86_64-pc-windows-msvc.zip` |
| Linux (x86_64) | `x86_64-unknown-linux-gnu` | `uva-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `aarch64-unknown-linux-gnu` | `uva-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `aarch64-apple-darwin` | `uva-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `x86_64-apple-darwin` | `uva-x86_64-apple-darwin.tar.gz` |

**发布一个新版本** = 在 `Cargo.toml` 里把 `version` 改大，然后推到 `main`：

```toml
# Cargo.toml
version = "0.1.1"   # 改这一行
```

推送后 CI 会自动建出 `v0.1.1` 标签和 Release。版本没变的推送是幂等的（标签已存在则跳过，
不会重复发布）。也可在 *Actions → Release → Run workflow* 手动触发，并可选填一个版本号覆盖。

资产名不含版本号，因此 `install.ps1` 始终可用
`releases/latest/download/uva-x86_64-pc-windows-msvc.zip` 这一固定地址拉取最新版。

## 开发

```
cargo test                 # 单元测试 + 冒烟测试（不真正调用 uv 下载）
cargo test -- --ignored    # 额外跑真正调用 uv 的端到端测试
cargo clippy --all-targets -- -D warnings
```
