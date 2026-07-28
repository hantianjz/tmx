# TMX and HMX

Declarative tmux session and Herdr workspace managers sharing one TOML configuration.

## Features

- Create and manage tmux sessions from TOML configuration files
- Create and manage Herdr workspaces from the same configuration
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
- [Herdr](https://herdr.dev/docs/install/) for `hmx` (also required remotely)

### Building from Source

```bash
git clone https://github.com/hjz/fishmux.git
cd fishmux
cargo build --release
```

The binaries will be at `target/release/tmx` and `target/release/hmx`.

### Installing

```bash
# Option 1: Install with cargo
cargo install --path .

# Option 2: Copy manually
cp target/release/{tmx,hmx} ~/.local/bin/
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

## Herdr workspace management

`hmx` maps configured sessions to Herdr workspaces, windows to tabs, and
panes to panes. It preserves commands, working directories, and environment
variables. The `[sessions.*]` spelling remains unchanged so `tmx` and `hmx`
can share the file.

```bash
hmx                         # Attach, or cycle when already inside Herdr
hmx open dev                # Create/focus a workspace and attach
hmx close dev               # Close a running workspace
hmx refresh dev             # Add missing tabs and panes
hmx list                    # List configured and running workspaces
hmx validate
hmx completions fish

hmx --session agents open dev
hmx --remote workbox open dev
hmx --remote workbox --session agents open dev
```

`hmx open <name>` first focuses an already-running workspace whose label
exactly matches `<name>`, even when the configuration is missing or invalid.
Otherwise, `<name>` may be a configured key or workspace name. An unknown name
creates a dynamic workspace by cloning the configured default layout and
renaming it. Dynamic local workspaces use the caller's current directory as
their root; dynamic remote workspaces use the remote account's home directory.

Bare `hmx` cycles through running workspaces without requiring configuration.
When configuration is available, configured workspaces come first; if it is
missing or invalid, live workspace labels are ordered alphabetically.

`--remote` reads the configuration locally, expands `~` from the remote home
directory, manages the remote Herdr server over SSH, and attaches through the
local Herdr client. `hmx` is not needed remotely, but Herdr must be installed.
Remote targets are rejected from inside an existing Herdr client.

For `hmx`, configuration path precedence is `--config`, `HMX_CONFIG_PATH`,
`TMX_CONFIG_PATH`, then `~/.config/tmx/tmx.toml`.

Layout conversion is best effort: horizontal and vertical splits become
right and down splits, percentage sizes become ratios, and absolute cell
sizes are ignored with a warning. Refresh preserves extra tabs, panes, and
existing processes. It matches configured windows to Herdr tabs by position,
not their current labels, so Herdr's automatic tab renaming does not create
duplicates. Refreshing an unconfigured running workspace uses the default
layout.

```bash
hmx completions bash > ~/.local/share/bash-completion/completions/hmx
hmx completions fish > ~/.config/fish/completions/hmx.fish
hmx completions zsh > ~/.local/share/zsh/site-functions/_hmx
```

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

### Releasing

Releases are managed with [GoReleaser](https://goreleaser.com/). To create a new release:

```bash
# Tag the release
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Run GoReleaser (requires GITHUB_TOKEN with repo access)
export GITHUB_TOKEN="your-token"
goreleaser release --clean
```

GoReleaser will:
1. Build binaries for Linux and macOS (x86_64 and arm64)
2. Create a GitHub release with the binaries
3. Update the Homebrew formula in [homebrew-tap](https://github.com/hantianjz/homebrew-tap)

For local testing without publishing:

```bash
goreleaser release --snapshot --clean
```

## License

This is free and unencumbered software released into the public domain. See [UNLICENSE](./UNLICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
