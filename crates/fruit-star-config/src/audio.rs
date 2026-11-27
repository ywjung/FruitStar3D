//! `audio.toml`: 음량.

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, check_range};

const FILE: &str = "audio.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioConfig {
    /// 전체 음량 (0 ~ 1).
    pub master: f32,
    /// 효과음 음량 (0 ~ 1).
    pub effects: f32,
    /// 배경음(환경음) 음량 (0 ~ 1).
    pub ambient: f32,
    pub muted: bool,
    /// 릴·버튼 위치 기반 공간 음향 사용 여부.
    pub spatial: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master: 0.8,
            effects: 0.9,
            ambient: 0.35,
            muted: false,
            spatial: true,
        }
    }
}

impl AudioConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        check_range(FILE, "master", self.master, 0.0, 1.0)?;
        check_range(FILE, "effects", self.effects, 0.0, 1.0)?;
        check_range(FILE, "ambient", self.ambient, 0.0, 1.0)?;
        Ok(())
    }

    /// 음소거를 반영한 효과음 실효 음량.
    pub fn effective_effects(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.master * self.effects
        }
    }

    /// 음소거를 반영한 배경음 실효 음량.
    pub fn effective_ambient(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.master * self.ambient
        }
    }
}
