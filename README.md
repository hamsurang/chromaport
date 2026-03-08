<p align="center">
  <img src="assets/chromaport.png" alt="Chromaport" width="600" />
</p>

# chromaport

Your favorite editor theme, everywhere.

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
> Set as active theme? One Monokai

Converting 2 theme(s)...
  ✔ One Monokai → /Users/you/.superset/app-state.json
    Active theme set to 'One Monokai'
  ✔ Ayu Dark → /Users/you/.superset/app-state.json
    Restart Superset to apply.
```

### Options

```
chromaport [OPTIONS] [COMMAND]

Commands:
  update    Check for updates and upgrade chromaport

Options:
  -e, --editor <EDITOR>    Source editor [possible values: vscode, cursor]
  -t, --target <TARGET>    Target app [possible values: superset, warp, ghostty]
  -y, --yes                Non-interactive mode (import active theme, overwrite if exists)
      --activate           Apply the theme to the target app's config
  -h, --help               Print help
  -V, --version            Print version
```

### Non-interactive

```sh
# Import the active VS Code theme to Superset
chromaport --editor vscode --target superset --yes
```

## Supported editors

| Editor  | Path                    |
| ------- | ----------------------- |
| VS Code | `~/.vscode/extensions/` |
| Cursor  | `~/.cursor/extensions/` |

## Supported targets

| Target   | How it works                                                    |
| -------- | --------------------------------------------------------------- |
| Superset | Writes to `~/.superset/app-state.json` (quit Superset first)    |
| Warp     | Writes to `~/.warp/themes/*.yaml` (auto-detected while running) |

## How it works

1. Scans editor extension directories for `package.json` theme contributions
2. Parses VS Code theme JSON (with JSONC comment stripping and `include` inheritance)
3. Converts to an intermediate representation (IR)
4. Writes to the selected target format

## Development

```sh
cargo test
cargo fmt-check
cargo lint
```

## License

MIT
