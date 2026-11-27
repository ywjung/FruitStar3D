//! 설정 오류.

use std::path::PathBuf;

use thiserror::Error;

/// 설정 로딩·검증 오류. 메시지는 사용자에게 그대로 보여 줄 수 있도록 작성한다.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("설정 파일 {path}을(를) 읽을 수 없습니다: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("설정 파일 {path}의 형식이 올바르지 않습니다: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("설정 {file}의 {field} 값이 올바르지 않습니다: {message}")]
    Invalid {
        file: &'static str,
        field: String,
        message: String,
    },
    #[error("설정을 TOML로 변환할 수 없습니다: {0}")]
    Serialize(String),
}

impl ConfigError {
    pub(crate) fn invalid(
        file: &'static str,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ConfigError::Invalid {
            file,
            field: field.into(),
            message: message.into(),
        }
    }
}

/// `value`가 `[min, max]` 범위에 있는지 검사한다.
pub(crate) fn check_range<T: PartialOrd + std::fmt::Display>(
    file: &'static str,
    field: &str,
    value: T,
    min: T,
    max: T,
) -> Result<(), ConfigError> {
    // NaN은 어떤 비교도 참이 아니므로 범위 밖으로 처리된다.
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(ConfigError::invalid(
            file,
            field,
            format!("{value} (허용 범위 {min} ~ {max})"),
        ))
    }
}
