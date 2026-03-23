use crate::ir::{HexColor, ThemeIR};
use crate::store::{atomic_write, chromaport_themes_dir, theme_slug};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// iTerm2 preferences plist path (macOS only).
pub fn iterm2_plist_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .filter(|h| h.is_absolute())
            .map(|h| h.join("Library/Preferences/com.googlecode.iterm2.plist"))
            .filter(|p| p.exists())
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn detect() -> bool {
    iterm2_plist_path().is_some()
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir = chromaport_themes_dir("iterm2").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let slug = theme_slug(&ir.name);
    let theme_path = themes_dir.join(format!("{slug}.itermcolors"));

    let dict = format_iterm2_theme(ir);
    let mut buf: Vec<u8> = Vec::new();
    plist::to_writer_xml(&mut buf, &dict).with_context(|| "failed to serialize iTerm2 plist")?;

    atomic_write(&theme_path, &buf)
        .with_context(|| format!("failed to write {}", theme_path.display()))?;

    // .itermcolors files may be shared or imported — use 0o644 (matching Obsidian target)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&theme_path, std::fs::Permissions::from_mode(0o644));
    }

    Ok(theme_path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("iterm2")?;
    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("{slug}.itermcolors"));
    path.exists().then_some(path)
}

pub fn link(_ir: &ThemeIR, _written_path: &Path) -> LinkResult {
    LinkResult::NotApplicable
}

pub fn post_write_action(_ir: &ThemeIR, written_path: &Path) -> PostWriteAction {
    PostWriteAction::Guide {
        message: format!(
            "  To import into iTerm2:\n\
             \x20 1. Open iTerm2 \u{2192} Settings \u{2192} Profiles \u{2192} Colors\n\
             \x20 2. Click \"Color Presets...\" \u{2192} \"Import...\"\n\
             \x20 3. Select {}\n\
             \x20 4. Choose the imported preset from \"Color Presets...\"\n\n  Tip: Use `chromaport apply` to re-apply this theme to other targets later.",
            written_path.display()
        ),
    }
}

fn hex_to_float(color: &HexColor) -> (f64, f64, f64) {
    let (r, g, b) = color.to_rgb();
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

fn color_dict(color: &HexColor) -> plist::Dictionary {
    let (r, g, b) = hex_to_float(color);
    let mut d = plist::Dictionary::new();
    d.insert("Alpha Component".to_string(), plist::Value::Real(1.0));
    d.insert("Blue Component".to_string(), plist::Value::Real(b));
    d.insert(
        "Color Space".to_string(),
        plist::Value::String("sRGB".to_string()),
    );
    d.insert("Green Component".to_string(), plist::Value::Real(g));
    d.insert("Red Component".to_string(), plist::Value::Real(r));
    d
}

fn format_iterm2_theme(ir: &ThemeIR) -> plist::Dictionary {
    let mut dict = plist::Dictionary::new();

    // Ansi 0-7 (normal palette)
    for (idx, color) in ir.terminal.normal.as_indexed(0) {
        dict.insert(
            format!("Ansi {} Color", idx),
            plist::Value::Dictionary(color_dict(color)),
        );
    }

    // Ansi 8-15 (bright palette)
    for (idx, color) in ir.terminal.bright.as_indexed(8) {
        dict.insert(
            format!("Ansi {} Color", idx),
            plist::Value::Dictionary(color_dict(color)),
        );
    }

    // Background / Foreground
    dict.insert(
        "Background Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.terminal.background)),
    );
    dict.insert(
        "Foreground Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.terminal.foreground)),
    );

    // Cursor
    dict.insert(
        "Cursor Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.terminal.cursor)),
    );
    dict.insert(
        "Cursor Text Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.background)),
    );

    // Selection
    let sel_bg = ir
        .terminal
        .selection_bg
        .as_ref()
        .unwrap_or(&ir.selection_bg);
    dict.insert(
        "Selection Color".to_string(),
        plist::Value::Dictionary(color_dict(sel_bg)),
    );
    dict.insert(
        "Selected Text Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.foreground)),
    );

    // Bold
    dict.insert(
        "Bold Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.terminal.foreground)),
    );

    // Link
    dict.insert(
        "Link Color".to_string(),
        plist::Value::Dictionary(color_dict(&ir.accent)),
    );

    dict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn format_iterm2_theme_has_all_24_keys() {
        let ir = make_test_ir();
        let dict = format_iterm2_theme(&ir);

        // 16 ANSI colors
        for i in 0..16 {
            assert!(
                dict.contains_key(&format!("Ansi {} Color", i)),
                "missing Ansi {} Color",
                i
            );
        }

        // 8 UI colors
        for key in &[
            "Background Color",
            "Foreground Color",
            "Cursor Color",
            "Cursor Text Color",
            "Selection Color",
            "Selected Text Color",
            "Bold Color",
            "Link Color",
        ] {
            assert!(dict.contains_key(key), "missing {}", key);
        }

        assert_eq!(dict.len(), 24);
    }

    #[test]
    fn color_dict_correct_components() {
        let color = HexColor::parse("#FF8040").unwrap();
        let d = color_dict(&color);

        let r = d.get("Red Component").unwrap().as_real().unwrap();
        let g = d.get("Green Component").unwrap().as_real().unwrap();
        let b = d.get("Blue Component").unwrap().as_real().unwrap();
        let a = d.get("Alpha Component").unwrap().as_real().unwrap();
        let cs = d.get("Color Space").unwrap().as_string().unwrap();

        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.502).abs() < 0.01);
        assert!((b - 0.251).abs() < 0.01);
        assert!((a - 1.0).abs() < 0.001);
        assert_eq!(cs, "sRGB");
    }

    #[test]
    fn hex_to_float_roundtrip() {
        let color = HexColor::parse("#804020").unwrap();
        let (r, g, b) = hex_to_float(&color);

        let r8 = (r * 255.0).round() as u8;
        let g8 = (g * 255.0).round() as u8;
        let b8 = (b * 255.0).round() as u8;

        let reconstructed = HexColor::parse(&format!("#{:02x}{:02x}{:02x}", r8, g8, b8)).unwrap();
        assert_eq!(color.as_str(), reconstructed.as_str());
    }

    #[test]
    fn format_iterm2_theme_produces_valid_plist() {
        let ir = make_test_ir();
        let dict = format_iterm2_theme(&ir);

        let mut buf: Vec<u8> = Vec::new();
        plist::to_writer_xml(&mut buf, &dict).expect("plist serialization should succeed");

        let xml = String::from_utf8(buf).expect("plist output should be valid UTF-8");
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("plist"));
    }
}
