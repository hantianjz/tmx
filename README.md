# TMX

A tmux session manager with declarative TOML configuration, written in Rust.

## Features

- Create and manage tmux sessions from TOML configuration files
- Define multiple windows with custom panes and layouts
- Execute commands in specific panes on session creation
- Set environment variables per pane
- List configured and running sessions
- Native TOML parsing (no external dependencies)
- Fast, compiled binary
- Fish shell completion generation

## Installation

### Requirements

- [Rust](https://rustup.rs/) (for building)
- [tmux](https://github.com/tmux/tmux) 2.0+

### Building from Source

```bash
git clone https://github.com/hjz/fishmux.git
cd fishmux
cargo build --release
```

The binary will be at `target/release/tmx`.

### Installing

```bash
# Option 1: Install with cargo
cargo install --path .

# Option 2: Copy manually
cp target/release/tmx ~/.local/bin/
# or
sudo cp target/release/tmx /usr/local/bin/
```

### Fish Shell Setup

```bash
# Generate and install completions
tmx completions fish > ~/.config/fish/completions/tmx.fish

# Generate t alias completions
tmx completions fish | sed 's/tmx/t/g; s/__tmx/__t/g' > ~/.config/fish/completions/t.fish

# Generate t alias function
tmx alias fish > ~/.config/fish/functions/t.fish

# Reload fish
source ~/.config/fish/config.fish
```

Or use the automated setup:

```bash
tmx setup fish
# Follow the instructions
```

## Quick Start

1. Initialize the configuration file:

```bash
tmx init
```

2. Edit the configuration file at `~/.config/tmx/tmx.toml`

3. Start a session:

```bash
tmx start dev
# or use the short alias
t start dev
```

## Usage

### Commands

```bash
tmx                    # List configured and running sessions (default)
tmx start <session>    # Create and/or attach to a session
tmx stop <session>     # Stop (kill) a session
tmx list               # List configured and running sessions
tmx running            # Show only running tmux sessions
tmx init               # Create default configuration file
tmx validate           # Validate configuration syntax
tmx completions fish   # Generate Fish shell completions
tmx alias fish         # Generate Fish shell alias
tmx setup fish         # Show setup instructions
```

**Short alias:** Use `t` as a shorthand for `tmx`:

```bash
t start dev
t stop dev
t list
```

### Configuration

Configuration file location: `~/.config/tmx/sessions.toml`

#### Basic Example

```toml
[sessions.dev]
name = "dev"
root = "~/projects/myapp"
windows = [
    { name = "editor", panes = [
        { command = "nvim" }
    ]},
    { name = "shell", panes = [
        { command = "git status" }
    ]}
]
```

#### Advanced Example

```toml
[sessions.fullstack]
name = "fullstack"
root = "~/projects/webapp"
windows = [
    { name = "editor", panes = [
        { command = "nvim ." }
    ]},
    { name = "servers", panes = [
        { command = "cd backend && npm run dev", env = { NODE_ENV = "development", PORT = "3000" } },
        { command = "cd frontend && npm start", env = { PORT = "3001" } }
    ]},
    { name = "database", panes = [
        { command = "docker-compose up postgres" }
    ]},
    { name = "logs", panes = [
        { command = "tail -f backend/logs/app.log" },
        { command = "tail -f frontend/logs/access.log" }
    ]}
]
```

### Configuration Schema

#### Session

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Session name (used with tmux) |
| `root` | string | No | Starting directory for all windows (default: `~`) |
| `windows` | array | Yes | List of window configurations |

#### Window

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Window name |
| `panes` | array | Yes | List of pane configurations |

#### Pane

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | No | Command to execute in the pane |
| `env` | object | No | Environment variables for the pane |

## Examples

See the [examples](./examples/) directory for more configuration examples.

## Tips

- Use `tmux list-sessions` to see all tmux sessions
- Use `tmux attach -t <session>` to manually attach to a session
- Use `Ctrl-b d` to detach from a tmux session
- Session names are used as-is (no automatic prefixing)
- The tool respects your tmux `base-index` setting

## Migrating from fishmux (Fish shell version)

If you were using the previous Fish shell implementation:

1. **Copy your config:**
   ```bash
   cp ~/.config/fishmux/sessions.toml ~/.config/tmx/sessions.toml
   ```

2. **Remove old Fish functions:**
   ```bash
   rm ~/.config/fish/functions/fishmux.fish
   rm ~/.config/fish/functions/tm.fish
   rm ~/.config/fish/functions/__fishmux_*.fish
   rm ~/.config/fish/completions/fishmux.fish
   rm ~/.config/fish/completions/tm.fish
   ```

3. **Install the Rust version** (see Installation above)

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running

```bash
cargo run -- <command>
# Example:
cargo run -- init
cargo run -- start dev
```

## License

This is free and unencumbered software released into the public domain. See [UNLICENSE](./UNLICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
