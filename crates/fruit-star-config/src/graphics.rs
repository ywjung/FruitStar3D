//! `graphics.toml`: 렌더링 품질과 연출.

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, check_range};

const FILE: &str = "graphics.toml";

/// 품질 프리셋.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
}

impl Quality {
    pub const ALL: [Quality; 4] = [Quality::Low, Quality::Medium, Quality::High, Quality::Ultra];

    pub const fn key(self) -> &'static str {
        match self {
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
            Quality::Ultra => "ultra",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|q| q.key() == key)
    }

    pub fn next(self) -> Self {
        match self {
            Quality::Low => Quality::Medium,
            Quality::Medium => Quality::High,
            Quality::High => Quality::Ultra,
            Quality::Ultra => Quality::Low,
        }
    }

    /// 릴 심볼 한 칸의 텍스처 너비 (픽셀). 릴 띠 전체는 1K~4K급이 된다.
    pub const fn symbol_texture_width(self) -> u32 {
        match self {
            Quality::Low => 256,
            Quality::Medium => 512,
            Quality::High => 512,
            Quality::Ultra => 1024,
        }
    }

    /// 그림자 맵 해상도.
    pub const fn shadow_map_size(self) -> usize {
        match self {
            Quality::Low => 1024,
            Quality::Medium => 2048,
            Quality::High => 2048,
            Quality::Ultra => 4096,
        }
    }

    /// 릴 원통의 둘레 분할 수.
    pub const fn reel_segments(self) -> u32 {
        match self {
            Quality::Low => 42,
            Quality::Medium => 84,
            Quality::High => 126,
            Quality::Ultra => 168,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderSection {
    /// 그래픽 백엔드. macOS에서는 `metal`만 지원한다.
    pub backend: String,
    pub quality: Quality,
    /// 목표 프레임률. 이 값을 넘지 않도록 제한한다.
    pub target_fps: u32,
    /// 기기 열 상태가 "높음" 이상일 때의 목표 프레임률. `target_fps`보다 크면 `target_fps`를 쓴다.
    /// 열 상태는 iOS에서만 읽으므로 다른 플랫폼에서는 쓰이지 않는다.
    pub hot_target_fps: u32,
    pub bloom: bool,
    pub shadows: bool,
    pub ambient_occlusion: bool,
    pub motion_blur: bool,
    /// 렌더 배율 (3D 화면 해상도 배율).
    pub render_scale: f32,
    /// 동시에 화면에 남는 메달 수 상한.
    pub max_medals: u32,
}

impl Default for RenderSection {
    fn default() -> Self {
        Self {
            backend: "metal".to_owned(),
            quality: Quality::High,
            target_fps: 120,
            hot_target_fps: 30,
            bloom: true,
            shadows: true,
            ambient_occlusion: true,
            motion_blur: true,
            render_scale: 1.0,
            max_medals: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LedSection {
    /// LED 밝기 배율 (0 ~ 1.5).
    pub brightness: f32,
    /// 점멸을 줄인다 (광과민 배려).
    pub reduce_flashing: bool,
}

impl Default for LedSection {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            reduce_flashing: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CameraSection {
    /// 카메라 전환 시간 (밀리초).
    pub transition_ms: u32,
    /// 세로 시야각 (도).
    pub fov_degrees: f32,
}

impl Default for CameraSection {
    fn default() -> Self {
        Self {
            transition_ms: 500,
            fov_degrees: 32.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GraphicsConfig {
    pub render: RenderSection,
    pub led: LedSection,
    pub camera: CameraSection,
}

impl GraphicsConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.render.backend != "metal" {
            return Err(ConfigError::invalid(
                FILE,
                "render.backend",
                format!("'{}' (macOS 빌드는 metal만 지원)", self.render.backend),
            ));
        }
        check_range(FILE, "render.target_fps", self.render.target_fps, 30, 240)?;
        check_range(
            FILE,
            "render.hot_target_fps",
            self.render.hot_target_fps,
            30,
            240,
        )?;
        check_range(
            FILE,
            "render.render_scale",
            self.render.render_scale,
            0.25,
            2.0,
        )?;
        check_range(FILE, "render.max_medals", self.render.max_medals, 0, 500)?;
        check_range(FILE, "led.brightness", self.led.brightness, 0.0, 1.5)?;
        check_range(
            FILE,
            "camera.transition_ms",
            self.camera.transition_ms,
            0,
            5_000,
        )?;
        check_range(
            FILE,
            "camera.fov_degrees",
            self.camera.fov_degrees,
            15.0,
            90.0,
        )?;
        Ok(())
    }
}
