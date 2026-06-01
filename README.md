# uva — uv advanced

让 Python 拥有 [`yarn`](https://yarnpkg.com/) 那般的体验。

`uva` 是 [`uv`](https://docs.astral.sh/uv/) 的一层轻量封装：你可以完全抛弃
`venv` 的概念，却依然享受到它带来的隔离。它适合这样的人：

- 想用一个工具直接搞定机器上的 Python 环境；
- 觉得 anaconda / miniconda 太重、协议太复杂；
- 想用 uv，却被它独特的使用方式弄糊涂；
- 不想在依赖版本上太较真，只想尽快把脚本跑起来。

## 前置要求

系统中需要有 `uv`。还没装？运行：

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

| 命令 | 作用 | 实际运行的 uv 命令 |
| --- | --- | --- |
| `uva` | 安装依赖（等同于 `uva install`） | 见下 |
| `uva install` | 按优先级安装当前项目依赖 | `uv.lock` / `pyproject.toml` → `uv sync`；只有 `requirements.txt` → `uv venv`（若无 `.venv`）+ `uv pip install -r requirements.txt`；都没有 → 报错 |
| `uva run [文件] [参数...]` | 运行脚本 | `uv run <文件> [参数...]` |
| `uva start [文件] [参数...]` | 等同于 `uva run` | 同上 |
| `uva add <包>... [--save]` | 安装包到当前环境 | 见下方“add / remove” |
| `uva remove <包>... [--save]` | 从当前环境卸载包 | 见下方“add / remove” |
| `uva <文件> [参数...]` | 若文件存在，等同于 `uva run <文件>`；否则显示帮助 | 同上 |
| `uva how-to-install-uv` | 输出 uv 安装地址 | 无（无需 uv） |
| `uva --help` / `uva --version` | 帮助 / 版本 | 无 |

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

```
uva add requests flask          # 空格分隔
uva add requests,flask,numpy    # 逗号分隔
```

默认只改动当前环境，**不**碰任何依赖文件。加上 `--save` 才会把改动持久化：

| 场景 | `uva add <包>...` | `uva add <包>... --save` |
| --- | --- | --- |
| 有 `pyproject.toml` | 确保 `.venv` → `uv pip install <包>...` | `uv add <包>...`（写入 pyproject.toml 并更新 lock） |
| 否则（有/无 `requirements.txt`） | 确保 `.venv` → `uv pip install <包>...` | 同左，并把包名追加进 `requirements.txt`（不存在则创建、已存在则去重） |

| 场景 | `uva remove <包>...` | `uva remove <包>... --save` |
| --- | --- | --- |
| 有 `pyproject.toml` | `uv pip uninstall <包>...` | `uv remove <包>...`（从 pyproject.toml 移除） |
| 否则 | `uv pip uninstall <包>...` | 先从 `requirements.txt` 删除对应行，再尽力卸载（`.venv` 不存在时跳过卸载） |

`--save` 写入 `requirements.txt` 时按 [PEP 503](https://peps.python.org/pep-0503/)
规范化包名做去重与匹配（`Flask`、`flask`、`flask==2.0` 视为同一个包），并保留
文件中的注释与空行。

## 设计原则

- **抹除 venv 概念**：你永远不必给环境起名字。`uv sync` / `uv run` 会隐式管理
  `.venv`；只有 `requirements.txt` 这条路径会显式地 `uv venv` 一下——但你看不到。
- **透明**：执行前，`uva` 会把对应的 `uv` 命令以 `$ uv ...` 的形式打印到
  stderr，方便你了解底层在做什么。
- **不碰 Python 版本**：`uva` 始终使用 uv 当前激活的 Python 版本。要切换版本，
  请直接使用 `uv`。
- **退出码**：用法/参数错误返回 `2`；“不是 Python 项目”/“未指定源文件”返回 `1`；
  其余情况原样透传所封装的 `uv` 命令的退出码。

## 开发

```
cargo test                 # 单元测试 + 冒烟测试（不真正调用 uv 下载）
cargo test -- --ignored    # 额外跑真正调用 uv 的端到端测试
cargo clippy --all-targets -- -D warnings
```
