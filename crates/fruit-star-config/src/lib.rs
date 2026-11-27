//! FRUIT STAR 3D 설정.
//!
//! `config/` 디렉터리의 TOML 파일을 Serde로 읽고 값의 범위를 검증한다.
//! 여기 정의된 키는 이 애플리케이션 자체의 스키마이며 Bevy API가 아니다.

pub mod application;
pub mod audio;
pub mod controls;
pub mod error;
pub mod graphics;
pub mod loader;
pub mod machine;

pub use application::{ApplicationConfig, GameSection, Language, WindowSection};
pub use audio::AudioConfig;
pub use controls::{ControlsConfig, KNOWN_KEYS, KeyBindings, MouseSection};
pub use error::ConfigError;
pub use graphics::{CameraSection, GraphicsConfig, LedSection, Quality, RenderSection};
pub use loader::{ConfigSet, LoadReport, Recovery, missing_assets, required_assets};
pub use machine::{AnimationSection, MachineConfig};
