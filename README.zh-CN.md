![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)

# llama-cpp-launcher

一个终端用户界面（TUI）工具，用于启动和管理 [llama.cpp](https://github.com/ggml-org/llama.cpp) 模型服务器。自动检测硬件、计算最优启动参数，并提供实时服务器控制台日志——全部在终端中完成。

## 功能特性

- **硬件自动检测** — 启动时自动检测 CPU 核心数、内存和 GPU（NVIDIA 通过 NVML）
- **最优参数计算** — 根据硬件配置和模型元数据，自动计算最佳参数（上下文大小、批处理大小、GPU 层数、线程数等）
- **模型浏览器** — 扫描模型目录中的 GGUF 文件，按提供商分组显示（如 Qwen、Meta、Mistral）
- **交互式参数调整** — 启动服务器前微调各项启动参数
- **预设管理** — 保存和切换参数预设方案（如"高吞吐量"、"低内存"）
- **实时服务器日志** — 直接在 TUI 中查看 llama.cpp 的 stderr 输出，支持滚动浏览
- **配置编辑** — 在 TUI 内直接编辑模型目录、二进制路径和调试设置

## 使用截图

```
┌────────────── 选择模型 ────────────────┐
│  Qwen ──── 3 个模型                      │
│    Qwen2.5-0.5B-Instruct-Q8_0  │ 0.5B参数 │ 500 MB │
│    Qwen2.5-1.5B-Instruct-Q8_0  │ 1.5B参数 │ 1.2 GB │
│  Meta ──── 2 个模型                      │
│    Llama-3.2-1B-Instruct-Q8_0  │ 1.0B参数 │ 1.1 GB │
└──────────────────────────────────────────┘
```

```
┌────────── 启动配置 ────────────────────┐
│  模型: /models/qwen/qwen2.5-q8_0.gguf    │
│  参数量: 0.5B  上下文: 32K                │
│                                          │
│  可调参数:                                │
│    GPU 层数     99                        │
│    上下文大小   8192                      │
│    批处理大小   2048                      │
│    Flash Attn   auto                      │
│    ...                                    │
│                                          │
│  启动命令:                                │
│    llama-server --host 127.0.0.1 --port.. │
└──────────────────────────────────────────┘
```

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（edition 2021，最低版本 1.75+）
- [llama.cpp](https://github.com/ggml-org/llama.cpp/releases) 服务器二进制文件（`llama-server`），需在系统 PATH 中或通过 `llama_bin_path` 配置
- GGUF 格式的模型文件

## 安装

### 从源码编译

```bash
git clone https://github.com/pingziii/llama-cpp-launcher.git
cd llama-cpp-launcher
cargo build --release
```

编译后的二进制文件位于：
- **Windows**: `target/release/llama-cpp-launcher.exe`
- **Linux/macOS**: `target/release/llama-cpp-launcher`

### 安装后配置

1. 将 GGUF 模型文件放入 `~/models/` 目录（可在配置中修改）
2. 确保 `llama-server` 在系统 PATH 中，或在配置中设置 `llama_bin_path`
3. 运行二进制文件

## 使用说明

1. **启动**: 在终端中运行 `llama-cpp-launcher`
2. **硬件检测**: 工具自动检测系统规格
3. **选择模型**: 浏览并选择一个 GGUF 模型
4. **检查参数**: 查看自动计算的设置，按 `a` 进行调整
5. **启动**: 按 `Enter` 启动 llama.cpp 服务器
6. **监控**: 查看服务器信息，或按 `l` 查看实时控制台日志
7. **停止**: 按 `s` 或 `Esc` 停止服务器并返回

### 快捷键

| 界面           | 按键         | 功能                            |
|----------------|-------------|----------------------------------|
| 硬件检测       | `Enter`     | 继续到模型选择                    |
| 模型选择       | `↑/↓`       | 在组内浏览模型                     |
|                | `←/→`       | 切换模型分组                       |
|                | `Enter`     | 选择模型                           |
|                | `c`         | 打开配置编辑器                     |
| 启动配置       | `Enter`     | 启动服务器                         |
|                | `a`         | 进入参数调整模式                   |
|                | `p`         | 切换预设方案                       |
| (调整模式)     | `↑/↓`       | 选择参数                           |
|                | `←/→`       | 调整数值（Shift = 10 倍步进）      |
| 聊天/日志      | `l`         | 切换显示服务器控制台日志            |
|                | `↑/↓`       | 滚动日志（在日志视图中）            |
|                | `s` / `Esc` | 停止服务器                         |
| 全局           | `q`         | 退出程序                           |
|                | `Esc`       | 返回上一个界面                     |

## 配置文件

配置文件在首次运行时自动创建：

- **Windows**: `C:\Users\<用户名>\.llamalauncher\config.yml`
- **Linux**: `~/.llamalauncher/config.yml`
- **macOS**: `~/.llamalauncher/config.yml`

配置示例：

```yaml
model_dir: "~/models"
host: "127.0.0.1"
port: 8080
llama_bin_path: "/usr/local/bin/llama-server"
debug_logging: false
presets:
  - name: "high-throughput"
    ctx_size: 8192
    batch_size: 2048
    description: "最大吞吐量，高内存占用"
  - name: "low-memory"
    ctx_size: 1024
    n_gpu_layers: 0
    batch_size: 128
    description: "最小内存占用"
```

## 技术栈

- **TUI 框架**: [Ratatui](https://ratatui.rs/) + [Crossterm](https://github.com/crossterm-rs/crossterm)
- **异步运行时**: [Tokio](https://tokio.rs/)
- **硬件检测**: [sysinfo](https://github.com/GuillaumeGomez/sysinfo)（CPU/内存）、[nvml-wrapper](https://github.com/Cldfire/nvml-wrapper)（NVIDIA GPU）
- **HTTP 客户端**: [reqwest](https://github.com/seanmonstar/reqwest)（服务器健康检查、聊天 API 调用）
- **模型解析**: 自定义 GGUF 元数据读取器（无外部 GGUF 依赖）
- **配置文件**: [serde_yaml](https://github.com/dtolnay/serde-yaml) + [serde](https://serde.rs/)

## 项目结构

```
src/
├── main.rs              # 程序入口
├── lib.rs               # 库根模块
├── config/              # 配置加载、保存、预设管理
├── error.rs             # 错误类型定义
├── hardware/            # CPU、内存、GPU 检测
├── launcher/            # 进程管理、服务器管理、API 客户端
├── params/              # 参数优化、模型元数据解析
└── tui/
    ├── app.rs           # 应用状态、事件循环、界面导航
    ├── screens/         # 硬件、模型选择、启动、聊天、配置界面
    └── widgets/         # 可复用的 UI 组件
```

## 贡献指南

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解参与方式。

## 许可证

本项目采用 Apache License 2.0 — 详见 [LICENSE](LICENSE) 文件。

## 致谢

- [llama.cpp](https://github.com/ggml-org/llama.cpp) — 本工具所封装的优秀推理引擎
- [Ratatui](https://ratatui.rs/) — Rust TUI 框架
- 所有使这一切成为可能的开源贡献者
