//! Color space conversions (RGB ↔ HSL ↔ OKLCH) and palette derivation.

use crate::ir::{AnsiColors, AnsiPalette, HexColor, ThemeIR, ThemeType};

// ── sRGB ↔ Linear RGB ──────────────────────────────────────────────

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ── RGB ↔ HSL ───────────────────────────────────────────────────────

/// Convert RGB to HSL. Returns (H: 0..360, S: 0..100, L: 0..100).
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-10 {
        return (0.0, 0.0, l * 100.0);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - rf).abs() < 1e-10 {
        let mut h = (gf - bf) / d;
        if gf < bf {
            h += 6.0;
        }
        h
    } else if (max - gf).abs() < 1e-10 {
        (bf - rf) / d + 2.0
    } else {
        (rf - gf) / d + 4.0
    };

    (h * 60.0, s * 100.0, l * 100.0)
}

/// Convert HSL to RGB. Input: H: 0..360, S: 0..100, L: 0..100.
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let s = s / 100.0;
    let l = l / 100.0;

    if s.abs() < 1e-10 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return (v, v, v);
    }

    let h = (h / 360.0).rem_euclid(1.0);
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hue_to_rgb = |t: f64| -> f64 {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    let r = (hue_to_rgb(h + 1.0 / 3.0) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    let g = (hue_to_rgb(h) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (hue_to_rgb(h - 1.0 / 3.0) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    (r, g, b)
}

// ── RGB ↔ OKLCH (via OKLab) ─────────────────────────────────────────

fn rgb_to_oklab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = srgb_to_linear(r as f64 / 255.0);
    let g = srgb_to_linear(g as f64 / 255.0);
    let b = srgb_to_linear(b as f64 / 255.0);

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.max(0.0).cbrt();
    let m_ = m.max(0.0).cbrt();
    let s_ = s.max(0.0).cbrt();

    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

fn oklab_to_rgb(l: f64, a: f64, b: f64) -> (u8, u8, u8) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b_val = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    (
        (linear_to_srgb(r.clamp(0.0, 1.0)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
        (linear_to_srgb(g.clamp(0.0, 1.0)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
        (linear_to_srgb(b_val.clamp(0.0, 1.0)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
    )
}

/// Convert RGB to OKLCH. Returns (L: 0..1, C: 0..~0.4, H: 0..360).
pub fn rgb_to_oklch(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (l, a, b_lab) = rgb_to_oklab(r, g, b);
    let c = (a * a + b_lab * b_lab).sqrt();
    let h = if c < 1e-10 {
        0.0
    } else {
        b_lab.atan2(a).to_degrees().rem_euclid(360.0)
    };
    (l, c, h)
}

/// Convert OKLCH to RGB. Input: L: 0..1, C: 0..~0.4, H: 0..360.
pub fn oklch_to_rgb(l: f64, c: f64, h: f64) -> (u8, u8, u8) {
    let h_rad = h.to_radians();
    oklab_to_rgb(l, c * h_rad.cos(), c * h_rad.sin())
}

// ── WCAG ────────────────────────────────────────────────────────────

pub fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * srgb_to_linear(r as f64 / 255.0)
        + 0.7152 * srgb_to_linear(g as f64 / 255.0)
        + 0.0722 * srgb_to_linear(b as f64 / 255.0)
}

pub fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
    let l1 = relative_luminance(fg.0, fg.1, fg.2);
    let l2 = relative_luminance(bg.0, bg.1, bg.2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

// ── Helpers ─────────────────────────────────────────────────────────

pub fn hex_from_rgb(r: u8, g: u8, b: u8) -> HexColor {
    HexColor::parse(&format!("#{r:02X}{g:02X}{b:02X}")).unwrap()
}

fn adjust_lightness(rgb: (u8, u8, u8), delta: f64) -> HexColor {
    let (l, c, h) = rgb_to_oklch(rgb.0, rgb.1, rgb.2);
    let (r, g, b) = oklch_to_rgb((l + delta).clamp(0.0, 1.0), c, h);
    hex_from_rgb(r, g, b)
}

fn ensure_contrast(l: f64, c: f64, h: f64, bg: (u8, u8, u8), min: f64) -> (f64, f64, f64) {
    let lighten = relative_luminance(bg.0, bg.1, bg.2) < 0.5;
    let mut l = l;
    for _ in 0..50 {
        let rgb = oklch_to_rgb(l, c, h);
        if contrast_ratio(rgb, bg) >= min {
            return (l, c, h);
        }
        l += if lighten { 0.02 } else { -0.02 };
        l = l.clamp(0.0, 1.0);
    }
    (l, c, h)
}

fn make_ansi(hue: f64, chroma: f64, lightness: f64, bg: (u8, u8, u8)) -> HexColor {
    let (l, c, h) = ensure_contrast(lightness, chroma, hue, bg, 4.5);
    let (r, g, b) = oklch_to_rgb(l, c, h);
    hex_from_rgb(r, g, b)
}

// ── Palette derivation ──────────────────────────────────────────────

pub fn derive_palette(
    bg: &HexColor,
    fg: &HexColor,
    accent: &HexColor,
    theme_type: &ThemeType,
) -> ThemeIR {
    let bg_rgb = bg.to_rgb();
    let fg_rgb = fg.to_rgb();
    let accent_rgb = accent.to_rgb();
    let (_al, ac, ah) = rgb_to_oklch(accent_rgb.0, accent_rgb.1, accent_rgb.2);

    let is_dark = matches!(theme_type, ThemeType::Dark);
    let sign: f64 = if is_dark { 1.0 } else { -1.0 };

    // UI colors
    let sidebar_bg = adjust_lightness(bg_rgb, -0.03 * sign);
    let input_bg = adjust_lightness(bg_rgb, -0.02 * sign);
    let border = adjust_lightness(bg_rgb, 0.08 * sign);
    let selection_bg = adjust_lightness(bg_rgb, 0.12 * sign);
    let muted_fg = adjust_lightness(fg_rgb, -0.20 * sign);

    // ANSI palette
    let chroma = ac.clamp(0.10, 0.20);
    let ansi_l = if is_dark { 0.72 } else { 0.48 };
    let bright_l = if is_dark { 0.82 } else { 0.38 };

    let normal_black = adjust_lightness(bg_rgb, if is_dark { 0.03 } else { -0.40 });
    let normal_white = adjust_lightness(fg_rgb, if is_dark { -0.08 } else { 0.08 });
    let bright_black = adjust_lightness(bg_rgb, if is_dark { 0.20 } else { -0.20 });
    let bright_white = if is_dark {
        hex_from_rgb(255, 255, 255)
    } else {
        hex_from_rgb(0, 0, 0)
    };

    let normal = AnsiPalette {
        black: normal_black,
        red: make_ansi(25.0, chroma, ansi_l, bg_rgb),
        green: make_ansi(145.0, chroma, ansi_l, bg_rgb),
        yellow: make_ansi(90.0, chroma, ansi_l, bg_rgb),
        blue: make_ansi(265.0, chroma, ansi_l, bg_rgb),
        magenta: make_ansi(330.0, chroma, ansi_l, bg_rgb),
        cyan: make_ansi(195.0, chroma, ansi_l, bg_rgb),
        white: normal_white,
    };

    let bright = AnsiPalette {
        black: bright_black,
        red: make_ansi(25.0, chroma, bright_l, bg_rgb),
        green: make_ansi(145.0, chroma, bright_l, bg_rgb),
        yellow: make_ansi(90.0, chroma, bright_l, bg_rgb),
        blue: make_ansi(265.0, chroma, bright_l, bg_rgb),
        magenta: make_ansi(330.0, chroma, bright_l, bg_rgb),
        cyan: make_ansi(195.0, chroma, bright_l, bg_rgb),
        white: bright_white,
    };

    // Chart colors: accent hue + 72° intervals
    let chart_l = if is_dark { 0.72 } else { 0.52 };
    let chart_c = ac.max(0.10);
    let chart_colors: [HexColor; 5] = std::array::from_fn(|i| {
        let hue = (ah + i as f64 * 72.0) % 360.0;
        let (l, c, h) = ensure_contrast(chart_l, chart_c, hue, bg_rgb, 3.0);
        let (r, g, b) = oklch_to_rgb(l, c, h);
        hex_from_rgb(r, g, b)
    });

    ThemeIR {
        id: String::new(),
        name: String::new(),
        theme_type: theme_type.clone(),
        background: bg.clone(),
        foreground: fg.clone(),
        accent: accent.clone(),
        cursor: fg.clone(),
        selection_bg,
        border,
        sidebar_bg,
        sidebar_fg: fg.clone(),
        input_bg,
        muted_fg,
        chart_colors,
        terminal: AnsiColors {
            normal,
            bright,
            background: bg.clone(),
            foreground: fg.clone(),
            cursor: fg.clone(),
            cursor_accent: None,
            selection_bg: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_roundtrip() {
        for &(r, g, b) in &[
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 64, 200),
            (0, 0, 0),
            (255, 255, 255),
        ] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!(r.abs_diff(r2) <= 1, "HSL roundtrip R: {r} vs {r2}");
            assert!(g.abs_diff(g2) <= 1, "HSL roundtrip G: {g} vs {g2}");
            assert!(b.abs_diff(b2) <= 1, "HSL roundtrip B: {b} vs {b2}");
        }
    }

    #[test]
    fn oklch_roundtrip() {
        for &(r, g, b) in &[
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 64, 200),
            (30, 30, 30),
            (200, 200, 200),
        ] {
            let (l, c, h) = rgb_to_oklch(r, g, b);
            let (r2, g2, b2) = oklch_to_rgb(l, c, h);
            assert!(r.abs_diff(r2) <= 1, "OKLCH roundtrip R: {r} vs {r2}");
            assert!(g.abs_diff(g2) <= 1, "OKLCH roundtrip G: {g} vs {g2}");
            assert!(b.abs_diff(b2) <= 1, "OKLCH roundtrip B: {b} vs {b2}");
        }
    }

    #[test]
    fn wcag_white_on_black() {
        let ratio = contrast_ratio((255, 255, 255), (0, 0, 0));
        assert!((ratio - 21.0).abs() < 0.1);
    }

    #[test]
    fn wcag_same_color() {
        let ratio = contrast_ratio((128, 128, 128), (128, 128, 128));
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn derive_palette_dark() {
        let bg = HexColor::parse("#1E1E1E").unwrap();
        let fg = HexColor::parse("#D4D4D4").unwrap();
        let accent = HexColor::parse("#0078D4").unwrap();
        let ir = derive_palette(&bg, &fg, &accent, &ThemeType::Dark);

        assert_eq!(ir.chart_colors.len(), 5);
        assert_eq!(ir.background.as_str(), "#1E1E1E");

        // All ANSI foreground colors should have sufficient contrast
        let bg_rgb = bg.to_rgb();
        for c in [
            &ir.terminal.normal.red,
            &ir.terminal.normal.green,
            &ir.terminal.normal.blue,
        ] {
            assert!(
                contrast_ratio(c.to_rgb(), bg_rgb) >= 4.5,
                "{c} contrast too low"
            );
        }
    }

    #[test]
    fn derive_palette_light() {
        let bg = HexColor::parse("#FFFFFF").unwrap();
        let fg = HexColor::parse("#333333").unwrap();
        let accent = HexColor::parse("#0078D4").unwrap();
        let ir = derive_palette(&bg, &fg, &accent, &ThemeType::Light);
        assert!(matches!(ir.theme_type, ThemeType::Light));
    }
}
