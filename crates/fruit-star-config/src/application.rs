//! `application.toml`: 창과 게임 기본값.

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, check_range};

const FILE: &str = "application.toml";

/// 표시 언어.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Ko,
    Ja,
    En,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::Ko, Language::Ja, Language::En];

    pub const fn key(self) -> &'static str {
        match self {
            Language::Ko => "ko",
            Language::Ja => "ja",
            Language::En => "en",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.key() == key)
    }

    /// 다음 언어 (순환).
    pub fn next(self) -> Self {
        match self {
            Language::Ko => Language::Ja,
            Language::Ja => Language::En,
            Language::En => Language::Ko,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowSection {
    pub title: String,
    /// 창 너비 (논리 픽셀).
    pub width: u32,
    /// 창 높이 (논리 픽셀).
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Default for WindowSection {
    fn default() -> Self {
        Self {
            title: "FRUIT STAR 3D".to_owned(),
            width: 1920,
            height: 1080,
            fullscreen: false,
            vsync: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameSection {
    /// 기종 ID. `config/machines/<id>.toml`을 읽는다.
    pub machine: String,
    pub language: Language,
    /// 시작 모드: `classic`, `free_play`, `practice`.
    pub mode: String,
    /// Classic 모드의 첫 실행 시 보유 가상 메달.
    pub initial_credit: u32,
    /// 메달 투입으로 도달할 수 있는 최대 크레딧.
    pub credit_limit: u32,
    /// 메달 추가 입력 한 번에 투입되는 가상 메달 수.
    pub medals_per_insert: u32,
}

impl Default for GameSection {
    fn default() -> Self {
        Self {
            machine: "fruit_classic".to_owned(),
            language: Language::Ko,
            mode: "classic".to_owned(),
            initial_credit: 50,
            credit_limit: 9_999,
            medals_per_insert: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApplicationConfig {
    pub window: WindowSection,
    pub game: GameSection,
}

impl ApplicationConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        check_range(FILE, "window.width", self.window.width, 640, 7_680)?;
        check_range(FILE, "window.height", self.window.height, 360, 4_320)?;
        if self.window.title.trim().is_empty() {
            return Err(ConfigError::invalid(FILE, "window.title", "비어 있습니다"));
        }
        let machine = &self.game.machine;
        if machine.is_empty()
            || !machine
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(ConfigError::invalid(
                FILE,
                "game.machine",
                format!("'{machine}' (영문 소문자·숫자·밑줄만 허용)"),
            ));
        }
        if !matches!(
            self.game.mode.as_str(),
            "classic" | "free_play" | "practice"
        ) {
            return Err(ConfigError::invalid(
                FILE,
                "game.mode",
                format!(
                    "'{}' (classic, free_play, practice 중 하나)",
                    self.game.mode
                ),
            ));
        }
        check_range(FILE, "game.credit_limit", self.game.credit_limit, 1, 9_999)?;
        check_range(
            FILE,
            "game.initial_credit",
            self.game.initial_credit,
            0,
            self.game.credit_limit,
        )?;
        check_range(
            FILE,
            "game.medals_per_insert",
            self.game.medals_per_insert,
            1,
            1_000,
        )?;
        Ok(())
    }
}
