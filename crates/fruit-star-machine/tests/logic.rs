#![allow(clippy::unwrap_used, clippy::expect_used)]

use fruit_star_machine::button::press_curve;
use fruit_star_machine::display::segment_lit;
use fruit_star_machine::lamp::led_state;
use fruit_star_machine::layout::*;
use fruit_star_machine::{LampMode, LedStrip, PartKind};

/// 표시기에 켜진 획을 비트마스크로 모은다 (비트 0 = a).
fn mask(value: Option<u32>, digits: u8, digit: u8) -> u8 {
    (0..7).fold(0, |acc, segment| {
        acc | (u8::from(segment_lit(value, digits, digit, segment)) << segment)
    })
}

#[test]
fn seven_segment_digits_use_standard_patterns() {
    assert_eq!(mask(Some(8), 2, 0), 0b1111111);
    assert_eq!(mask(Some(1), 2, 0), 0b0000110);
    assert_eq!(mask(Some(7), 2, 0), 0b0000111);
    assert_eq!(mask(Some(0), 2, 0), 0b0111111);
    // 47 → 십의 자리 4, 일의 자리 7
    assert_eq!(mask(Some(47), 4, 1), 0b1100110);
    assert_eq!(mask(Some(47), 4, 0), 0b0000111);
}

#[test]
fn leading_zeros_are_blank_and_overflow_is_clamped() {
    assert_eq!(mask(Some(47), 4, 2), 0);
    assert_eq!(mask(Some(47), 4, 3), 0);
    // 일의 자리 0은 표시한다
    assert_eq!(mask(Some(0), 4, 0), 0b0111111);
    // 값이 없으면 전부 끈다
    assert_eq!(mask(None, 3, 0), 0);
    // 자릿수를 넘으면 99로 표시
    assert_eq!(mask(Some(250), 2, 0), 0b1101111);
    assert_eq!(mask(Some(250), 2, 1), 0b1101111);
    // 범위를 벗어난 자리·획은 항상 꺼진다
    assert!(!segment_lit(Some(8), 2, 2, 0));
    assert!(!segment_lit(Some(8), 2, 0, 7));
}

#[test]
fn press_curve_starts_and_ends_at_rest() {
    for kind in [PartKind::Lever, PartKind::Bet, PartKind::Stop(1)] {
        assert_eq!(press_curve(kind, 0.0), 0.0);
        assert!(press_curve(kind, 1.0).abs() < 1e-4, "{kind:?}");
        // 35% 지점에서 끝까지 눌린다
        assert!((press_curve(kind, 0.35) - 1.0).abs() < 1e-4);
        for step in 0..=100 {
            let amount = press_curve(kind, step as f32 / 100.0);
            assert!((-0.15..=1.0).contains(&amount), "{kind:?} {step} {amount}");
        }
    }
    // 버튼은 원위치를 지나치지 않는다
    for step in 0..=100 {
        assert!(press_curve(PartKind::Bet, step as f32 / 100.0) >= 0.0);
    }
}

#[test]
fn led_patterns_stay_in_range_in_every_mode() {
    for mode in [
        LampMode::Attract,
        LampMode::Spin,
        LampMode::Win,
        LampMode::Bonus,
    ] {
        for strip in [LedStrip::Left, LedStrip::Right, LedStrip::Top] {
            for reduce in [false, true] {
                for index in 0..16 {
                    for step in 0..200 {
                        let t = step as f32 * 0.013;
                        let (brightness, _) = led_state(mode, strip, index, 16, t, true, reduce);
                        assert!((0.0..=1.3).contains(&brightness), "{mode:?} {brightness}");
                    }
                }
            }
        }
    }
}

#[test]
fn reduce_flashing_removes_hard_blinking() {
    // 당첨 연출: 기본은 밝기가 크게 오르내리지만, 점멸 줄이기에서는 변화 폭이 작고 느리다.
    let sample = |reduce: bool| -> (f32, f32, u32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut jumps = 0;
        let mut last: Option<f32> = None;
        for step in 0..1000 {
            let t = step as f32 * 0.002; // 2초
            let (b, _) = led_state(LampMode::Win, LedStrip::Left, 3, 16, t, false, reduce);
            min = min.min(b);
            max = max.max(b);
            if let Some(previous) = last
                && (b - previous).abs() > 0.3
            {
                jumps += 1;
            }
            last = Some(b);
        }
        (min, max, jumps)
    };
    let (min, max, jumps) = sample(false);
    assert!(max - min > 0.5 && jumps >= 10);
    let (min, max, jumps) = sample(true);
    assert!(max - min <= 0.25, "변화 폭 {}", max - min);
    assert_eq!(jumps, 0, "급격한 밝기 변화가 없어야 한다");
}

#[test]
fn layout_is_physically_consistent() {
    // 릴 세 개가 표시창 안에 들어온다
    let span = 3.0 * REEL_WIDTH + 2.0 * REEL_GAP;
    assert!(span <= WINDOW_WIDTH);
    const { assert!(WINDOW_WIDTH < CABINET_WIDTH - 0.06) };
    // 릴은 캐비닛 안에 있다 (앞 유리 뒤, 뒤판 앞)
    const { assert!(REEL_AXIS_Z + REEL_RADIUS < FRONT_Z - 0.01) };
    const { assert!(REEL_AXIS_Z - REEL_RADIUS > -CABINET_DEPTH / 2.0 + WALL) };
    assert!((reel_center(0).x + reel_center(2).x).abs() < 1e-6);
    assert_eq!(reel_center(1).x, 0.0);
    // 표시창 높이는 세 줄(±1.5칸)의 현 길이와 거의 같다
    let half_angle = 1.5 * std::f32::consts::TAU / 21.0;
    let chord = 2.0 * REEL_RADIUS * half_angle.sin();
    assert!(
        (chord - WINDOW_HEIGHT).abs() < 0.004,
        "{chord} vs {WINDOW_HEIGHT}"
    );
    // 앞면 구획이 아래에서 위로 겹치지 않고 쌓인다
    let zones = [
        CABINET_BOTTOM,
        BELLY_BOTTOM,
        BELLY_TOP,
        DECK_BOTTOM,
        DECK_TOP,
        REEL_PANEL_BOTTOM,
        REEL_PANEL_TOP,
        TOP_PANEL_BOTTOM,
        TOP_PANEL_TOP,
        CABINET_TOP,
    ];
    assert!(zones.windows(2).all(|pair| pair[0] < pair[1]), "{zones:?}");
    // 정지 버튼은 각 릴 바로 아래에 있다
    for reel in 0..3 {
        assert_eq!(stop_button_pos(reel).x, reel_center(reel).x);
    }
    // 메달은 배출구와 받침 안에 들어간다
    const { assert!(CHUTE_POS.y > TRAY_FLOOR_Y + MEDAL_RADIUS) };
    const { assert!(TRAY_HALF_WIDTH > 4.0 * MEDAL_RADIUS) };
}
