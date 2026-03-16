<p align="center">
  <img src="assets/chromaport.png" alt="Chromaport" width="600" />
</p>

# chromaport

> Your favorite editor theme, everywhere.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

[한국어](./README.ko.md)

## Name

**chroma** (color) + **port** (carry across) — carry your editor colors everywhere.

## Install

### Homebrew

```sh
brew tap hamsurang/chromaport
brew install chromaport
```

### Cargo

```sh
cargo install chromaport
```

### From source

```sh
git clone https://github.com/hamsurang/chromaport.git
cd chromaport
cargo install --path .
```

## Update

chromaport automatically checks for new releases once a week. When an update is available, you'll see a notice after running any command:

```
A new release of chromaport is available: 0.2.0 → 0.3.0
Run `chromaport update` to upgrade.
```

To update, simply run:

```sh
chromaport update
```

This auto-detects your install method (Homebrew or Cargo) and runs the appropriate upgrade command.

To disable the automatic update check, set:

```sh
export CHROMAPORT_NO_UPDATE_CHECK=1
```

The check is also automatically disabled in CI environments and non-interactive shells.

## Usage

Run `chromaport` and follow the interactive prompts:

```
$ chromaport
> Select editor: Cursor
> Select themes to migrate: One Monokai, Ayu Dark
> Select target app: Superset

Converting 2 theme(s)...
  ✔ One Monokai → ~/.config/chromaport/themes/one-monokai.json
  ✔ Ayu Dark → ~/.config/chromaport/themes/ayu-dark.json
```

### Commands

```
chromaport [OPTIONS] [COMMAND]

Commands:
  update   Check for updates and upgrade chromaport
  apply    Apply a saved theme to additional targets
  create   Create a custom theme from scratch
  presets  Manage preset themes

Options:
  -v, --version          Print version
  -e, --editor <EDITOR>  Source editor [possible values: vscode, cursor]
  -t, --target <TARGET>  Target app [possible values: superset, warp, ghostty]
  -h, --help             Print help
```

### Import themes

```sh
# Interactive mode — select editor, themes, and target step by step
chromaport

# Non-interactive — specify editor and target directly
chromaport --editor vscode --target ghostty
```

Theme selection includes a **TUI-based live preview** powered by ratatui, showing a real-time rendering of each theme as you browse. Use arrow keys to navigate and type to filter.

### Apply saved themes

```sh
chromaport apply
```

Re-apply a previously imported theme to additional targets. Shows which targets already have the theme applied.

### Create a custom theme

```sh
chromaport create
```

Build a theme from scratch using an **interactive color picker**:

1. Pick a background color
2. Pick a foreground color
3. Pick an accent color
4. Preview and confirm

The color picker supports **slider mode** (arrow keys to adjust HSL values, Shift for 5x step) and **hex mode** (press `#` to type a hex code directly). A full palette is automatically derived from your 3 base colors.

### Preset themes

```sh
# List available presets
chromaport presets list

# Install presets
chromaport presets install
```

Browse and install curated preset themes from the chromaport repository.

## Supported editors

| Editor  | Path                    |
| ------- | ----------------------- |
| VS Code | `~/.vscode/extensions/` |
| Cursor  | `~/.cursor/extensions/` |

## Supported targets

| Target   | How it works                                                                      |
| -------- | --------------------------------------------------------------------------------- |
| Superset | Writes to `~/.superset/chromaport-themes/` — import via Superset UI              |
| Warp     | Symlinks to `~/.warp/themes/` — auto-detected while running                      |
| Ghostty  | Symlinks to `~/.config/ghostty/themes/` — apply via config or reload             |

## How it works

1. Scans editor extension directories for `package.json` theme contributions
2. Parses VS Code theme JSON (with JSONC comment stripping and `include` inheritance)
3. Converts to an intermediate representation (IR)
4. Saves to a central theme store (`~/.config/chromaport/themes/`)
5. Symlinks or writes to the selected target format

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets
```

## License

MIT
