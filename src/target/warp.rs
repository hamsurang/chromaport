use crate::ir::{ThemeIR, ThemeType};
use crate::store::{theme_slug, atomic_write};
use anyhow::{Context, Result};
use serde::Serialize;

pub fn detect() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".warp").exists())
        .unwrap_or(false)
}

pub fn write(ir: &ThemeIR) -> Result<()> {
    let themes_dir = dirs::home_dir()
        .context("cannot determine home directory")?
        .join(".warp/themes");

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("{slug}.yaml"));

    let theme = WarpTheme {
        name: ir.name.as_str(),
        accent: ir.accent.as_str(),
        cursor: ir.cursor.as_str(),
        background: ir.background.as_str(),
        foreground: ir.foreground.as_str(),
        details: match ir.theme_type {
            ThemeType::Dark => "darker",
            ThemeType::Light => "lighter",
        },
        terminal_colors: WarpTerminalColors {
            normal: WarpPalette {
                black: ir.terminal.normal.black.as_str(),
                red: ir.terminal.normal.red.as_str(),
                green: ir.terminal.normal.green.as_str(),
                yellow: ir.terminal.normal.yellow.as_str(),
                blue: ir.terminal.normal.blue.as_str(),
                magenta: ir.terminal.normal.magenta.as_str(),
                cyan: ir.terminal.normal.cyan.as_str(),
                white: ir.terminal.normal.white.as_str(),
            },
            bright: WarpPalette {
                black: ir.terminal.bright.black.as_str(),
                red: ir.terminal.bright.red.as_str(),
                green: ir.terminal.bright.green.as_str(),
                yellow: ir.terminal.bright.yellow.as_str(),
                blue: ir.terminal.bright.blue.as_str(),
                magenta: ir.terminal.bright.magenta.as_str(),
                cyan: ir.terminal.bright.cyan.as_str(),
                white: ir.terminal.bright.white.as_str(),
            },
        },
    };

    let yaml = serde_yaml_ng::to_string(&theme).context("failed to serialize warp theme")?;

    atomic_write(&path, yaml.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;

    println!("  ✔ {} → {}", ir.name, path.display());

    Ok(())
}

#[derive(Serialize)]
struct WarpTheme<'a> {
    name: &'a str,
    accent: &'a str,
    cursor: &'a str,
    background: &'a str,
    foreground: &'a str,
    details: &'a str,
    terminal_colors: WarpTerminalColors<'a>,
}

#[derive(Serialize)]
struct WarpTerminalColors<'a> {
    normal: WarpPalette<'a>,
    bright: WarpPalette<'a>,
}

#[derive(Serialize)]
struct WarpPalette<'a> {
    black: &'a str,
    red: &'a str,
    green: &'a str,
    yellow: &'a str,
    blue: &'a str,
    magenta: &'a str,
    cyan: &'a str,
    white: &'a str,
}
