//! 릴 원통 메시.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec2;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// 릴 띠(원통 옆면) 메시를 만든다.
///
/// * 축은 X축이고 원통 중심이 원점이다.
/// * 메시 각도 `θ`는 +Z(정면)에서 +Y(위)로 잰다. 심볼 `k`의 중심은 `θ = k × 한 칸 각도`에
///   놓이므로, 회전하지 않은 상태에서 심볼 0이 정면 가운데에 온다.
/// * 릴 위치가 `p`(스텝)일 때는 X축을 기준으로 `+p × 한 칸 각도`만큼 회전시키면
///   심볼 `p`가 정면에 오고, `p`가 커질수록 표면이 아래로 흐른다.
/// * UV: `u`는 왼쪽(−X)에서 0, 오른쪽에서 1. `v`는 띠 텍스처가 위에서부터
///   마지막 인덱스 → 0번 순으로 쌓여 있다고 보고 `1 − (θ/한 칸 + 0.5)/심볼 수`다.
pub fn reel_band_mesh(radius: f32, width: f32, symbol_count: usize, segments: u32) -> Mesh {
    let segments = segments.max(symbol_count as u32);
    let step = TAU / symbol_count as f32;
    let half = width / 2.0;
    let rows = segments + 1;

    // 인쇄 필름이 양쪽 가장자리에서 드럼의 둥근 어깨를 감싼다.
    let shoulder = (width * 0.035).min(radius * 0.03);
    let profile = [
        (-half, radius - shoulder * 0.65, -0.83_f32),
        (-half + shoulder * 0.35, radius - shoulder * 0.18, -0.48),
        (-half + shoulder, radius, 0.0),
        (half - shoulder, radius, 0.0),
        (half - shoulder * 0.35, radius - shoulder * 0.18, 0.48),
        (half, radius - shoulder * 0.65, 0.83),
    ];
    let stride = profile.len() as u32;
    let vertices = (rows * stride) as usize;
    let mut positions = Vec::with_capacity(vertices);
    let mut normals = Vec::with_capacity(vertices);
    let mut uvs = Vec::with_capacity(vertices);
    for i in 0..rows {
        // 텍스처 이음매가 심볼 사이 경계(θ = −반 칸)에 오도록 시작 각도를 잡는다.
        let fraction = i as f32 / segments as f32;
        let theta = -step / 2.0 + fraction * TAU;
        let (sin, cos) = theta.sin_cos();
        let v = 1.0 - fraction;
        for (x, r, nx) in profile {
            let nr = (1.0 - nx * nx).sqrt();
            positions.push([x, r * sin, r * cos]);
            normals.push([nx, nr * sin, nr * cos]);
            uvs.push([(x + half) / width, v]);
        }
    }

    let mut indices = Vec::with_capacity((segments * (stride - 1) * 6) as usize);
    for i in 0..segments {
        for j in 0..stride - 1 {
            let a = i * stride + j;
            indices.extend_from_slice(&[a, a + 1, a + stride + 1, a, a + stride + 1, a + stride]);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// 양쪽 끝판에 쓰는 금속 프레스 부품. 여섯 개의 원형 타공과 축 구멍은 관통한다.
pub(crate) fn reel_flange_mesh(radius: f32, bore: f32, thickness: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut quad = |vertices: [[f32; 3]; 4], face_normals: [[f32; 3]; 4]| {
        let a = positions.len() as u32;
        positions.extend(vertices);
        normals.extend(face_normals);
        indices.extend([a, a + 1, a + 2, a, a + 2, a + 3]);
    };
    let at = |side: f32, p: Vec2| [side * thickness * 0.5, p.x, p.y];
    let normal = |p: Vec2| [0.0, p.x, p.y];
    const ARC: usize = 12;
    const RADIAL: usize = 8;
    for sector in 0..6 {
        let angle = sector as f32 * TAU / 6.0;
        let start = angle - TAU / 12.0;
        let middle = Vec2::from_angle(angle) * (radius * 0.63);
        // 한 타공을 둘러싼 부채꼴 둘레를 돈다. 이웃 부채꼴과 맞닿는 직선에는 벽이 없다.
        let direction = |t: f32| Vec2::from_angle(start + t * TAU / 6.0);
        let arc = (0..ARC).map(|i| i as f32 / ARC as f32);
        let radial = (0..RADIAL).map(|i| i as f32 / RADIAL as f32);
        let mut contour = Vec::with_capacity((ARC + RADIAL) * 2);
        contour.extend(arc.clone().map(|t| direction(t) * radius));
        contour.extend(
            radial
                .clone()
                .map(|t| direction(1.) * (radius + (bore - radius) * t)),
        );
        contour.extend(arc.map(|t| direction(1. - t) * bore));
        contour.extend(radial.map(|t| direction(0.) * (bore + (radius - bore) * t)));
        for i in 0..contour.len() {
            let a = contour[i];
            let b = contour[(i + 1) % contour.len()];
            let na = (a - middle).normalize();
            let nb = (b - middle).normalize();
            let ha = middle + na * (radius * 0.242);
            let hb = middle + nb * (radius * 0.242);
            quad(
                [at(1., a), at(1., b), at(1., hb), at(1., ha)],
                [[1., 0., 0.]; 4],
            );
            quad(
                [at(-1., ha), at(-1., hb), at(-1., b), at(-1., a)],
                [[-1., 0., 0.]; 4],
            );
            quad(
                [at(1., ha), at(1., hb), at(-1., hb), at(-1., ha)],
                [normal(-na), normal(-nb), normal(-nb), normal(-na)],
            );
            if i < ARC || (ARC + RADIAL..2 * ARC + RADIAL).contains(&i) {
                let sign = if i < ARC { 1.0 } else { -1.0 };
                let na = normal(a.normalize() * sign);
                let nb = normal(b.normalize() * sign);
                quad(
                    [at(-1., a), at(-1., b), at(1., b), at(1., a)],
                    [na, nb, nb, na],
                );
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// 표시창 턱의 차광을 원통의 앞쪽 호에 맞춘다. 릴이 돌아도 명암은 창에 고정된다.
pub(crate) fn window_shade_mesh(radius: f32, width: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for i in 0..=48 {
        let theta = (i as f32 / 48.0 - 0.5) * 1.4;
        let (sin, cos) = theta.sin_cos();
        let distance = ((theta - 0.065).abs() / 0.50).min(1.0);
        let shade = distance * distance * (3.0 - 2.0 * distance);
        for x in [-width / 2.0, width / 2.0] {
            positions.push([x, radius * sin, radius * cos]);
            normals.push([0.0, sin, cos]);
            colors.push([0.015_f32, 0.012, 0.017, shade * 0.86]);
        }
        if i < 48 {
            let a = i * 2;
            indices.extend([a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// 릴 위치(스텝)를 원통의 X축 회전 각도로 바꾼다.
pub fn rotation_for_position(position: f64, symbol_count: usize) -> f32 {
    (position * std::f64::consts::TAU / symbol_count as f64) as f32
}

/// 띠 텍스처 한 칸의 가로세로 비 (너비 ÷ 호의 길이).
pub fn cell_aspect(radius: f32, width: f32, symbol_count: usize) -> f32 {
    width / (TAU * radius / symbol_count as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn flange_triangles_face_their_normals_on_both_faces_and_inside_the_holes() {
        let mesh = reel_flange_mesh(0.13, 0.010, 0.004);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals")
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("indices")
        };
        for triangle in indices.as_chunks::<3>().0 {
            let [a, b, c] = triangle.map(|i| Vec3::from(positions[i as usize]));
            let normal = Vec3::from(normals[triangle[0] as usize]);
            assert!((b - a).cross(c - a).dot(normal) > 0.0, "{a} {b} {c}");
        }
    }

    #[test]
    fn symbol_centres_map_to_their_texture_cells() {
        let count = 21;
        let mesh = reel_band_mesh(0.1, 0.08, count, 126);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs");
        };
        let step = TAU / count as f32;
        for (position, uv) in positions.iter().zip(uvs) {
            let theta = position[1].atan2(position[2]).rem_euclid(TAU);
            // θ에 있는 표면이 속한 심볼 인덱스
            let symbol = ((theta / step + 0.5).floor() as usize) % count;
            // 그 정점의 v가 가리키는 텍스처 칸 (위에서부터 count-1 … 0)
            let row = (uv[1] * count as f32).min(count as f32 - 1e-3).floor() as usize;
            let cell_symbol = count - 1 - row;
            let on_boundary =
                ((theta / step + 0.5).fract()).min(1.0 - (theta / step + 0.5).fract()) < 1e-3;
            if !on_boundary {
                assert_eq!(symbol, cell_symbol, "θ={theta}, v={}", uv[1]);
            }
        }
    }

    #[test]
    fn rotation_brings_the_requested_symbol_to_the_front() {
        let count = 21;
        let step = TAU / count as f32;
        for index in 0..count {
            let angle = rotation_for_position(index as f64, count);
            // 심볼 index의 중심(θ = index × step)을 X축으로 +angle 회전하면 θ' = θ − angle = 0 (정면)
            let theta = index as f32 * step - angle;
            assert!(theta.abs() < 1e-5);
        }
        // 위치가 커지면 표면은 아래로 흐른다: 정면(θ=0)에 있던 점의 θ가 음수가 된다.
        assert!(0.0 - rotation_for_position(0.25, count) < 0.0);
    }

    #[test]
    fn triangles_face_outward() {
        let mesh = reel_band_mesh(0.1, 0.08, 21, 42);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("indices");
        };
        for tri in indices.chunks(3) {
            let [a, b, c] = [
                positions[tri[0] as usize],
                positions[tri[1] as usize],
                positions[tri[2] as usize],
            ];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            // 면 법선이 축에서 바깥쪽을 향해야 한다
            let outward = normal[1] * a[1] + normal[2] * a[2];
            assert!(outward > 0.0);
        }
    }

    #[test]
    fn cell_aspect_matches_geometry() {
        let aspect = cell_aspect(0.108, 0.080, 21);
        assert!((aspect - 2.476).abs() < 0.01, "{aspect}");
    }
}
