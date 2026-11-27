//! CPU 이미지 처리: 리샘플링, 밉맵 생성, 릴 띠 합성, 환경 큐브맵.
//!
//! 모두 순수 함수이며 RGBA8(sRGB, 비사전곱 알파) 픽셀 버퍼를 다룬다.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// RGBA8 픽셀 버퍼.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Pixels {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    /// Bevy 이미지에서 RGBA8 픽셀을 꺼낸다. 형식이 다르면 변환을 시도한다.
    pub fn from_image(image: &Image) -> Option<Self> {
        let converted;
        let source = match image.texture_descriptor.format {
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => image,
            _ => {
                converted = image.convert(TextureFormat::Rgba8UnormSrgb)?;
                &converted
            }
        };
        let data = source.data.clone()?;
        let (width, height) = (source.width(), source.height());
        // 밉맵이 붙어 있으면 최상위 레벨만 쓴다.
        let top = (width * height * 4) as usize;
        (data.len() >= top).then(|| Self {
            width,
            height,
            data: data[..top].to_vec(),
        })
    }

    fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    fn put(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        let i = ((y * self.width + x) * 4) as usize;
        self.data[i..i + 4].copy_from_slice(&pixel);
    }

    /// 가장자리를 고정(clamp)한 쌍선형 표본. 사전곱 알파 RGBA(0~1)를 돌려준다.
    fn sample_premultiplied(&self, u: f32, v: f32) -> [f32; 4] {
        let x = (u * self.width as f32 - 0.5).clamp(0.0, self.width as f32 - 1.0);
        let y = (v * self.height as f32 - 0.5).clamp(0.0, self.height as f32 - 1.0);
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let mut out = [0.0f32; 4];
        for (px, py, weight) in [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ] {
            let p = self.get(px, py);
            let alpha = f32::from(p[3]) / 255.0;
            out[0] += f32::from(p[0]) / 255.0 * alpha * weight;
            out[1] += f32::from(p[1]) / 255.0 * alpha * weight;
            out[2] += f32::from(p[2]) / 255.0 * alpha * weight;
            out[3] += alpha * weight;
        }
        out
    }

    /// 크기를 바꾼다. 축소할 때는 여러 점을 평균해 계단 현상을 줄인다.
    pub fn resized(&self, width: u32, height: u32) -> Pixels {
        let mut out = Pixels::new(width, height);
        let taps_x = (self.width as f32 / width as f32).ceil().clamp(1.0, 6.0) as u32;
        let taps_y = (self.height as f32 / height as f32).ceil().clamp(1.0, 6.0) as u32;
        let norm = 1.0 / (taps_x * taps_y) as f32;
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f32; 4];
                for sy in 0..taps_y {
                    for sx in 0..taps_x {
                        let u = (x as f32 + (sx as f32 + 0.5) / taps_x as f32) / width as f32;
                        let v = (y as f32 + (sy as f32 + 0.5) / taps_y as f32) / height as f32;
                        let s = self.sample_premultiplied(u, v);
                        for c in 0..4 {
                            sum[c] += s[c] * norm;
                        }
                    }
                }
                out.put(x, y, unpremultiply(sum));
            }
        }
        out
    }

    /// `top`을 (`left`, `upper`) 위치에 알파 합성한다. 범위를 벗어난 부분은 잘린다.
    pub fn blend(&mut self, top: &Pixels, left: i64, upper: i64) {
        for y in 0..top.height {
            let dy = upper + i64::from(y);
            if dy < 0 || dy >= i64::from(self.height) {
                continue;
            }
            for x in 0..top.width {
                let dx = left + i64::from(x);
                if dx < 0 || dx >= i64::from(self.width) {
                    continue;
                }
                let src = top.get(x, y);
                if src[3] == 0 {
                    continue;
                }
                let dst = self.get(dx as u32, dy as u32);
                let sa = f32::from(src[3]) / 255.0;
                let da = f32::from(dst[3]) / 255.0;
                let out_a = sa + da * (1.0 - sa);
                let mut out = [0u8; 4];
                for c in 0..3 {
                    let value = (f32::from(src[c]) * sa + f32::from(dst[c]) * da * (1.0 - sa))
                        / out_a.max(1e-6);
                    out[c] = value.round().clamp(0.0, 255.0) as u8;
                }
                out[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
                self.put(dx as u32, dy as u32, out);
            }
        }
    }
}

fn unpremultiply(p: [f32; 4]) -> [u8; 4] {
    let alpha = p[3].clamp(0.0, 1.0);
    if alpha <= 1e-5 {
        return [0, 0, 0, 0];
    }
    let channel = |v: f32| ((v / alpha).clamp(0.0, 1.0) * 255.0).round() as u8;
    [
        channel(p[0]),
        channel(p[1]),
        channel(p[2]),
        (alpha * 255.0).round() as u8,
    ]
}

fn srgb_to_linear(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let c = value.clamp(0.0, 1.0);
    let encoded = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

/// 한 단계 축소한 밉 레벨을 만든다 (감마 보정, 알파 가중 평균).
fn downsample(src: &Pixels, lut: &[f32; 256]) -> Pixels {
    let width = (src.width / 2).max(1);
    let height = (src.height / 2).max(1);
    let mut out = Pixels::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut rgb = [0.0f32; 3];
            let mut alpha_sum = 0.0f32;
            let mut count = 0.0f32;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let sx = (x * 2 + ox).min(src.width - 1);
                let sy = (y * 2 + oy).min(src.height - 1);
                let p = src.get(sx, sy);
                let alpha = f32::from(p[3]) / 255.0;
                for c in 0..3 {
                    rgb[c] += lut[p[c] as usize] * alpha;
                }
                alpha_sum += alpha;
                count += 1.0;
            }
            let pixel = if alpha_sum <= 1e-5 {
                [0, 0, 0, 0]
            } else {
                [
                    linear_to_srgb(rgb[0] / alpha_sum),
                    linear_to_srgb(rgb[1] / alpha_sum),
                    linear_to_srgb(rgb[2] / alpha_sum),
                    (alpha_sum / count * 255.0).round() as u8,
                ]
            };
            out.put(x, y, pixel);
        }
    }
    out
}

/// 최상위 레벨부터 1×1까지의 밉 체인.
pub fn mip_chain(top: Pixels) -> Vec<Pixels> {
    let mut lut = [0.0f32; 256];
    for (value, slot) in lut.iter_mut().enumerate() {
        *slot = srgb_to_linear(value as u8);
    }
    let mut levels = vec![top];
    while let Some(last) = levels.last() {
        if last.width == 1 && last.height == 1 {
            break;
        }
        let next = downsample(last, &lut);
        levels.push(next);
    }
    levels
}

/// 텍스처 표본화 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    Clamp,
    /// 세로 방향만 반복 (릴 띠).
    RepeatV,
}

/// 밉맵을 포함한 sRGB 텍스처를 만든다.
pub fn textured_image(pixels: Pixels, wrap: Wrap) -> Image {
    let (width, height) = (pixels.width, pixels.height);
    let levels = mip_chain(pixels);
    let mip_level_count = levels.len() as u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize * 4 / 3 + 16);
    for level in &levels {
        data.extend_from_slice(&level.data);
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mip_level_count;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: match wrap {
            Wrap::Clamp => ImageAddressMode::ClampToEdge,
            Wrap::RepeatV => ImageAddressMode::Repeat,
        },
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..ImageSamplerDescriptor::default()
    });
    image
}

/// 릴 한 개의 띠 텍스처 픽셀을 합성한다.
///
/// 띠는 위에서 아래로 심볼 인덱스가 작아지도록 쌓는다 (맨 위 칸 = 마지막 인덱스).
/// 릴 메시의 UV가 이 배치를 전제로 한다.
///
/// * `cells`: 인덱스 순서의 심볼 그림 (모두 같은 크기일 필요는 없다).
/// * `band`: 세로로 반복되는 띠 배경.
/// * `cell_width`, `cell_height`: 한 칸의 픽셀 크기.
pub fn compose_strip(
    cells: &[&Pixels],
    band: &Pixels,
    cell_width: u32,
    cell_height: u32,
) -> Pixels {
    let count = cells.len() as u32;
    let mut strip = Pixels::new(cell_width, cell_height * count);

    // 배경: 띠 무늬를 너비에 맞춰 늘리고 세로로 반복한다.
    let tile_height = ((band.height as f32) * cell_width as f32 / band.width as f32)
        .round()
        .max(1.0) as u32;
    let tile = band.resized(cell_width, tile_height);
    let mut y = 0;
    while y < strip.height {
        strip.blend(&tile, 0, i64::from(y));
        y += tile_height;
    }

    // 심볼: 비율을 유지한 채 칸 높이의 94%에 맞추고 가운데 정렬한다.
    for (index, art) in cells.iter().enumerate() {
        let fit = (cell_height as f32 * 0.94 / art.height as f32)
            .min(cell_width as f32 * 0.96 / art.width as f32);
        let width = ((art.width as f32 * fit).round() as u32).max(1);
        let height = ((art.height as f32 * fit).round() as u32).max(1);
        let scaled = art.resized(width, height);
        let row = count - 1 - index as u32;
        let left = (i64::from(cell_width) - i64::from(width)) / 2;
        let upper = i64::from(row * cell_height) + (i64::from(cell_height) - i64::from(height)) / 2;
        strip.blend(&scaled, left, upper);
    }
    strip
}

/// f32를 IEEE 754 반정밀도(f16) 비트로 바꾼다. 음수와 NaN은 0으로 취급한다.
fn f16_bits(value: f32) -> u16 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x007f_ffff;
    if exponent >= 31 {
        return 0x7bff; // 표현 가능한 최댓값으로 고정
    }
    if exponent <= 0 {
        if exponent < -10 {
            return 0;
        }
        let m = (mantissa | 0x0080_0000) >> (1 - exponent);
        return (m >> 13) as u16;
    }
    ((exponent as u16) << 10) | (mantissa >> 13) as u16
}

/// 방향 벡터에 대한 환경 복사 휘도를 주는 함수로 HDR 큐브맵을 만든다.
pub fn environment_cubemap(size: u32, radiance: impl Fn([f32; 3]) -> [f32; 3]) -> Image {
    let mut data = Vec::with_capacity((6 * size * size * 8) as usize);
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let a = 2.0 * (x as f32 + 0.5) / size as f32 - 1.0;
                let b = 2.0 * (y as f32 + 0.5) / size as f32 - 1.0;
                // 큐브맵 면 순서: +X, -X, +Y, -Y, +Z, -Z
                let dir = match face {
                    0 => [1.0, -b, -a],
                    1 => [-1.0, -b, a],
                    2 => [a, 1.0, b],
                    3 => [a, -1.0, -b],
                    4 => [a, -b, 1.0],
                    _ => [-a, -b, -1.0],
                };
                let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
                let rgb = radiance([dir[0] / len, dir[1] / len, dir[2] / len]);
                for channel in [rgb[0], rgb[1], rgb[2], 1.0] {
                    data.extend_from_slice(&f16_bits(channel).to_le_bytes());
                }
            }
        }
    }
    Image {
        texture_view_descriptor: Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        }),
        ..Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba16Float,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::f16_bits;

    #[test]
    fn f16_encoding_matches_known_values() {
        assert_eq!(f16_bits(0.0), 0x0000);
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(2.0), 0x4000);
        assert_eq!(f16_bits(0.5), 0x3800);
        assert_eq!(f16_bits(-3.0), 0x0000);
        assert_eq!(f16_bits(f32::NAN), 0x0000);
        assert_eq!(f16_bits(1.0e9), 0x7bff);
    }
}
