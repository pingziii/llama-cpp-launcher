# Contributing to llama-cpp-launcher

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating, you agree to maintain a respectful and inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in Issues
2. Open a new issue with a clear title and description
3. Include steps to reproduce, expected behavior, and actual behavior
4. Attach relevant logs or screenshots if applicable

### Suggesting Features

1. Open an issue describing the feature and its use case
2. Explain why the feature would be useful to most users
3. If possible, outline how the feature might be implemented

### Pull Requests

1. Fork the repository and create a branch from `main`
2. Make your changes, ensuring the code compiles without warnings:
   ```bash
   cargo build --release
   ```
3. Run existing tests:
   ```bash
   cargo test
   ```
4. Write tests for new functionality when applicable
5. Ensure your code follows the project's style (use `cargo fmt`)
6. Run `cargo clippy` and address any warnings
7. Submit a pull request with a clear description of the changes

## Development Setup

```bash
git clone https://github.com/YOUR_USERNAME/llama-cpp-launcher.git
cd llama-cpp-launcher
cargo build --release
```

### Useful commands

- `cargo build --release` — Build optimized binary
- `cargo test` — Run all tests
- `cargo fmt` — Format code
- `cargo clippy` — Lint checks

## Project Structure

See the main [README.md](README.md) for project structure details.

## Coding Guidelines

- Follow Rust standard idioms and patterns
- Write clear, self-documenting code with minimal comments (explain *why*, not *what*)
- Handle errors with `thiserror` and propagate via `Result` types
- Add unit tests for new functionality in `tests/`
- Prefer `tracing` over `println` for logging
- Keep TUI rendering logic in screens/widgets, business logic in `params/`, `launcher/`, `hardware/`

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
