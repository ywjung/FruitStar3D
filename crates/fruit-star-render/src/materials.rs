//! 재질 모음.
//!
//! 금속성·거칠기 값은 계획서의 재질 명세를 따른다.

use bevy::prelude::*;

use crate::assets::ArtAssets;

/// 공용 재질 핸들.
#[derive(Resource, Debug, Clone)]
pub struct MaterialLibrary {
    /// 크롬 (metallic 1.0, roughness 0.12).
    pub chrome: Handle<StandardMaterial>,
    /// 스테인리스 (metallic 1.0, roughness 0.30).
    pub stainless: Handle<StandardMaterial>,
    /// 빨간 광택 플라스틱 (metallic 0.0, roughness 0.22).
    pub red_plastic: Handle<StandardMaterial>,
    /// 검은 광택 플라스틱 (패널).
    pub black_gloss: Handle<StandardMaterial>,
    /// 검은 무광 (내부, 그릴).
    pub black_matte: Handle<StandardMaterial>,
    /// 어두운 회색 (받침대).
    pub dark_grey: Handle<StandardMaterial>,
    /// 유리 (metallic 0.0, roughness 0.05).
    pub glass: Handle<StandardMaterial>,
    /// 메달 (metallic 1.0, roughness 0.25).
    pub medal: Handle<StandardMaterial>,
    /// 7세그먼트 점등 / 소등.
    pub segment_on: Handle<StandardMaterial>,
    pub segment_off: Handle<StandardMaterial>,
    /// 내부 부품용 재질.
    pub pcb_green: Handle<StandardMaterial>,
    pub motor_black: Handle<StandardMaterial>,
    pub copper: Handle<StandardMaterial>,
    /// 바닥과 벽.
    pub floor: Handle<StandardMaterial>,
    pub wall: Handle<StandardMaterial>,
}

impl MaterialLibrary {
    pub fn new(materials: &mut Assets<StandardMaterial>, art: &ArtAssets) -> Self {
        let chrome = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.97),
            metallic: 1.0,
            perceptual_roughness: 0.12,
            ..default()
        });
        let stainless = materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.79, 0.80),
            metallic: 1.0,
            perceptual_roughness: 0.30,
            ..default()
        });
        let red_plastic = materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.02, 0.05),
            metallic: 0.0,
            perceptual_roughness: 0.22,
            clearcoat: 1.0,
            clearcoat_perceptual_roughness: 0.08,
            ..default()
        });
        let black_gloss = materials.add(StandardMaterial {
            base_color: Color::srgb(0.012, 0.012, 0.016),
            metallic: 0.0,
            perceptual_roughness: 0.16,
            ..default()
        });
        let black_matte = materials.add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.02, 0.022),
            metallic: 0.0,
            perceptual_roughness: 0.85,
            ..default()
        });
        let dark_grey = materials.add(StandardMaterial {
            base_color: Color::srgb(0.07, 0.07, 0.08),
            metallic: 0.0,
            perceptual_roughness: 0.55,
            ..default()
        });
        // 유리는 확산광을 거의 내지 않고 반사만 얹는다. 사전곱 알파라 반사 하이라이트는
        // 투명도와 무관하게 남고, 뒤의 심볼은 흐려지지 않는다.
        let glass = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.02),
            metallic: 0.0,
            perceptual_roughness: 0.05,
            reflectance: 0.35,
            alpha_mode: AlphaMode::Premultiplied,
            ..default()
        });
        let medal = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.82, 0.38),
            base_color_texture: Some(art.medal_face.clone()),
            metallic: 1.0,
            perceptual_roughness: 0.25,
            ..default()
        });
        let segment_on = materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.08, 0.05),
            emissive: LinearRgba::rgb(9.0, 0.7, 0.35),
            perceptual_roughness: 0.4,
            ..default()
        });
        let segment_off = materials.add(StandardMaterial {
            base_color: Color::srgb(0.07, 0.008, 0.008),
            perceptual_roughness: 0.4,
            ..default()
        });
        let pcb_green = materials.add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.22, 0.09),
            perceptual_roughness: 0.5,
            ..default()
        });
        let motor_black = materials.add(StandardMaterial {
            base_color: Color::srgb(0.03, 0.03, 0.035),
            metallic: 0.6,
            perceptual_roughness: 0.45,
            ..default()
        });
        let copper = materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.45, 0.25),
            metallic: 1.0,
            perceptual_roughness: 0.35,
            ..default()
        });
        let floor = materials.add(StandardMaterial {
            base_color: Color::srgb(0.035, 0.03, 0.045),
            perceptual_roughness: 0.55,
            ..default()
        });
        let wall = materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.04, 0.06),
            perceptual_roughness: 0.9,
            ..default()
        });
        Self {
            chrome,
            stainless,
            red_plastic,
            black_gloss,
            black_matte,
            dark_grey,
            glass,
            medal,
            segment_on,
            segment_off,
            pcb_green,
            motor_black,
            copper,
            floor,
            wall,
        }
    }
}

/// 버튼 재질 (metallic 0.0, roughness 0.18). 램프로 쓰이므로 개별 인스턴스를 만든다.
///
/// 투명 코팅(clearcoat)은 쓰지 않는다. Bevy는 코팅 층이 발광을 크게 깎기 때문에,
/// 안에서 빛나야 하는 버튼과 LED에 코팅을 얹으면 불이 꺼진 것처럼 보인다.
pub fn button_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        metallic: 0.0,
        perceptual_roughness: 0.18,
        reflectance: 0.6,
        ..default()
    }
}

/// 빛나지 않는 고광택 플라스틱 (레버 손잡이). 투명 코팅으로 깊은 광택을 낸다.
pub fn gloss_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        metallic: 0.0,
        perceptual_roughness: 0.2,
        clearcoat: 1.0,
        clearcoat_perceptual_roughness: 0.05,
        ..default()
    }
}

/// 부품 표면에 인쇄된 글자 재질. 라벨 텍스처(흰 글자 + 알파)를 `ink` 색으로 찍는다.
pub fn printed_material(texture: Handle<Image>, ink: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: ink,
        base_color_texture: Some(texture),
        perceptual_roughness: 0.5,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }
}

/// 드럼 바깥쪽에 감긴 인쇄 필름. 아래의 불투명 슬리브가 안쪽 면을 이룬다.
pub fn reel_print_material(strip: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.88, 0.86, 0.81),
        base_color_texture: Some(strip.clone()),
        emissive: LinearRgba::BLACK,
        emissive_texture: Some(strip),
        metallic: 0.0,
        perceptual_roughness: 0.36,
        ..default()
    }
}

/// 텍스처가 스스로 빛나는 패널 재질 (백라이트 패널, 라벨).
pub fn backlit_material(texture: Handle<Image>, alpha: bool) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture.clone()),
        emissive: LinearRgba::BLACK,
        emissive_texture: Some(texture),
        perceptual_roughness: 0.3,
        alpha_mode: if alpha {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..default()
    }
}
