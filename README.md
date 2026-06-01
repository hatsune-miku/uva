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

```
cargo install --path .
# 或
cargo build --release   # 产物在 target/release/uva
```

## 命令

| 命令 | 类比 |
| --- | --- | 
| `uva` | `yarn` |
| `uva install` | `yarn add` |
| `uva run [文件] [参数...]` | `yarn start ...` |
| `uva start [文件] [参数...]` | 与 `uva run` 完全一样 |
| `uva <文件> [参数...]` | 同上，与 `uva run` 完全一样 |
| `uva add <包>... [--save]` | `yarn add <包>... [--save]` |
| `uva remove <包>... [--save]` | `yarn remove <包>... [--save]` |
| `uva cn` | 一键切清华源用。 |
| `uva unset-base-url` | 还原官方源。 |

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

Windows、Linux、macOS 均受支持，CI 在 Windows 与 Linux 上同时构建与测试。

- 全局 `uv.toml` 的位置按 uv 自身的规则解析：Windows 为 `%APPDATA%\uv\uv.toml`，
  macOS / Linux 为 `$XDG_CONFIG_HOME/uv/uv.toml`（否则 `~/.config/uv/uv.toml`）；
  若设置了 `UV_CONFIG_FILE`，则以它为准。**macOS 与 Linux 走同一套逻辑，无需特殊处理。**
- 改写 `requirements.txt` / `uv.toml` 时会保留文件原有的换行风格（CRLF 或 LF）。

## 开发

```
cargo test                 # 单元测试 + 冒烟测试（不真正调用 uv 下载）
cargo test -- --ignored    # 额外跑真正调用 uv 的端到端测试
cargo clippy --all-targets -- -D warnings
```
