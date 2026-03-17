use crate::ir::{AnsiColors, AnsiPalette, HexColor, ThemeIR, ThemeType};
use anyhow::Result;

/// Convert an iTerm2 color preset into ThemeIR.
pub fn convert_iterm2(
    name: &str,
    preset: &plist::Dictionary,
    theme_type: ThemeType,
) -> Result<ThemeIR> {
    // ANSI 0-7 (normal)
    let normal = AnsiPalette {
        black: get_color(preset, "Ansi 0 Color")?,
        red: get_color(preset, "Ansi 1 Color")?,
        green: get_color(preset, "Ansi 2 Color")?,
        yellow: get_color(preset, "Ansi 3 Color")?,
        blue: get_color(preset, "Ansi 4 Color")?,
        magenta: get_color(preset, "Ansi 5 Color")?,
        cyan: get_color(preset, "Ansi 6 Color")?,
        white: get_color(preset, "Ansi 7 Color")?,
    };

    // ANSI 8-15 (bright)
    let bright = AnsiPalette {
        black: get_color(preset, "Ansi 8 Color")?,
        red: get_color(preset, "Ansi 9 Color")?,
        green: get_color(preset, "Ansi 10 Color")?,
        yellow: get_color(preset, "Ansi 11 Color")?,
        blue: get_color(preset, "Ansi 12 Color")?,
        magenta: get_color(preset, "Ansi 13 Color")?,
        cyan: get_color(preset, "Ansi 14 Color")?,
        white: get_color(preset, "Ansi 15 Color")?,
    };

    let background = get_color(preset, "Background Color")?;
    let foreground = get_color(preset, "Foreground Color")?;
    let cursor = get_color(preset, "Cursor Color")?;
    let selection_bg = get_color(preset, "Selection Color").ok();

    let bg_rgb = background.to_rgb();
    let is_dark = matches!(theme_type, ThemeType::Dark);
    let delta_sign: f64 = if is_dark { 1.0 } else { -1.0 };

    // Derived UI fields
    let accent = normal.blue.clone();
    let muted_fg = bright.black.clone();
    let border = crate::color::adjust_lightness(bg_rgb, 0.05 * delta_sign);
    let sidebar_bg = crate::color::adjust_lightness(bg_rgb, 0.02 * delta_sign);
    let sidebar_fg = foreground.clone();
    let input_bg = crate::color::adjust_lightness(bg_rgb, -0.02 * delta_sign);

    let chart_colors = [
        normal.red.clone(),
        normal.green.clone(),
        normal.yellow.clone(),
        normal.blue.clone(),
        normal.magenta.clone(),
    ];

    let id = crate::store::theme_slug(name);

    Ok(ThemeIR {
        id,
        name: name.to_string(),
        theme_type,
        background: background.clone(),
        foreground: foreground.clone(),
        accent,
        cursor: cursor.clone(),
        selection_bg: selection_bg.clone().unwrap_or_else(|| background.clone()),
        border,
        sidebar_bg,
        sidebar_fg,
        input_bg,
        muted_fg,
        chart_colors,
        terminal: AnsiColors {
            normal,
            bright,
            background,
            foreground,
            cursor,
            cursor_accent: None,
            selection_bg,
        },
        syntax: None,
        diff: None,
        created_at: None,
    })
}

/// Sanitize a plist float component: NaN and Infinity become 0.
fn sanitize_component(v: f64) -> u8 {
    if v.is_nan() || v.is_infinite() {
        return 0;
    }
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn plist_color_to_hex(color_dict: &plist::Dictionary) -> Result<HexColor> {
    let r = color_dict
        .get("Red Component")
        .and_then(|v| v.as_real())
        .unwrap_or(0.0);
    let g = color_dict
        .get("Green Component")
        .and_then(|v| v.as_real())
        .unwrap_or(0.0);
    let b = color_dict
        .get("Blue Component")
        .and_then(|v| v.as_real())
        .unwrap_or(0.0);
    let r = sanitize_component(r);
    let g = sanitize_component(g);
    let b = sanitize_component(b);
    Ok(crate::color::hex_from_rgb(r, g, b))
}

fn get_color(preset: &plist::Dictionary, key: &str) -> Result<HexColor> {
    let dict = preset
        .get(key)
        .and_then(|v| v.as_dictionary())
        .ok_or_else(|| anyhow::anyhow!("missing color key: {}", key))?;
    plist_color_to_hex(dict)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_color_dict(r: f64, g: f64, b: f64) -> plist::Dictionary {
        let mut d = plist::Dictionary::new();
        d.insert("Red Component".to_string(), plist::Value::Real(r));
        d.insert("Green Component".to_string(), plist::Value::Real(g));
        d.insert("Blue Component".to_string(), plist::Value::Real(b));
        d.insert("Alpha Component".to_string(), plist::Value::Real(1.0));
        d.insert(
            "Color Space".to_string(),
            plist::Value::String("sRGB".to_string()),
        );
        d
    }

    #[test]
    fn plist_color_to_hex_normal_values() {
        let d = make_color_dict(1.0, 0.5, 0.0);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#FF8000");
    }

    #[test]
    fn plist_color_to_hex_boundary_zero() {
        let d = make_color_dict(0.0, 0.0, 0.0);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#000000");
    }

    #[test]
    fn plist_color_to_hex_boundary_one() {
        let d = make_color_dict(1.0, 1.0, 1.0);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#FFFFFF");
    }

    #[test]
    fn plist_color_to_hex_out_of_range_clamped() {
        let d = make_color_dict(1.5, -0.5, 2.0);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#FF00FF");
    }

    #[test]
    fn plist_color_to_hex_missing_components_default_zero() {
        let d = plist::Dictionary::new();
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#000000");
    }

    #[test]
    fn plist_color_to_hex_nan_treated_as_zero() {
        let d = make_color_dict(f64::NAN, 0.5, f64::NAN);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#008000");
    }

    #[test]
    fn plist_color_to_hex_infinity_treated_as_zero() {
        let d = make_color_dict(f64::INFINITY, f64::NEG_INFINITY, 0.5);
        let hex = plist_color_to_hex(&d).unwrap();
        assert_eq!(hex.as_str(), "#000080");
    }

    #[test]
    fn get_color_missing_key_returns_error() {
        let preset = plist::Dictionary::new();
        let result = get_color(&preset, "Nonexistent Color");
        assert!(result.is_err());
    }

    #[test]
    fn convert_iterm2_full_preset() {
        let mut preset = plist::Dictionary::new();

        // Set up all 20 required colors
        let colors = [
            ("Ansi 0 Color", (0.0, 0.0, 0.0)),
            ("Ansi 1 Color", (0.8, 0.0, 0.0)),
            ("Ansi 2 Color", (0.0, 0.8, 0.0)),
            ("Ansi 3 Color", (0.8, 0.8, 0.0)),
            ("Ansi 4 Color", (0.0, 0.0, 0.8)),
            ("Ansi 5 Color", (0.8, 0.0, 0.8)),
            ("Ansi 6 Color", (0.0, 0.8, 0.8)),
            ("Ansi 7 Color", (0.8, 0.8, 0.8)),
            ("Ansi 8 Color", (0.3, 0.3, 0.3)),
            ("Ansi 9 Color", (1.0, 0.0, 0.0)),
            ("Ansi 10 Color", (0.0, 1.0, 0.0)),
            ("Ansi 11 Color", (1.0, 1.0, 0.0)),
            ("Ansi 12 Color", (0.0, 0.0, 1.0)),
            ("Ansi 13 Color", (1.0, 0.0, 1.0)),
            ("Ansi 14 Color", (0.0, 1.0, 1.0)),
            ("Ansi 15 Color", (1.0, 1.0, 1.0)),
            ("Background Color", (0.1, 0.1, 0.1)),
            ("Foreground Color", (0.9, 0.9, 0.9)),
            ("Cursor Color", (0.9, 0.9, 0.9)),
            ("Selection Color", (0.3, 0.3, 0.5)),
        ];

        for (key, (r, g, b)) in &colors {
            preset.insert(
                key.to_string(),
                plist::Value::Dictionary(make_color_dict(*r, *g, *b)),
            );
        }

        let ir = convert_iterm2("Test Preset", &preset, ThemeType::Dark).unwrap();

        assert_eq!(ir.name, "Test Preset");
        assert_eq!(ir.id, "test-preset");
        assert_eq!(ir.terminal.normal.black.as_str(), "#000000");
        assert_eq!(ir.terminal.bright.white.as_str(), "#FFFFFF");
        assert_eq!(ir.terminal.background.as_str(), "#1A1A1A");
        assert_eq!(ir.accent.as_str(), ir.terminal.normal.blue.as_str());
        assert!(ir.syntax.is_none());
        assert!(ir.diff.is_none());
    }
}
