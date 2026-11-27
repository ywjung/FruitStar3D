//! The application icon: a macOS-style rounded tile with a red seven on a gold star.
//!
//! The artwork reuses the reel symbols, so the icon matches what the player sees on
//! the reels. It is rendered straight from the vector source at every size macOS
//! asks for, instead of scaling one bitmap down.

use std::fmt::Write as _;

use anyhow::Result;

use crate::svg::{self, Doc};
use crate::symbols;

/// Canvas size of the vector source.
pub const CANVAS: u32 = 1024;

/// `(file name, pixel size)` of every member of a macOS `.iconset` directory.
pub const ICONSET: [(&str, u32); 10] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

/// Centre of the tile.
const CENTRE: f32 = 512.0;
/// Half the tile's side. macOS icons leave a margin around the tile for the shadow.
const HALF: f32 = 412.0;

/// Path of a superellipse ("squircle") centred on (`cx`, `cy`).
///
/// A superellipse keeps its curvature continuous where a rounded rectangle would
/// switch abruptly from straight edge to circular arc, which is what gives macOS
/// icons their shape.
fn squircle(cx: f32, cy: f32, half: f32) -> String {
    const EXPONENT: f32 = 5.0;
    const STEPS: usize = 240;
    let mut d = String::new();
    for i in 0..STEPS {
        let t = i as f32 / STEPS as f32 * std::f32::consts::TAU;
        let (sin, cos) = t.sin_cos();
        let x = cx + half * cos.signum() * cos.abs().powf(2.0 / EXPONENT);
        let y = cy + half * sin.signum() * sin.abs().powf(2.0 / EXPONENT);
        let _ = write!(d, "{}{x:.2},{y:.2} ", if i == 0 { 'M' } else { 'L' });
    }
    d.push('Z');
    d
}

/// Alternating sunburst wedges around (`cx`, `cy`) as one path.
fn rays(cx: f32, cy: f32, radius: f32, count: usize, phase: f32) -> String {
    let mut d = String::new();
    let step = std::f32::consts::TAU / count as f32;
    for i in (0..count).step_by(2) {
        let (a0, a1) = (phase + i as f32 * step, phase + (i + 1) as f32 * step);
        let _ = write!(
            d,
            "M{cx},{cy} L{:.1},{:.1} L{:.1},{:.1} Z ",
            cx + radius * a0.cos(),
            cy + radius * a0.sin(),
            cx + radius * a1.cos(),
            cy + radius * a1.sin(),
        );
    }
    d
}

/// Transform that draws a 1024×512 reel symbol centred on (`cx`, `cy`).
fn symbol_at(cx: f32, cy: f32, scale: f32, degrees: f32) -> String {
    format!("translate({cx} {cy}) rotate({degrees}) scale({scale}) translate(-512 -256)")
}

/// Builds the icon's SVG document.
pub fn app_icon_svg() -> Result<String> {
    icon_svg(false)
}

/// iPhone용 아이콘: 캔버스 전체를 채운 불투명한 정사각형. 둥근 모서리는 iOS가 씌운다.
pub fn app_icon_ios_svg() -> Result<String> {
    icon_svg(true)
}

fn icon_svg(ios: bool) -> Result<String> {
    let mut doc = Doc::new(CANVAS, CANVAS);
    let tile = if ios {
        format!("M0,0 H{CANVAS} V{CANVAS} H0 Z")
    } else {
        squircle(CENTRE, CENTRE, HALF)
    };
    // 금색 테두리: iOS는 시스템이 모서리를 깎으므로 그 안쪽에 둔다.
    let rim_half = if ios { 462.0 } else { HALF - 22.0 };

    doc.defs(&format!(
        "<clipPath id=\"icon-clip\"><path d=\"{tile}\"/></clipPath>"
    ));
    doc.defs(&svg::radial(
        "icon-base",
        CENTRE,
        430.0,
        640.0,
        &[
            (0.0, "#e8323a"),
            (0.36, "#b5121f"),
            (0.74, "#6f0716"),
            (1.0, "#3a020c"),
        ],
    ));
    doc.defs(&svg::linear(
        "icon-rim",
        0.0,
        100.0,
        0.0,
        924.0,
        &[
            (0.0, "#fff3b8"),
            (0.35, "#f7c23c"),
            (0.7, "#c9861a"),
            (1.0, "#8a5208"),
        ],
    ));
    doc.defs(&svg::linear(
        "icon-gloss",
        0.0,
        100.0,
        0.0,
        560.0,
        &[(0.0, "#ffffff"), (1.0, "#ffffff")],
    ));
    doc.defs(&svg::radial(
        "icon-glow",
        CENTRE,
        500.0,
        400.0,
        &[(0.0, "#fff2a8"), (0.55, "#ffb347"), (1.0, "#ff7a2a")],
    ));

    // Soft shadow under the tile, built from stacked translucent copies (no filters,
    // so the result is identical in every SVG renderer).
    let shadows: &[(f32, f32, f32)] = if ios {
        &[]
    } else {
        &[(26.0, 6.0, 0.05), (18.0, 3.0, 0.07), (10.0, 0.0, 0.10)]
    };
    for &(offset, grow, opacity) in shadows {
        doc.push(&format!(
            "<path d=\"{}\" fill=\"#000000\" opacity=\"{opacity}\"/>",
            squircle(CENTRE, CENTRE + offset, HALF + grow)
        ));
    }

    // Tile: deep red with a sunburst.
    doc.push(&format!("<path d=\"{tile}\" fill=\"url(#icon-base)\"/>"));
    doc.push(&format!(
        "<g clip-path=\"url(#icon-clip)\">\
         <path d=\"{}\" fill=\"#ffe08a\" opacity=\"0.13\"/>\
         <circle cx=\"{CENTRE}\" cy=\"500\" r=\"350\" fill=\"url(#icon-glow)\" opacity=\"0.26\"/>\
         </g>",
        rays(CENTRE, 500.0, 760.0, 28, 0.09)
    ));

    // Gold rim just inside the edge, with a thin dark line to seat it.
    doc.push(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"#3a0208\" stroke-width=\"26\" opacity=\"0.55\"/>",
        squircle(CENTRE, CENTRE, rim_half)
    ));
    doc.push(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"url(#icon-rim)\" stroke-width=\"16\"/>",
        squircle(CENTRE, CENTRE, rim_half)
    ));

    // Gold star behind, red seven in front.
    let star = symbols::build("star")?;
    let seven = symbols::build("red_seven")?;
    doc.defs(&star.defs);
    doc.defs(&seven.defs);
    doc.push("<g clip-path=\"url(#icon-clip)\">");
    doc.place(&star, &symbol_at(CENTRE, 508.0, 1.47, -10.0));
    doc.place(&seven, &symbol_at(CENTRE + 8.0, 522.0, 0.98, 0.0));
    doc.push("</g>");

    // Sparkles.
    for (x, y, size, opacity) in [
        (232.0, 262.0, 40.0, 0.95),
        (806.0, 232.0, 28.0, 0.9),
        (822.0, 770.0, 34.0, 0.9),
        (214.0, 790.0, 22.0, 0.8),
    ] {
        doc.push(&format!(
            "<path d=\"{}\" fill=\"#fffbe6\" opacity=\"{opacity}\"/>",
            svg::sparkle_path(x, y, size)
        ));
    }

    // Glass highlight across the top half.
    doc.push(&format!(
        "<g clip-path=\"url(#icon-clip)\">\
         <ellipse cx=\"{CENTRE}\" cy=\"130\" rx=\"520\" ry=\"300\" fill=\"url(#icon-gloss)\" \
         opacity=\"0.13\"/></g>"
    ));

    Ok(doc.finish(if ios {
        "FRUIT STAR 3D iPhone application icon - original procedural artwork generated by \
             fruit-star-assetgen (icon.rs). No third-party material."
    } else {
        "FRUIT STAR 3D application icon - original procedural artwork generated by \
             fruit-star-assetgen (icon.rs). No third-party material."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster;

    #[test]
    fn icon_renders_with_transparent_corners_and_an_opaque_centre() -> Result<()> {
        let image = raster::render_svg(&app_icon_svg()?, 256, 256)?;
        assert_eq!(image.pixel(0, 0)[3], 0, "corner must be transparent");
        assert_eq!(image.pixel(255, 0)[3], 0);
        assert_eq!(image.pixel(128, 128)[3], 255, "centre must be opaque");
        // The tile's edge sits inside the canvas, leaving room for the shadow.
        assert_eq!(image.pixel(128, 6)[3], 0);
        assert_eq!(image.pixel(128, 40)[3], 255);
        Ok(())
    }

    #[test]
    fn ios_icon_is_fully_opaque() -> Result<()> {
        let image = raster::render_svg(&app_icon_ios_svg()?, 128, 128)?;
        for (x, y) in [(0, 0), (127, 0), (0, 127), (127, 127), (64, 64), (2, 64)] {
            assert_eq!(image.pixel(x, y)[3], 255, "({x}, {y}) must be opaque");
        }
        Ok(())
    }

    #[test]
    fn iconset_lists_every_size_macos_expects() {
        let sizes: Vec<u32> = ICONSET.iter().map(|(_, size)| *size).collect();
        assert_eq!(sizes, [16, 32, 32, 64, 128, 256, 256, 512, 512, 1024]);
        for (name, _) in ICONSET {
            assert!(name.starts_with("icon_") && name.ends_with(".png"));
        }
    }

    #[test]
    fn squircle_is_wider_at_the_corners_than_a_circle() {
        let d = squircle(0.0, 0.0, 100.0);
        let far = d
            .split_whitespace()
            .filter_map(|point| {
                let (x, y) = point.trim_start_matches(['M', 'L']).split_once(',')?;
                Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?))
            })
            .map(|(x, y)| (x * x + y * y).sqrt())
            .fold(0.0f32, f32::max);
        assert!(far > 110.0 && far < 141.5, "{far}");
    }
}
