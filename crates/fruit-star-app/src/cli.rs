//! 명령줄 옵션.

use std::path::PathBuf;

use fruit_star_game::PlayMode;
use fruit_star_machine::ViewMode;

/// 실행 옵션. 대부분 자동화된 실행 확인(스모크 테스트)을 위한 것이다.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Options {
    /// 메뉴를 건너뛰고 이 모드로 바로 시작한다.
    pub mode: Option<PlayMode>,
    pub view: Option<ViewMode>,
    /// 난수 시드 (재현 가능한 실행).
    pub seed: Option<u64>,
    /// 데이터베이스 경로. `:memory:`면 저장하지 않는다.
    pub database: Option<String>,
    /// 자동으로 베팅·레버·정지를 눌러 가며 플레이한다.
    pub auto_play: bool,
    /// (시각(초), 저장 경로) 스크린샷 예약.
    pub screenshots: Vec<(f32, PathBuf)>,
    /// 이 시간(초)이 지나면 종료한다.
    pub exit_after: Option<f32>,
    /// 지정한 초에 카메라를 다음 프리셋으로 넘긴다.
    pub camera_at: Vec<f32>,
    /// 창 대신 화면 밖 이미지(1920×1080)에 그린다. 스크린샷도 그 이미지에서 저장한다.
    pub offscreen: bool,
    /// 지정한 초에 일시정지를 전환한다.
    pub pause_at: Vec<f32>,
    pub help: bool,
}

pub const USAGE: &str = "\
FRUIT STAR 3D

사용법: fruit-star-app [옵션]

  --mode <classic|free_play|practice>   메뉴를 건너뛰고 바로 시작
  --view <play|mechanical|gallery>      보기 방식 (--mode와 함께)
  --seed <숫자>                          난수 시드 고정
  --db <경로|:memory:>                   데이터베이스 위치
  --auto-play                           자동 플레이 (실행 확인용)
  --screenshot <초> <파일.png>           지정 시각에 화면 저장 (여러 번 지정 가능)
  --camera-at <초>                       지정 시각에 카메라 전환 (여러 번 지정 가능)
  --offscreen                           창 대신 화면 밖 이미지에 렌더링 (창이 가려져 있어도 스크린샷 가능)
  --pause-at <초>                        지정 시각에 일시정지 전환 (여러 번 지정 가능)
  --exit-after <초>                      지정 시간 뒤 종료
  --help                                이 도움말
";

fn value<'a>(args: &mut impl Iterator<Item = &'a str>, option: &str) -> Result<&'a str, String> {
    args.next()
        .ok_or_else(|| format!("{option} 옵션에 값이 필요합니다"))
}

fn number<T: std::str::FromStr>(text: &str, option: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{option} 옵션의 값 '{text}'이(가) 올바른 숫자가 아닙니다"))
}

/// 명령줄 인자를 해석한다 (프로그램 이름 제외).
pub fn parse<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg {
            "--mode" => {
                let text = value(&mut args, arg)?;
                options.mode = Some(
                    PlayMode::from_key(text).ok_or_else(|| format!("알 수 없는 모드 '{text}'"))?,
                );
            }
            "--view" => {
                let text = value(&mut args, arg)?;
                options.view = Some(match text {
                    "play" => ViewMode::Play,
                    "mechanical" => ViewMode::Mechanical,
                    "gallery" => ViewMode::Gallery,
                    _ => return Err(format!("알 수 없는 보기 방식 '{text}'")),
                });
            }
            "--seed" => options.seed = Some(number(value(&mut args, arg)?, arg)?),
            "--db" => options.database = Some(value(&mut args, arg)?.to_owned()),
            "--auto-play" => options.auto_play = true,
            "--offscreen" => options.offscreen = true,
            "--pause-at" => options.pause_at.push(number(value(&mut args, arg)?, arg)?),
            "--screenshot" => {
                let at = number(value(&mut args, arg)?, arg)?;
                let path = PathBuf::from(value(&mut args, arg)?);
                options.screenshots.push((at, path));
            }
            "--camera-at" => options.camera_at.push(number(value(&mut args, arg)?, arg)?),
            "--exit-after" => options.exit_after = Some(number(value(&mut args, arg)?, arg)?),
            "--help" | "-h" => options.help = true,
            other => return Err(format!("알 수 없는 옵션 '{other}'")),
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_options() {
        let options = parse([
            "--mode",
            "practice",
            "--view",
            "mechanical",
            "--seed",
            "42",
            "--db",
            ":memory:",
            "--auto-play",
            "--screenshot",
            "1.5",
            "a.png",
            "--screenshot",
            "3",
            "b.png",
            "--camera-at",
            "2",
            "--exit-after",
            "9",
        ])
        .unwrap_or_default();
        assert_eq!(options.mode, Some(PlayMode::Practice));
        assert_eq!(options.view, Some(ViewMode::Mechanical));
        assert_eq!(options.seed, Some(42));
        assert_eq!(options.database.as_deref(), Some(":memory:"));
        assert!(options.auto_play);
        assert_eq!(options.screenshots.len(), 2);
        assert_eq!(options.screenshots[1], (3.0, PathBuf::from("b.png")));
        assert_eq!(options.camera_at, vec![2.0]);
        assert_eq!(options.exit_after, Some(9.0));
    }

    #[test]
    fn no_arguments_means_defaults() {
        assert_eq!(parse([]), Ok(Options::default()));
    }

    #[test]
    fn bad_input_is_reported() {
        assert!(parse(["--mode"]).is_err());
        assert!(parse(["--mode", "turbo"]).is_err());
        assert!(parse(["--seed", "abc"]).is_err());
        assert!(parse(["--screenshot", "1.0"]).is_err());
        assert!(parse(["--frobnicate"]).is_err());
    }
}
