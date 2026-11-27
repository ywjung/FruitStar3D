//! Minimal helpers for assembling SVG documents as strings.

use std::fmt::Write as _;

/// A reusable piece of artwork: gradient/clip definitions plus drawable body markup.
///
/// Ids inside `defs` are prefixed with the artwork name, so several pieces can
/// be embedded in one document without clashing.
#[derive(Debug, Clone, Default)]
pub struct Art {
    pub defs: String,
    pub body: String,
}

/// An SVG document under construction.
#[derive(Debug, Clone)]
pub struct Doc {
    width: u32,
    height: u32,
    defs: String,
    body: String,
}

impl Doc {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            defs: String::new(),
            body: String::new(),
        }
    }

    pub fn defs(&mut self, markup: &str) {
        self.defs.push_str(markup);
        self.defs.push('\n');
    }

    pub fn push(&mut self, markup: &str) {
        self.body.push_str(markup);
        self.body.push('\n');
    }

    /// Adds an artwork's definitions (once) and draws its body.
    pub fn add_art(&mut self, art: &Art) {
        self.defs(&art.defs);
        self.push(&art.body);
    }

    /// Draws an artwork inside a transformed group. Call [`Doc::defs`] with the
    /// artwork's definitions once beforehand.
    pub fn place(&mut self, art: &Art, transform: &str) {
        self.push(&format!(
            "<g transform=\"{transform}\">\n{}\n</g>",
            art.body
        ));
    }

    /// Serialises the document. `comment` documents the origin of the file.
    pub fn finish(&self, comment: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- {comment} -->\n\
             <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
             viewBox=\"0 0 {w} {h}\">\n<defs>\n{defs}</defs>\n{body}</svg>\n",
            w = self.width,
            h = self.height,
            defs = self.defs,
            body = self.body,
        )
    }
}

fn stops(list: &[(f32, &str)]) -> String {
    let mut out = String::new();
    for (offset, color) in list {
        let _ = write!(out, "<stop offset=\"{offset}\" stop-color=\"{color}\"/>");
    }
    out
}

/// Linear gradient in user-space coordinates (safe for strokes and thin shapes).
pub fn linear(id: &str, x1: f32, y1: f32, x2: f32, y2: f32, list: &[(f32, &str)]) -> String {
    format!(
        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{x1}\" y1=\"{y1}\" \
         x2=\"{x2}\" y2=\"{y2}\">{}</linearGradient>",
        stops(list)
    )
}

/// Radial gradient in object-bounding-box units with an offset focal point.
pub fn radial_bbox(id: &str, cx: f32, cy: f32, r: f32, list: &[(f32, &str)]) -> String {
    format!(
        "<radialGradient id=\"{id}\" cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fx=\"{cx}\" fy=\"{cy}\">{}\
         </radialGradient>",
        stops(list)
    )
}

/// Radial gradient in user-space coordinates.
pub fn radial(id: &str, cx: f32, cy: f32, r: f32, list: &[(f32, &str)]) -> String {
    format!(
        "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{cx}\" cy=\"{cy}\" \
         r=\"{r}\">{}</radialGradient>",
        stops(list)
    )
}

/// Vertices of a star with `points` tips, first tip pointing up (y grows downwards).
pub fn star_vertices(cx: f32, cy: f32, outer: f32, inner: f32, points: usize) -> Vec<(f32, f32)> {
    let n = points * 2;
    (0..n)
        .map(|i| {
            let radius = if i.is_multiple_of(2) { outer } else { inner };
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n as f32;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect()
}

/// Closed polygon path data from a vertex list.
pub fn polygon_path(vertices: &[(f32, f32)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in vertices.iter().enumerate() {
        let _ = write!(d, "{}{x:.2},{y:.2} ", if i == 0 { 'M' } else { 'L' });
    }
    d.push('Z');
    d
}

/// Path data of a five-point star.
pub fn star_path(cx: f32, cy: f32, outer: f32, inner: f32) -> String {
    polygon_path(&star_vertices(cx, cy, outer, inner, 5))
}

/// A four-point sparkle with concave sides.
pub fn sparkle_path(cx: f32, cy: f32, size: f32) -> String {
    let k = size * 0.16;
    format!(
        "M{x0:.1},{t:.1} Q{xk1:.1},{yk0:.1} {r:.1},{y0:.1} Q{xk1:.1},{yk1:.1} {x0:.1},{b:.1} \
         Q{xk0:.1},{yk1:.1} {l:.1},{y0:.1} Q{xk0:.1},{yk0:.1} {x0:.1},{t:.1} Z",
        x0 = cx,
        y0 = cy,
        t = cy - size,
        b = cy + size,
        l = cx - size,
        r = cx + size,
        xk0 = cx - k,
        xk1 = cx + k,
        yk0 = cy - k,
        yk1 = cy + k,
    )
}

/// Parses `#rrggbb` into components.
pub fn parse_hex(color: &str) -> [u8; 3] {
    let hex = color.trim_start_matches('#');
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    [channel(0), channel(2), channel(4)]
}

/// Linear interpolation between two `#rrggbb` colours.
pub fn mix_hex(a: &str, b: &str, t: f32) -> String {
    let (ca, cb) = (parse_hex(a), parse_hex(b));
    let t = t.clamp(0.0, 1.0);
    let mix =
        |i: usize| (f32::from(ca[i]) + (f32::from(cb[i]) - f32::from(ca[i])) * t).round() as u8;
    format!("#{:02x}{:02x}{:02x}", mix(0), mix(1), mix(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(parse_hex("#ff8000"), [255, 128, 0]);
        assert_eq!(mix_hex("#000000", "#ffffff", 0.5), "#808080");
        assert_eq!(mix_hex("#102030", "#405060", 0.0), "#102030");
    }

    #[test]
    fn star_has_ten_vertices_and_points_up() {
        let v = star_vertices(100.0, 100.0, 50.0, 25.0, 5);
        assert_eq!(v.len(), 10);
        assert!((v[0].0 - 100.0).abs() < 1e-3);
        assert!((v[0].1 - 50.0).abs() < 1e-3);
    }
}
