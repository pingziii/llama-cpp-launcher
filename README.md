<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white">
</picture>
![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)

# llama-cpp-launcher

A Terminal User Interface (TUI) tool for launching and managing [llama.cpp](https://github.com/ggml-org/llama.cpp) model servers. Automatically detects hardware, calculates optimal launch parameters, and provides a real-time server console — all from the terminal.

## Features

- **Hardware Auto-Detection** — Detects CPU cores, RAM, and GPU (NVIDIA via NVML) on startup
- **Optimal Parameter Calculation** — Automatically computes ideal settings (context size, batch size, GPU layers, threads) based on your hardware and model metadata
- **Model Browser** — Scans your model directory for GGUF files, grouped by provider (e.g., Qwen, Meta, Mistral)
- **Interactive Parameter Adjustment** — Fine-tune launch parameters before starting the server
- **Preset Management** — Save and switch between parameter presets (e.g., "high-throughput", "low-memory")
- **Real-Time Server Logs** — View llama.cpp stderr output directly in the TUI, with scroll support
- **Config Editor** — Edit model directory, binary path, and debug settings from within the TUI

## Screenshots

```
┌────────────── Select Model ───────────────┐
│  Qwen ──── 3 models                        │
│    Qwen2.5-0.5B-Instruct-Q8_0  │ 0.5B params │ 500 MB │
│    Qwen2.5-1.5B-Instruct-Q8_0  │ 1.5B params │ 1.2 GB │
│  Meta ──── 2 models                        │
│    Llama-3.2-1B-Instruct-Q8_0  │ 1.0B params │ 1.1 GB │
└────────────────────────────────────────────┘
```

```
┌────────── Launch Configuration ───────────┐
│  Model: /models/qwen/qwen2.5-q8_0.gguf     │
│  Parameters: 0.5B  Context: 32K            │
│                                            │
│  Adjustable Parameters:                    │
│    GPU Layers     99                       │
│    Context Size   8192                     │
│    Batch Size     2048                     │
│    Flash Attn     auto                     │
│    ...                                     │
│                                            │
│  Command:                                  │
│    llama-server --host 127.0.0.1 --port... │
└────────────────────────────────────────────┘
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021, MSRV 1.75+)
- A [llama.cpp](https://github.com/ggml-org/llama.cpp/releases) server binary (`llama-server`) on your system PATH or configured via `llama_bin_path`
- GGUF format model files

## Installation

### From source

```bash
git clone https://github.com/pingziii/llama-cpp-launcher.git
cd llama-cpp-launcher
cargo build --release
```

The binary will be at `target/release/llama-cpp-launcher.exe` (Windows) or `target/release/llama-cpp-launcher` (Linux/macOS).

### Post-install

1. Place your GGUF model files in `~/models/` (or set a custom path in the config)
2. Ensure `llama-server` is available in your PATH, or set `llama_bin_path` in the config
3. Run the binary

## Usage

1. **Launch**: Run `llama-cpp-launcher` in your terminal
2. **Hardware Detection**: The tool automatically detects your system specs
3. **Select Model**: Browse and choose a GGUF model from the list
4. **Review Parameters**: Check the auto-calculated settings, press `a` to adjust
5. **Launch**: Press `Enter` to start the llama.cpp server
6. **Monitor**: View server info or press `l` to see real-time console logs
7. **Stop**: Press `s` or `Esc` to stop the server and return

### Key Bindings

| Screen        | Key         | Action                          |
|---------------|-------------|----------------------------------|
| Hardware      | `Enter`     | Continue to model selection      |
| Model Select  | `↑/↓`       | Navigate models within a group   |
|               | `←/→`       | Switch model group               |
|               | `Enter`     | Select model                     |
|               | `c`         | Open configuration editor        |
| Launch Config | `Enter`     | Launch the server                |
|               | `a`         | Enter adjust mode                |
|               | `p`         | Cycle presets                    |
| (Adjust mode) | `↑/↓`       | Select parameter                 |
|               | `←/→`       | Adjust value (Shift = 10x step)  |
| Chat / Logs   | `l`         | Toggle server console logs       |
|               | `↑/↓`       | Scroll logs (in log view)        |
|               | `s` / `Esc` | Stop server                      |
| Global        | `q`         | Quit                             |
|               | `Esc`       | Back to previous screen          |

## Configuration

The config file is auto-created at:

- **Windows**: `C:\Users\<You>\.llamalauncher\config.yml`
- **Linux**: `~/.llamalauncher/config.yml`
- **macOS**: `~/.llamalauncher/config.yml`

Example config:

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
    description: "Maximum throughput, high memory usage"
  - name: "low-memory"
    ctx_size: 1024
    n_gpu_layers: 0
    batch_size: 128
    description: "Minimal memory footprint"
```

## Technical Stack

- **TUI Framework**: [Ratatui](https://ratatui.rs/) + [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Async Runtime**: [Tokio](https://tokio.rs/)
- **Hardware Detection**: [sysinfo](https://github.com/GuillaumeGomez/sysinfo) (CPU/RAM), [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper) (NVIDIA GPU)
- **HTTP Client**: [reqwest](https://github.com/seanmonstar/reqwest) (server health check, chat API calls)
- **Model Parsing**: Custom GGUF metadata reader (no external dependency for GGUF format)
- **Config**: [serde_yaml](https://github.com/dtolnay/serde-yaml) + [serde](https://serde.rs/)

## Project Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # Library root
├── config/              # Configuration loading, saving, presets
├── error.rs             # Error types
├── hardware/            # CPU, RAM, GPU detection
├── launcher/            # Process spawning, server management, API client
├── params/              # Parameter optimization, model metadata parsing
└── tui/
    ├── app.rs           # App state, event loop, navigation
    ├── screens/         # Hardware, model select, launch, chat, config screens
    └── widgets/         # Reusable UI components
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the Apache License 2.0 — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [llama.cpp](https://github.com/ggml-org/llama.cpp) — The incredible inference engine this tool wraps
- [Ratatui](https://ratatui.rs/) — The Rust TUI framework
- All open-source contributors whose work made this possible
