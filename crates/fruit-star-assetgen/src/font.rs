//! "Fruit Star Stroke" — a small original vector stroke font.
//!
//! Every glyph is a handful of polylines and elliptical arcs on a unit grid
//! (cap height = 1.0, y grows downwards, baseline at y = 1.0). The glyphs were
//! drawn for this project and are rendered as SVG `<path>` data that callers
//! stroke with thick round caps and joins. No system or third-party font is
//! involved anywhere in the tool.

use std::fmt::Write as _;

use anyhow::{Result, bail};

use Seg::{A, L, M, Q, Z};

/// One drawing command of a glyph, in unit-grid coordinates.
#[derive(Debug, Clone, Copy)]
enum Seg {
    /// Move to.
    M(f32, f32),
    /// Line to.
    L(f32, f32),
    /// Quadratic curve: control point, end point.
    Q(f32, f32, f32, f32),
    /// Elliptical arc: rx, ry, large-arc flag, sweep flag, end point.
    A(f32, f32, bool, bool, f32, f32),
    /// Close the current sub-path.
    Z,
}

/// Glyph outline plus its advance width (excluding tracking).
#[derive(Debug, Clone, Copy)]
struct Glyph {
    width: f32,
    segs: &'static [Seg],
}

const fn g(width: f32, segs: &'static [Seg]) -> Glyph {
    Glyph { width, segs }
}

/// Advance of the space character in cap-height units.
const SPACE_WIDTH: f32 = 0.42;

fn glyph(c: char) -> Option<Glyph> {
    Some(match c {
        ' ' => g(SPACE_WIDTH, &[]),
        'A' => g(
            0.72,
            &[
                M(0.0, 1.0),
                L(0.36, 0.0),
                L(0.72, 1.0),
                M(0.13, 0.66),
                L(0.59, 0.66),
            ],
        ),
        'B' => g(
            0.62,
            &[
                M(0.0, 0.0),
                L(0.34, 0.0),
                A(0.24, 0.24, false, true, 0.34, 0.48),
                L(0.0, 0.48),
                M(0.34, 0.48),
                L(0.36, 0.48),
                A(0.26, 0.26, false, true, 0.36, 1.0),
                L(0.0, 1.0),
                L(0.0, 0.0),
            ],
        ),
        'C' => g(
            0.64,
            &[
                M(0.64, 0.28),
                A(0.28, 0.28, false, false, 0.36, 0.0),
                L(0.28, 0.0),
                A(0.28, 0.28, false, false, 0.0, 0.28),
                L(0.0, 0.72),
                A(0.28, 0.28, false, false, 0.28, 1.0),
                L(0.36, 1.0),
                A(0.28, 0.28, false, false, 0.64, 0.72),
            ],
        ),
        'D' => g(
            0.64,
            &[
                M(0.0, 0.0),
                L(0.32, 0.0),
                A(0.32, 0.32, false, true, 0.64, 0.32),
                L(0.64, 0.68),
                A(0.32, 0.32, false, true, 0.32, 1.0),
                L(0.0, 1.0),
                Z,
            ],
        ),
        'E' => g(
            0.56,
            &[
                M(0.56, 0.0),
                L(0.0, 0.0),
                L(0.0, 1.0),
                L(0.56, 1.0),
                M(0.0, 0.5),
                L(0.46, 0.5),
            ],
        ),
        'F' => g(
            0.54,
            &[
                M(0.54, 0.0),
                L(0.0, 0.0),
                L(0.0, 1.0),
                M(0.0, 0.5),
                L(0.44, 0.5),
            ],
        ),
        'G' => g(
            0.66,
            &[
                M(0.66, 0.28),
                A(0.28, 0.28, false, false, 0.38, 0.0),
                L(0.28, 0.0),
                A(0.28, 0.28, false, false, 0.0, 0.28),
                L(0.0, 0.72),
                A(0.28, 0.28, false, false, 0.28, 1.0),
                L(0.38, 1.0),
                A(0.28, 0.28, false, false, 0.66, 0.72),
                L(0.66, 0.54),
                L(0.38, 0.54),
            ],
        ),
        'H' => g(
            0.64,
            &[
                M(0.0, 0.0),
                L(0.0, 1.0),
                M(0.64, 0.0),
                L(0.64, 1.0),
                M(0.0, 0.5),
                L(0.64, 0.5),
            ],
        ),
        'I' => g(0.0, &[M(0.0, 0.0), L(0.0, 1.0)]),
        'J' => g(
            0.5,
            &[
                M(0.5, 0.0),
                L(0.5, 0.72),
                A(0.25, 0.28, false, true, 0.25, 1.0),
                A(0.25, 0.28, false, true, 0.0, 0.72),
            ],
        ),
        'K' => g(
            0.62,
            &[
                M(0.0, 0.0),
                L(0.0, 1.0),
                M(0.6, 0.0),
                L(0.0, 0.58),
                M(0.2, 0.44),
                L(0.62, 1.0),
            ],
        ),
        'L' => g(0.52, &[M(0.0, 0.0), L(0.0, 1.0), L(0.52, 1.0)]),
        'M' => g(
            0.8,
            &[
                M(0.0, 1.0),
                L(0.0, 0.0),
                L(0.4, 0.62),
                L(0.8, 0.0),
                L(0.8, 1.0),
            ],
        ),
        'N' => g(
            0.66,
            &[M(0.0, 1.0), L(0.0, 0.0), L(0.66, 1.0), L(0.66, 0.0)],
        ),
        'O' => g(
            0.68,
            &[
                M(0.3, 0.0),
                L(0.38, 0.0),
                A(0.3, 0.3, false, true, 0.68, 0.3),
                L(0.68, 0.7),
                A(0.3, 0.3, false, true, 0.38, 1.0),
                L(0.3, 1.0),
                A(0.3, 0.3, false, true, 0.0, 0.7),
                L(0.0, 0.3),
                A(0.3, 0.3, false, true, 0.3, 0.0),
                Z,
            ],
        ),
        'P' => g(
            0.6,
            &[
                M(0.0, 1.0),
                L(0.0, 0.0),
                L(0.33, 0.0),
                A(0.27, 0.27, false, true, 0.33, 0.54),
                L(0.0, 0.54),
            ],
        ),
        'Q' => g(
            0.68,
            &[
                M(0.3, 0.0),
                L(0.38, 0.0),
                A(0.3, 0.3, false, true, 0.68, 0.3),
                L(0.68, 0.7),
                A(0.3, 0.3, false, true, 0.38, 1.0),
                L(0.3, 1.0),
                A(0.3, 0.3, false, true, 0.0, 0.7),
                L(0.0, 0.3),
                A(0.3, 0.3, false, true, 0.3, 0.0),
                Z,
                M(0.42, 0.72),
                L(0.72, 1.04),
            ],
        ),
        'R' => g(
            0.62,
            &[
                M(0.0, 1.0),
                L(0.0, 0.0),
                L(0.33, 0.0),
                A(0.27, 0.27, false, true, 0.33, 0.54),
                L(0.0, 0.54),
                M(0.3, 0.54),
                L(0.62, 1.0),
            ],
        ),
        'S' => g(
            0.6,
            &[
                M(0.6, 0.25),
                A(0.25, 0.25, false, false, 0.35, 0.0),
                L(0.25, 0.0),
                A(0.25, 0.25, false, false, 0.25, 0.5),
                L(0.35, 0.5),
                A(0.25, 0.25, false, true, 0.35, 1.0),
                L(0.25, 1.0),
                A(0.25, 0.25, false, true, 0.0, 0.75),
            ],
        ),
        'T' => g(
            0.64,
            &[M(0.0, 0.0), L(0.64, 0.0), M(0.32, 0.0), L(0.32, 1.0)],
        ),
        'U' => g(
            0.66,
            &[
                M(0.0, 0.0),
                L(0.0, 0.7),
                A(0.3, 0.3, false, false, 0.3, 1.0),
                L(0.36, 1.0),
                A(0.3, 0.3, false, false, 0.66, 0.7),
                L(0.66, 0.0),
            ],
        ),
        'V' => g(0.7, &[M(0.0, 0.0), L(0.35, 1.0), L(0.7, 0.0)]),
        'W' => g(
            0.96,
            &[
                M(0.0, 0.0),
                L(0.22, 1.0),
                L(0.48, 0.25),
                L(0.74, 1.0),
                L(0.96, 0.0),
            ],
        ),
        'X' => g(
            0.66,
            &[M(0.0, 0.0), L(0.66, 1.0), M(0.66, 0.0), L(0.0, 1.0)],
        ),
        'Y' => g(
            0.68,
            &[
                M(0.0, 0.0),
                L(0.34, 0.52),
                L(0.68, 0.0),
                M(0.34, 0.52),
                L(0.34, 1.0),
            ],
        ),
        'Z' => g(
            0.62,
            &[M(0.0, 0.0), L(0.62, 0.0), L(0.0, 1.0), L(0.62, 1.0)],
        ),
        '0' => g(
            0.6,
            &[
                M(0.28, 0.0),
                L(0.32, 0.0),
                A(0.28, 0.28, false, true, 0.6, 0.28),
                L(0.6, 0.72),
                A(0.28, 0.28, false, true, 0.32, 1.0),
                L(0.28, 1.0),
                A(0.28, 0.28, false, true, 0.0, 0.72),
                L(0.0, 0.28),
                A(0.28, 0.28, false, true, 0.28, 0.0),
                Z,
            ],
        ),
        '1' => g(
            0.56,
            &[
                M(0.06, 0.24),
                L(0.3, 0.0),
                L(0.3, 1.0),
                M(0.04, 1.0),
                L(0.56, 1.0),
            ],
        ),
        '2' => g(
            0.6,
            &[
                M(0.02, 0.27),
                A(0.28, 0.27, false, true, 0.3, 0.0),
                A(0.28, 0.27, false, true, 0.58, 0.27),
                Q(0.58, 0.43, 0.42, 0.57),
                L(0.0, 1.0),
                L(0.6, 1.0),
            ],
        ),
        '3' => g(
            0.6,
            &[
                M(0.04, 0.18),
                Q(0.1, 0.0, 0.3, 0.0),
                L(0.32, 0.0),
                A(0.24, 0.24, false, true, 0.32, 0.48),
                L(0.2, 0.48),
                M(0.32, 0.48),
                A(0.26, 0.26, false, true, 0.32, 1.0),
                L(0.28, 1.0),
                Q(0.08, 1.0, 0.02, 0.82),
            ],
        ),
        '4' => g(
            0.64,
            &[M(0.46, 1.0), L(0.46, 0.0), L(0.0, 0.7), L(0.64, 0.7)],
        ),
        '5' => g(
            0.6,
            &[
                M(0.56, 0.0),
                L(0.08, 0.0),
                L(0.05, 0.42),
                Q(0.15, 0.38, 0.29, 0.38),
                A(0.31, 0.31, false, true, 0.29, 1.0),
                Q(0.1, 1.0, 0.02, 0.84),
            ],
        ),
        '6' => g(
            0.6,
            &[
                M(0.56, 0.14),
                Q(0.46, 0.0, 0.3, 0.0),
                A(0.3, 0.3, false, false, 0.0, 0.3),
                L(0.0, 0.7),
                A(0.3, 0.3, true, false, 0.6, 0.7),
                A(0.3, 0.3, true, false, 0.0, 0.7),
            ],
        ),
        '7' => g(0.6, &[M(0.0, 0.0), L(0.6, 0.0), L(0.22, 1.0)]),
        '8' => g(
            0.6,
            &[
                M(0.3, 0.0),
                A(0.24, 0.24, true, true, 0.3, 0.48),
                A(0.24, 0.24, true, true, 0.3, 0.0),
                M(0.3, 0.48),
                A(0.26, 0.26, true, true, 0.3, 1.0),
                A(0.26, 0.26, true, true, 0.3, 0.48),
            ],
        ),
        '9' => g(
            0.6,
            &[
                M(0.04, 0.86),
                Q(0.14, 1.0, 0.3, 1.0),
                A(0.3, 0.3, false, false, 0.6, 0.7),
                L(0.6, 0.3),
                A(0.3, 0.3, true, false, 0.0, 0.3),
                A(0.3, 0.3, true, false, 0.6, 0.3),
            ],
        ),
        '-' => g(0.4, &[M(0.0, 0.5), L(0.4, 0.5)]),
        '!' => g(0.0, &[M(0.0, 0.0), L(0.0, 0.66), M(0.0, 0.98), L(0.0, 1.0)]),
        _ => return None,
    })
}

/// Pair adjustments (in cap-height units) for the few combinations whose
/// diagonal shapes would otherwise leave a visible hole.
fn kerning(left: char, right: char) -> f32 {
    match (left, right) {
        ('T', 'A') | ('A', 'T') => -0.12,
        ('A', 'Y') | ('Y', 'A') => -0.14,
        ('P', 'A') | ('F', 'A') => -0.1,
        ('A', 'V') | ('V', 'A') => -0.14,
        ('L', 'T') | ('L', 'Y') => -0.08,
        ('Y', 'O') => -0.05,
        _ => 0.0,
    }
}

/// Returns the advance width of `text` at the given cap height and tracking.
pub fn text_width(text: &str, cap: f32, tracking: f32) -> Result<f32> {
    let mut pen = 0.0f32;
    let mut prev: Option<char> = None;
    for c in text.chars() {
        let Some(gl) = glyph(c) else {
            bail!("stroke font has no glyph for {c:?}");
        };
        if let Some(p) = prev {
            pen += tracking + kerning(p, c);
        }
        pen += gl.width;
        prev = Some(c);
    }
    Ok(pen * cap)
}

/// Lays out `text` with its left edge at `x` and its cap line at `top`, returning
/// SVG path data for the strokes' centre lines.
pub fn text_path(text: &str, x: f32, top: f32, cap: f32, tracking: f32) -> Result<String> {
    let mut d = String::new();
    let mut pen = 0.0f32;
    let mut prev: Option<char> = None;
    for c in text.chars() {
        let Some(gl) = glyph(c) else {
            bail!("stroke font has no glyph for {c:?}");
        };
        if let Some(p) = prev {
            pen += tracking + kerning(p, c);
        }
        let px = |ux: f32| x + (pen + ux) * cap;
        let py = |uy: f32| top + uy * cap;
        for seg in gl.segs {
            match *seg {
                M(ux, uy) => {
                    let _ = write!(d, "M{:.2},{:.2} ", px(ux), py(uy));
                }
                L(ux, uy) => {
                    let _ = write!(d, "L{:.2},{:.2} ", px(ux), py(uy));
                }
                Q(cx, cy, ux, uy) => {
                    let _ = write!(
                        d,
                        "Q{:.2},{:.2} {:.2},{:.2} ",
                        px(cx),
                        py(cy),
                        px(ux),
                        py(uy)
                    );
                }
                A(rx, ry, large, sweep, ux, uy) => {
                    let _ = write!(
                        d,
                        "A{:.2},{:.2} 0 {} {} {:.2},{:.2} ",
                        rx * cap,
                        ry * cap,
                        u8::from(large),
                        u8::from(sweep),
                        px(ux),
                        py(uy)
                    );
                }
                Z => d.push_str("Z "),
            }
        }
        pen += gl.width;
        prev = Some(c);
    }
    Ok(d.trim_end().to_owned())
}

/// Lays out `text` centred on (`cx`, `cy`), where `cy` is the middle of the cap height.
pub fn text_path_centered(text: &str, cx: f32, cy: f32, cap: f32, tracking: f32) -> Result<String> {
    let width = text_width(text, cap, tracking)?;
    text_path(text, cx - width / 2.0, cy - cap / 2.0, cap, tracking)
}

/// SVG markup that strokes `d` with round caps and joins.
pub fn stroke(d: &str, color: &str, width: f32, extra: &str) -> String {
    format!(
        "<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\" {extra}/>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_required_character_set() {
        for c in ('A'..='Z').chain('0'..='9').chain([' ']) {
            assert!(glyph(c).is_some(), "missing glyph {c:?}");
        }
    }

    #[test]
    fn every_glyph_starts_with_a_move_and_stays_on_the_grid() {
        for c in ('A'..='Z').chain('0'..='9').chain(['-', '!']) {
            let gl = glyph(c).unwrap();
            assert!(matches!(gl.segs.first(), Some(M(..))), "glyph {c:?}");
            for seg in gl.segs {
                let (x, y) = match *seg {
                    M(x, y) | L(x, y) | Q(_, _, x, y) | A(_, _, _, _, x, y) => (x, y),
                    Z => continue,
                };
                assert!((-0.01..=gl.width + 0.08).contains(&x), "glyph {c:?} x={x}");
                assert!((-0.01..=1.05).contains(&y), "glyph {c:?} y={y}");
            }
        }
    }

    #[test]
    fn layout_width_matches_path_width_and_centres() {
        // "BAR": B (0.62) + A (0.72) + R (0.62) plus two gaps of tracking.
        let w = text_width("BAR", 100.0, 0.2).unwrap();
        assert!((w - 236.0).abs() < 1e-3);
        let p = text_path("BAR", 10.0, 0.0, 100.0, 0.2).unwrap();
        assert!(p.starts_with("M10.00,0.00 "));
        // Centred text starts half its width to the left of the centre.
        let c = text_path_centered("BAR", 500.0, 100.0, 100.0, 0.2).unwrap();
        assert!(c.starts_with(&format!("M{:.2},{:.2} ", 500.0 - w / 2.0, 50.0)));
        // Kerned pairs are tighter than the plain sum of their advances.
        let plain = text_width("T", 100.0, 0.2).unwrap() + text_width("A", 100.0, 0.2).unwrap();
        assert!(text_width("TA", 100.0, 0.2).unwrap() < plain + 20.0);
    }

    #[test]
    fn unknown_characters_are_rejected() {
        assert!(text_path("a", 0.0, 0.0, 10.0, 0.1).is_err());
    }
}
