# Kicode

AI-powered coding assistant that runs in your terminal.

## Installation

### Quick Install (macOS & Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/kishanhitk/kicode/master/install.sh | sh
```

### Using Cargo

```bash
cargo install --git https://github.com/kishanhitk/kicode
```

### Manual Download

Download prebuilt binaries from [GitHub Releases](https://github.com/kishanhitk/kicode/releases).

## Setup

On first run, Kicode will prompt you for your OpenRouter API key:

```
$ kicode

Welcome to Kicode! Let's get you set up.

To use Kicode, you need an OpenRouter API key.
Get one at: https://openrouter.ai/keys

Enter your API key: ********
Validating... ✓

Config saved to ~/.config/kicode/config.toml
```

To reconfigure later:

```bash
kicode setup
```

## Usage

```bash
# Start interactive session
kicode

# Use a specific model
kicode --model anthropic/claude-3.5-sonnet
```

### Available Commands

In the REPL:
- `/help` - Show help
- `/clear` - Clear conversation history
- `/exit` - Exit kicode

### Tools

Kicode has access to:
- `read_file` - Read file contents
- `write_file` - Create or overwrite files
- `edit_file` - Make precise edits
- `shell` - Execute shell commands
- `glob_search` - Find files by pattern
- `grep` - Search file contents

## Configuration

Config file location: `~/.config/kicode/config.toml`

```toml
api_key = "sk-or-..."
model = "x-ai/grok-code-fast-1"

[safety]
additional_patterns = []
skip_confirmation = []
```

### Environment Variables

- `OPENROUTER_API_KEY` - API key (overrides config file)
- `KICODE_MODEL` - Model to use
- `KICODE_DEBUG=1` - Enable debug logging

## License

MIT
