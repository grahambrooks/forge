//! Font-accurate text measurement for SVG layout.
//!
//! Bundles Inter Regular and Inter Bold font files and uses `ab_glyph`
//! to compute actual glyph advance widths with kerning. This replaces
//! the naive `chars * constant_px` estimation.

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

// ─── Font Specs ─────────────────────────────────────────────────
// Single source of truth for font sizes/weights matching the SVG CSS.

#[derive(Debug, Clone, Copy)]
pub struct FontSpec {
    pub size: f64,
    pub bold: bool,
    pub line_height: f64,
}

#[allow(dead_code)]
pub const FONT_TITLE: FontSpec = FontSpec {
    size: 20.0,
    bold: true,
    line_height: 24.0,
};
pub const FONT_NAME: FontSpec = FontSpec {
    size: 14.0,
    bold: true,
    line_height: 16.0,
};
pub const FONT_DESC: FontSpec = FontSpec {
    size: 11.0,
    bold: false,
    line_height: 14.0,
};
pub const FONT_TECH: FontSpec = FontSpec {
    size: 11.0,
    bold: false,
    line_height: 14.0,
};
pub const FONT_KIND: FontSpec = FontSpec {
    size: 10.0,
    bold: false,
    line_height: 14.0,
};
pub const FONT_GATE: FontSpec = FontSpec {
    size: 9.0,
    bold: false,
    line_height: 12.0,
};
pub const FONT_ENTITY_FIELD: FontSpec = FontSpec {
    size: 11.0,
    bold: true,
    line_height: 14.0,
};
pub const FONT_ENTITY_TYPE: FontSpec = FontSpec {
    size: 10.0,
    bold: false,
    line_height: 14.0,
};
pub const FONT_ENTITY_SUB: FontSpec = FontSpec {
    size: 9.0,
    bold: false,
    line_height: 12.0,
};
pub const FONT_REL: FontSpec = FontSpec {
    size: 11.0,
    bold: false,
    line_height: 14.0,
};
pub const FONT_REL_TECH: FontSpec = FontSpec {
    size: 10.0,
    bold: false,
    line_height: 14.0,
};
#[allow(dead_code)]
pub const FONT_DEPLOY_NAME: FontSpec = FontSpec {
    size: 13.0,
    bold: false,
    line_height: 16.0,
};
#[allow(dead_code)]
pub const FONT_DEPLOY_TECH: FontSpec = FontSpec {
    size: 11.0,
    bold: false,
    line_height: 14.0,
};

// ─── TextMeasurer ───────────────────────────────────────────────

pub struct TextMeasurer {
    regular: FontArc,
    bold: FontArc,
}

impl TextMeasurer {
    pub fn new() -> Self {
        Self {
            regular: FontArc::try_from_slice(include_bytes!("fonts/Inter-Regular.ttf"))
                .expect("failed to load Inter-Regular.ttf"),
            bold: FontArc::try_from_slice(include_bytes!("fonts/Inter-Bold.ttf"))
                .expect("failed to load Inter-Bold.ttf"),
        }
    }

    /// Measure the pixel width of a string at a given font size and weight.
    pub fn text_width(&self, text: &str, font_size: f64, bold: bool) -> f64 {
        let font = if bold { &self.bold } else { &self.regular };
        let scaled = font.as_scaled(PxScale::from(font_size as f32));
        let mut width: f32 = 0.0;
        let mut prev_glyph_id = None;
        for ch in text.chars() {
            let gid = scaled.glyph_id(ch);
            width += scaled.h_advance(gid);
            if let Some(prev) = prev_glyph_id {
                width += scaled.kern(prev, gid);
            }
            prev_glyph_id = Some(gid);
        }
        width as f64
    }

    /// Shorthand: measure using a FontSpec.
    pub fn measure(&self, text: &str, spec: &FontSpec) -> f64 {
        self.text_width(text, spec.size, spec.bold)
    }

    /// Wrap text into lines that fit within `max_w` pixels.
    pub fn wrap_text(&self, text: &str, max_w: f64, font_size: f64, bold: bool) -> Vec<String> {
        let space_w = self.text_width(" ", font_size, bold);
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut line: Vec<&str> = Vec::new();
        let mut line_w = 0.0;

        for word in &words {
            let word_w = self.text_width(word, font_size, bold);
            if line_w + word_w > max_w && !line.is_empty() {
                lines.push(line.join(" "));
                line = vec![word];
                line_w = word_w;
            } else {
                if !line.is_empty() {
                    line_w += space_w;
                }
                line.push(word);
                line_w += word_w;
            }
        }
        if !line.is_empty() {
            lines.push(line.join(" "));
        }
        lines
    }

    /// Shorthand: wrap using a FontSpec.
    pub fn wrap(&self, text: &str, max_w: f64, spec: &FontSpec) -> Vec<String> {
        self.wrap_text(text, max_w, spec.size, spec.bold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_width_non_zero() {
        let tm = TextMeasurer::new();
        let w = tm.text_width("Hello", 14.0, false);
        assert!(w > 0.0, "text width should be positive");
    }

    #[test]
    fn bold_wider_than_regular() {
        let tm = TextMeasurer::new();
        let regular = tm.text_width("Payment Service", 14.0, false);
        let bold = tm.text_width("Payment Service", 14.0, true);
        assert!(bold > regular, "bold should be wider than regular");
    }

    #[test]
    fn empty_string_zero_width() {
        let tm = TextMeasurer::new();
        let w = tm.text_width("", 14.0, false);
        assert!((w - 0.0).abs() < 0.001);
    }

    #[test]
    fn wider_chars_wider_result() {
        let tm = TextMeasurer::new();
        let narrow = tm.text_width("iii", 14.0, false);
        let wide = tm.text_width("WWW", 14.0, false);
        assert!(wide > narrow, "WWW should be wider than iii");
    }

    #[test]
    fn wrap_single_line() {
        let tm = TextMeasurer::new();
        let lines = tm.wrap_text("short text", 500.0, 11.0, false);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn wrap_forces_break() {
        let tm = TextMeasurer::new();
        let lines = tm.wrap_text("this is a longer description that should wrap", 60.0, 11.0, false);
        assert!(lines.len() > 1, "should wrap into multiple lines");
    }

    #[test]
    fn font_spec_measure() {
        let tm = TextMeasurer::new();
        let w = tm.measure("Test", &FONT_NAME);
        assert!(w > 0.0);
    }
}
