# AGENTS.md - Guidelines for AI Coding Agents

This document provides guidelines for AI agents working in the `kicode` repository.

## Project Overview

**kicode** is a Rust CLI application - an AI-powered coding assistant that runs in the terminal.
It provides an interactive REPL interface for interacting with AI models via the OpenRouter API.

**Tech Stack:**
- Language: Rust (Edition 2024)
- Async runtime: tokio
- CLI parsing: clap
- HTTP client: reqwest
- Serialization: serde/serde_json
- Error handling: thiserror

---

## Build Commands

```bash
# Build the project (debug)
cargo build

# Build optimized release version
cargo build --release

# Run the application
cargo run

# Run with specific model
cargo run -- --model anthropic/claude-3.5-sonnet

# Check code without building (fast)
cargo check
```

---

## Linting and Formatting

```bash
# Format all code
cargo fmt

# Check formatting without applying changes
cargo fmt --check

# Run Clippy linter
cargo clippy

# Run Clippy with warnings as errors (CI mode)
cargo clippy -- -D warnings
```

---

## Testing

```bash
# Run all tests
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run a single test by name
cargo test test_safe_commands

# Run tests matching a pattern
cargo test dangerous

# Run tests in a specific module
cargo test safety::analyzer

# Run tests in a specific file
cargo test --test analyzer
```

**Test Location:** Tests are inline in source files using `#[cfg(test)] mod tests`.
Example: `src/safety/analyzer.rs` contains the test module at the bottom.

---

## Code Style Guidelines

### File Naming
- Use `snake_case` for all file names: `read_file.rs`, `glob_search.rs`

### Naming Conventions

| Element         | Convention              | Example                    |
|-----------------|-------------------------|----------------------------|
| Files           | `snake_case`            | `edit_file.rs`             |
| Structs/Enums   | `PascalCase`            | `KicodeError`, `ToolResult`|
| Functions       | `snake_case`            | `load_config`, `is_dangerous` |
| Constants       | `SCREAMING_SNAKE_CASE`  | `DEFAULT_MODEL`, `API_URL` |
| Variables       | `snake_case`            | `stream_buffer`, `api_key` |
| Type Parameters | Single uppercase letter | `T`, `F`                   |

### Import Ordering

Order imports as follows (no blank lines between groups):
1. Internal crate imports (`crate::`)
2. External crate imports

```rust
use crate::api::types::{ChatRequest, Message};
use crate::config::Config;
use crate::error::{KicodeError, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
```

Group multiple items from the same module with braces:
```rust
use crate::error::{KicodeError, Result};
```

### Type Annotations

- Use derive macros for common traits:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
}
```

- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Use `#[serde(rename_all = "lowercase")]` for enum variants
- Define custom `Result` type alias in error module:
```rust
pub type Result<T> = std::result::Result<T, KicodeError>;
```

### Error Handling

1. **Custom error enum with `thiserror`:**
```rust
#[derive(Error, Debug)]
pub enum KicodeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

2. **Use `?` operator for early returns**
3. **For tool-level recoverable errors, return `ToolResult::error()`:**
```rust
Err(e) => return Ok(ToolResult::error(format!("Failed: {}", e))),
```

4. **Provide helpful context in error messages for user-facing errors**

### Function Patterns

**Constructors:** Always name constructors `new()`:
```rust
impl OpenRouterClient {
    pub fn new(config: &Config) -> Self {
        Self { /* ... */ }
    }
}
```

**Async functions with callbacks:**
```rust
pub async fn process<F>(&self, mut on_chunk: F) -> Result<()>
where
    F: FnMut(String),
```

**Builder pattern for configuration:**
```rust
pub fn with_additional_patterns(mut self, patterns: &[String]) -> Self {
    // ...
    self
}
```

**Implement `Default` trait when appropriate:**
```rust
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

### Export Patterns

- Use `mod.rs` files for module exports and re-exports
- Prefer named exports via `pub use`:
```rust
// In mod.rs
pub mod client;
pub mod types;

pub use client::OpenRouterClient;
pub use types::{Message, Role};
```

### Documentation

- Use `///` doc comments for public API items (currently minimal in codebase)
- Use `//` inline comments for non-obvious logic
- Use `#[command(about = "...")]` for CLI documentation

### Test Organization

Place tests at the bottom of the source file:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // ...
    }
}
```

---

## Project Structure

```
src/
├── main.rs           # Entry point, CLI parsing, system prompt
├── lib.rs            # Module exports
├── config.rs         # Configuration loading
├── error.rs          # Custom error types
├── conversation.rs   # Message history management
├── api/              # OpenRouter API integration
│   ├── mod.rs        # Module exports
│   ├── client.rs     # HTTP client, streaming
│   ├── types.rs      # Data structures
│   └── streaming.rs  # SSE parsing
├── tools/            # AI tool implementations (one file per tool)
│   ├── mod.rs        # Tool trait, ToolRegistry
│   ├── read_file.rs
│   ├── write_file.rs
│   ├── edit_file.rs
│   ├── shell.rs
│   ├── grep.rs
│   └── glob_search.rs
├── repl/             # Interactive interface
│   ├── mod.rs        # Main loop
│   └── output.rs     # Terminal formatting
└── safety/           # Command safety analysis
    ├── mod.rs
    └── analyzer.rs   # Dangerous command detection
```

---

## Environment Configuration

- `OPENROUTER_API_KEY` - Required API key (or set in `~/.config/kicode/config.toml`)
- `KICODE_MODEL` - Override default model
- `KICODE_DEBUG=1` - Enable verbose debug logging

---

## Key Patterns to Follow

1. **One tool per file** in `src/tools/`
2. **Implement the `Tool` trait** for new tools
3. **Register tools** in `ToolRegistry::new()`
4. **Use `ToolResult::success()` or `ToolResult::error()`** for tool returns
5. **Add safety patterns** to `SafetyAnalyzer` for dangerous commands
6. **Prefer `?` operator** over explicit `match` for error propagation
7. **Keep functions focused** - extract helpers as needed
