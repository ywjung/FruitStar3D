//! SVG rasterisation (via resvg) and PNG output with straight (non-premultiplied) alpha.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use resvg::{tiny_skia, usvg};

/// An 8-bit RGBA image with straight alpha.
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// Row-major RGBA bytes.
    pub rgba: Vec<u8>,
}

/// Rasterises an SVG document at exactly `width` × `height` pixels.
pub fn render_svg(svg: &str, width: u32, height: u32) -> Result<Image> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).context("parsing generated SVG")?;
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| anyhow!("invalid raster size {width}x{height}"))?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Ok(Image {
        width,
        height,
        rgba,
    })
}

impl Image {
    /// A fully opaque image filled with one colour.
    pub fn solid(width: u32, height: u32, rgb: [u8; 3]) -> Self {
        let rgba = [rgb[0], rgb[1], rgb[2], 255].repeat(width as usize * height as usize);
        Self {
            width,
            height,
            rgba,
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = self.index(x, y);
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, px: [u8; 4]) {
        let i = self.index(x, y);
        self.rgba[i..i + 4].copy_from_slice(&px);
    }

    /// Replaces the colour of every pixel while keeping alpha (used for tintable masks,
    /// so that texture filtering never drags in a dark fringe).
    pub fn force_rgb(&mut self, rgb: [u8; 3]) {
        for px in self.rgba.as_chunks_mut::<4>().0 {
            px[..3].copy_from_slice(&rgb);
        }
    }

    /// Makes the image fully opaque by compositing it over `rgb`.
    pub fn flatten(&mut self, rgb: [u8; 3]) {
        for px in self.rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(px[3]);
            for (c, bg) in px[..3].iter_mut().zip(rgb) {
                *c = ((u32::from(*c) * a + u32::from(bg) * (255 - a) + 127) / 255) as u8;
            }
            px[3] = 255;
        }
    }

    /// Propagates edge colours into fully transparent pixels ("alpha bleeding").
    ///
    /// Alpha is untouched; only the hidden RGB of transparent pixels changes, so
    /// bilinear filtering and mip-mapping in the engine do not produce dark halos.
    pub fn bleed_edges(&mut self, passes: usize) {
        let (w, h) = (self.width as i32, self.height as i32);
        let mut known: Vec<bool> = self
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3] > 0)
            .collect();
        for _ in 0..passes {
            let mut updates: Vec<(usize, [u8; 3])> = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    if known[idx] {
                        continue;
                    }
                    let mut sum = [0u32; 3];
                    let mut count = 0u32;
                    for (dx, dy) in [
                        (-1, 0),
                        (1, 0),
                        (0, -1),
                        (0, 1),
                        (-1, -1),
                        (1, -1),
                        (-1, 1),
                        (1, 1),
                    ] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let n = (ny * w + nx) as usize;
                        if known[n] {
                            for (s, c) in sum.iter_mut().zip(&self.rgba[n * 4..n * 4 + 3]) {
                                *s += u32::from(*c);
                            }
                            count += 1;
                        }
                    }
                    if count > 0 {
                        updates.push((idx, sum.map(|s| ((s + count / 2) / count) as u8)));
                    }
                }
            }
            if updates.is_empty() {
                break;
            }
            for (idx, rgb) in updates {
                self.rgba[idx * 4..idx * 4 + 3].copy_from_slice(&rgb);
                known[idx] = true;
            }
        }
    }

    /// Bounding box `(x0, y0, x1, y1)` (inclusive) of pixels whose alpha exceeds `threshold`.
    pub fn alpha_bounds(&self, threshold: u8) -> Option<(u32, u32, u32, u32)> {
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.rgba[self.index(x, y) + 3] > threshold {
                    bounds = Some(match bounds {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        bounds
    }

    /// Alpha-composites `src` onto `self` with its top-left corner at (`ox`, `oy`).
    pub fn composite(&mut self, src: &Image, ox: u32, oy: u32) {
        for y in 0..src.height.min(self.height.saturating_sub(oy)) {
            for x in 0..src.width.min(self.width.saturating_sub(ox)) {
                let s = src.pixel(x, y);
                let d = self.pixel(x + ox, y + oy);
                let sa = u32::from(s[3]);
                let da = u32::from(d[3]);
                let out_a = sa + da * (255 - sa) / 255;
                let mut out = [0u8; 4];
                for i in 0..3 {
                    let num = u32::from(s[i]) * sa + u32::from(d[i]) * da * (255 - sa) / 255;
                    out[i] = num.checked_div(out_a).unwrap_or(0).min(255) as u8;
                }
                out[3] = out_a.min(255) as u8;
                self.set_pixel(x + ox, y + oy, out);
            }
        }
    }

    /// Box-filter downscale by an integer factor.
    pub fn downscale(&self, factor: u32) -> Image {
        let (w, h) = (self.width / factor, self.height / factor);
        let mut out = Image {
            width: w,
            height: h,
            rgba: vec![0; w as usize * h as usize * 4],
        };
        let n = factor * factor;
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 4];
                for sy in 0..factor {
                    for sx in 0..factor {
                        let p = self.pixel(x * factor + sx, y * factor + sy);
                        let a = u32::from(p[3]);
                        acc[0] += u32::from(p[0]) * a;
                        acc[1] += u32::from(p[1]) * a;
                        acc[2] += u32::from(p[2]) * a;
                        acc[3] += a;
                    }
                }
                let px = [
                    acc[0].checked_div(acc[3]).unwrap_or(0) as u8,
                    acc[1].checked_div(acc[3]).unwrap_or(0) as u8,
                    acc[2].checked_div(acc[3]).unwrap_or(0) as u8,
                    (acc[3] / n) as u8,
                ];
                out.set_pixel(x, y, px);
            }
        }
        out
    }

    /// Writes the image as an 8-bit RGBA PNG tagged as sRGB.
    pub fn save_png(&self, path: &Path) -> Result<()> {
        ensure!(
            self.rgba.len() == self.width as usize * self.height as usize * 4,
            "image buffer size mismatch"
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        encoder.set_compression(png::Compression::High);
        let mut writer = encoder
            .write_header()
            .with_context(|| format!("writing PNG header of {}", path.display()))?;
        writer
            .write_image_data(&self.rgba)
            .with_context(|| format!("writing PNG data of {}", path.display()))?;
        writer
            .finish()
            .with_context(|| format!("finishing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_SQUARE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\">\
        <rect x=\"2\" y=\"2\" width=\"4\" height=\"4\" fill=\"#ff0000\"/></svg>";

    #[test]
    fn renders_with_transparent_background() {
        let img = render_svg(RED_SQUARE, 8, 8).unwrap();
        assert_eq!(img.pixel(0, 0)[3], 0);
        assert_eq!(img.pixel(4, 4), [255, 0, 0, 255]);
        assert_eq!(img.alpha_bounds(0), Some((2, 2, 5, 5)));
    }

    #[test]
    fn bleeding_keeps_alpha_and_spreads_colour() {
        let mut img = render_svg(RED_SQUARE, 8, 8).unwrap();
        img.bleed_edges(8);
        assert_eq!(img.pixel(0, 0), [255, 0, 0, 0]);
        assert_eq!(img.pixel(4, 4), [255, 0, 0, 255]);
    }

    #[test]
    fn flatten_makes_opaque() {
        let mut img = render_svg(RED_SQUARE, 8, 8).unwrap();
        img.flatten([10, 20, 30]);
        assert_eq!(img.pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(img.pixel(3, 3), [255, 0, 0, 255]);
    }
}
