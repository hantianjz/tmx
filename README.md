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

# Reload fish
source ~/.config/fish/config.fish
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
```

## Usage

### Commands

```bash
tmx                    # List configured and running sessions (default)
tmx start <session>    # Create and/or attach to a session
tmx stop <session>     # Stop (kill) a session
tmx list               # List configured and running sessions
tmx init               # Create default configuration file
tmx validate           # Validate configuration syntax
tmx completions fish   # Generate Fish shell completions
```

### Global Options

```bash
tmx -c <path>          # Use custom config file
tmx --config <path>    # Long form

# Examples:
tmx -c ~/my-configs/work.toml list
tmx --config ./project.toml start dev
```

### Configuration

Configuration file location: `~/.config/tmx/tmx.toml`

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

#### Advanced Example (with layouts and custom sizes)

```toml
[sessions.fullstack]
name = "fullstack"
root = "~/projects/webapp"
startup_window = "editor"       # Focus editor window on startup
startup_pane = 0                # Focus first pane in that window

# Editor window with main-vertical layout
[[sessions.fullstack.windows]]
name = "editor"
layout = "main-vertical"        # Large left pane, smaller right panes

[[sessions.fullstack.windows.panes]]
command = "nvim ."              # Main pane (takes majority of space)

[[sessions.fullstack.windows.panes]]
command = "git status"
size = "30%"                    # Takes 30% of width

# Servers window with custom per-pane directories
[[sessions.fullstack.windows]]
name = "servers"
layout = "even-horizontal"

[[sessions.fullstack.windows.panes]]
command = "npm run dev"
root = "~/projects/webapp/backend"
env = { NODE_ENV = "development", PORT = "3000" }

[[sessions.fullstack.windows.panes]]
command = "npm start"
root = "~/projects/webapp/frontend"
env = { PORT = "3001" }

# Database window
[[sessions.fullstack.windows]]
name = "database"

[[sessions.fullstack.windows.panes]]
command = "docker-compose up postgres"

# Logs window with tiled layout
[[sessions.fullstack.windows]]
name = "logs"
layout = "tiled"                # Grid layout for multiple log panes

[[sessions.fullstack.windows.panes]]
command = "tail -f backend/logs/app.log"

[[sessions.fullstack.windows.panes]]
command = "tail -f frontend/logs/access.log"

[[sessions.fullstack.windows.panes]]
command = "docker logs -f postgres"
split = "vertical"              # Explicitly vertical split
size = "40"                     # 40 lines tall
```

### Configuration Schema

#### Session

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Session name (used with tmux) |
| `root` | string | No | Starting directory for all windows (default: `~`) |
| `windows` | array | Yes | List of window configurations |
| `startup_window` | string/number | No | Window to focus on startup (name or 0-based index, default: 0) |
| `startup_pane` | number | No | Pane to focus on startup (0-based index, default: 0) |

#### Window

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Window name |
| `panes` | array | Yes | List of pane configurations |
| `layout` | string | No | Tmux layout: `main-vertical`, `main-horizontal`, `even-horizontal`, `even-vertical`, `tiled` |
| `root` | string | No | Override session working directory |

#### Pane

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | No | Command to execute in the pane |
| `env` | object | No | Environment variables for the pane |
| `root` | string | No | Override window/session working directory |
| `split` | string | No | Split direction: `horizontal` or `vertical` (default: alternating) |
| `size` | string | No | Pane size: percentage (`30%`) or lines/columns (`20`) |

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
   mkdir -p ~/.config/tmx
   cp ~/.config/fishmux/sessions.toml ~/.config/tmx/tmx.toml
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
