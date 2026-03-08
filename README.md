<p align="center">
  <img src="assets/chromaport.png" alt="Chromaport" width="600" />
</p>

# chromaport

Your favorite editor theme, everywhere.

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
chromaport [OPTIONS]

Options:
  -e, --editor <EDITOR>    Source editor [possible values: vscode, cursor]
  -t, --target <TARGET>    Target app [possible values: superset, warp]
  -y, --yes                Non-interactive mode (import active theme, overwrite if exists)
      --no-activate        Do not change the active theme after import
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
