# Contributing to portr

Thank you for your interest in contributing to portr! 🐸

## Getting Started

### Prerequisites

- Rust 1.70+ (stable)
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/Brutus1066/portr.git
cd portr

# Build
cargo build

# Run tests
cargo test

# Run the TUI
cargo run -- dashboard
```

### With Docker Support

```bash
# Build with Docker feature
cargo build --features docker

# Test Docker functionality
cargo test --features docker
```

## Development Workflow

### Running Locally

```bash
# Debug build
cargo run -- [args]

# Release build
cargo run --release -- [args]

# TUI Dashboard
cargo run -- dashboard

# With verbose output
cargo run -- 3000 --verbose
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_app_new

# Run integration tests only
cargo test --test integration
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Check for errors
cargo check
```

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Public module exports
├── error.rs         # Error types
├── port.rs          # Port detection
├── process.rs       # Process killing
├── display.rs       # Terminal output
├── export.rs        # JSON/CSV/Markdown export
├── services.rs      # Known service detection
├── config.rs        # Configuration management
├── interactive.rs   # Interactive mode
└── tui/             # TUI Dashboard
    ├── mod.rs       # Event loop
    ├── app.rs       # Application state
    ├── ui.rs        # UI rendering
    └── events.rs    # Keyboard events
```

## Code Style

- Use `cargo fmt` before committing
- Follow Rust idioms and best practices
- Add tests for new functionality
- Document public functions

## Commit Messages

Use conventional commits:

```
feat: add export to CSV
fix: handle port not found error
docs: update README with TUI section
test: add integration tests for kill
refactor: simplify port detection logic
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Format code (`cargo fmt`)
6. Lint (`cargo clippy`)
7. Commit your changes
8. Push to your fork
9. Open a Pull Request

## Feature Requests

Open an issue with the `enhancement` label and describe:
- What problem does it solve?
- Proposed solution
- Any alternatives considered

## Bug Reports

Open an issue with the `bug` label and include:
- OS and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Any error messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

**🐸 LazyFrog | [kindware.dev](https://kindware.dev)**
