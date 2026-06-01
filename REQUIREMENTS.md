uv 是一款流行的 Python 环境管理器，而本项目 uva (uv advanced) 是一个 uv 工具的 wrapper，旨在让用户能够在 Python 中有一个如同 `yarn` 那般体验的工具，用户可以完全抛弃 venv 的概念，同时又享受到 venv 所解决的问题。uva 的受众是：

- 希望有个工具能直接搞定机器上的 python 环境、
- 不想用沉重而协议复杂的 anaconda 和 miniconda、
- 想要使用 uv，但是对 uv 独特的使用方式感到困惑、
- 不希望管太多、不想对依赖版本太过于严谨，只想尽快把脚本跑起来的用户。



技术栈：

- 同样基于 Rust

前置检查：

- 要求系统中有 uv

功能列表：

- `uva install`

  - 按以下优先级确定项目依赖，并运行相应的能够安装好依赖的 `uv` 命令：
    - `uv.lock`
    - `pyproject.toml` 
    - `requirements.txt` （尽管它是旧的、非标准的，但也早已成为事实标准，大量项目仍使用它，因此它值得成为一项默认）
    - （无依赖文件） - 停下并报错：当前目录不是一个 Python 项目

- `uva`

  - 无参运行 uva 时，等同于 `uva install`

- `uva run [filename]`

  - 生成并运行：运行对应脚本的 uv 命令

  - `filename` 为空时，按以下优先级取默认：

    - （唯一的那个以 .py 结尾的文件）
    - src/（唯一的那个以 .py 结尾的文件）

    - `main.py`
    - `src/main.py`
    - 报错：未指定源文件。已尝试过：...

  - 不强求 filename 一定是 py 后缀

- `uva start [filename]`

  - 等同于 `uva run [filename]`

- `uva [filename]`

  - 有参运行 uva 时，如果 filename 是一个确实存在的文件，那么等同于：

    - `uva run [filename]`

    否则显示 usage。

- `uva how-to-install-uv`

  - 直接输出字符串: "https://docs.astral.sh/uv/getting-started/installation/"



核心特性：将用户的心智负担从 venv 中解放出来

- 对用户完全抹除 venv 概念
- 必要时（uv 强制时），自动使用 `uv venv` 创建无名环境。例如：
  - `uva install` 所生成并运行的 uv 命令，须自动搞定一个环境



关于 Python 版本选择

- uva 不提供切换 Python 版本的功能 - 它总是使用 uv 目前激活着的 Python 版本。用户若要切换，告知用户使用 uv 来切换。

