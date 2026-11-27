#![allow(clippy::unwrap_used, clippy::expect_used)]

use fruit_star_core::{Reel, ReelError, ReelState, ReelTiming, Symbol};

fn strip() -> Vec<Symbol> {
    (0..21)
        .map(|i| Symbol::ALL[i % Symbol::ALL.len()])
        .collect()
}

fn reel() -> Reel {
    Reel::new(0, strip(), ReelTiming::default()).unwrap()
}

/// 일정한 프레임 간격으로 정확히 `seconds`초 동안 진행한다.
/// 프레임 간격으로 나누어떨어지지 않는 나머지는 마지막에 한 번 더 진행한다.
fn run(reel: &mut Reel, seconds: f64, fps: f64) {
    let frame = 1.0 / fps;
    let steps = (seconds / frame).floor() as usize;
    for _ in 0..steps {
        reel.update(frame);
    }
    let rest = seconds - steps as f64 * frame;
    if rest > 0.0 {
        reel.update(rest);
    }
}

#[test]
fn starts_idle_and_reaches_constant_speed() {
    let mut reel = reel();
    assert_eq!(reel.state(), ReelState::Idle);
    reel.start().unwrap();
    assert_eq!(reel.state(), ReelState::Accelerating);
    let update = reel.update(0.2);
    assert!(!update.reached_speed);
    assert_eq!(reel.state(), ReelState::Accelerating);
    let update = reel.update(0.2);
    assert!(update.reached_speed);
    assert_eq!(reel.state(), ReelState::Spinning);
    let expected = (reel.timing().spin_speed * reel.step_angle()) as f32;
    assert!((reel.angular_velocity() - expected).abs() < 1e-4);
}

#[test]
fn acceleration_distance_matches_kinematics() {
    let timing = ReelTiming::default();
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(timing.acceleration_s);
    // 등가속 운동: 이동 거리 = v * t / 2
    let expected = timing.spin_speed * timing.acceleration_s / 2.0;
    assert!((reel.position() - expected.rem_euclid(21.0)).abs() < 1e-9);
}

#[test]
fn stops_exactly_on_requested_index_and_passes_through_all_states() {
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(0.5);
    let target = (reel.next_index() + 3) % 21;
    reel.request_stop(target).unwrap();
    assert_eq!(reel.state(), ReelState::Decelerating);
    assert_eq!(reel.target_index(), Some(target));

    let mut locked = false;
    let mut settled = false;
    let mut seen_settling = false;
    for _ in 0..600 {
        let update = reel.update(1.0 / 120.0);
        locked |= update.locked;
        settled |= update.settled;
        seen_settling |= reel.state() == ReelState::Settling;
        if settled {
            break;
        }
    }
    assert!(locked && settled && seen_settling);
    assert_eq!(reel.state(), ReelState::Stopped);
    assert_eq!(reel.current_index(), target);
    assert_eq!(reel.position(), target as f64);
    assert_eq!(reel.angular_velocity(), 0.0);
    // 정지 후에는 진동 오프셋이 남지 않아 표시 각도와 논리 위치가 일치한다.
    assert_eq!(reel.display_position(), target as f64);
    let expected_angle = (target as f64 * reel.step_angle()) as f32;
    assert_eq!(reel.current_angle(), expected_angle);
    assert_eq!(reel.target_angle(), Some(expected_angle));
}

#[test]
fn result_is_independent_of_frame_rate() {
    let mut reference: Option<(usize, f64)> = None;
    for fps in [24.0, 30.0, 60.0, 120.0, 144.0, 240.0, 1000.0] {
        let mut reel = reel();
        reel.start().unwrap();
        // 프레임률과 무관하게 같은 시각에 정지 입력이 들어오도록 한 번에 진행한다.
        reel.update(0.777);
        let pressed = reel.next_index();
        let target = (pressed + 2) % 21;
        reel.request_stop(target).unwrap();
        // 고정 시간 뒤의 위치를 비교한다 (감속 도중).
        let mid = {
            let mut probe = reel.clone();
            run(&mut probe, 0.05, fps);
            probe.position()
        };
        run(&mut reel, 1.0, fps);
        assert_eq!(reel.state(), ReelState::Stopped, "fps {fps}");
        assert_eq!(reel.current_index(), target, "fps {fps}");
        match reference {
            None => reference = Some((target, mid)),
            Some((ref_target, ref_mid)) => {
                assert_eq!(target, ref_target);
                assert!(
                    (mid - ref_mid).abs() < 1e-6,
                    "fps {fps}: 감속 중 위치 {mid} != {ref_mid}"
                );
            }
        }
    }
}

#[test]
fn a_single_huge_time_step_still_lands_on_target() {
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(1.0);
    reel.request_stop(7).unwrap();
    let update = reel.update(30.0);
    assert!(update.locked);
    assert_eq!(reel.current_index(), 7);
    reel.update(1.0);
    assert_eq!(reel.state(), ReelState::Stopped);
}

#[test]
fn stop_time_is_bounded_by_slip_distance() {
    // 4칸 미끄러짐 + 누른 순간의 한 칸 미만 거리 → 최대 5칸.
    let timing = ReelTiming::default();
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(0.6);
    let target = (reel.next_index() + 4) % 21;
    reel.request_stop(target).unwrap();
    let mut elapsed = 0.0;
    while reel.state() == ReelState::Decelerating {
        reel.update(0.001);
        elapsed += 0.001;
        assert!(elapsed < 2.0);
    }
    let upper = 5.0 / timing.spin_speed + timing.deceleration_s;
    assert!(elapsed <= upper + 0.002, "정지 시간 {elapsed} > {upper}");
}

#[test]
fn duplicate_and_premature_stop_requests_are_rejected() {
    let mut reel = reel();
    assert!(matches!(
        reel.request_stop(3),
        Err(ReelError::CannotStop { .. })
    ));
    reel.start().unwrap();
    // 가속 중에는 정지 불가
    assert!(matches!(
        reel.request_stop(3),
        Err(ReelError::CannotStop { .. })
    ));
    reel.update(0.5);
    reel.request_stop(3).unwrap();
    // 중복 정지 입력은 목표를 바꾸지 못한다
    assert!(matches!(
        reel.request_stop(9),
        Err(ReelError::CannotStop { .. })
    ));
    assert_eq!(reel.target_index(), Some(3));
    // 도는 중에는 다시 시작할 수 없다
    assert!(matches!(reel.start(), Err(ReelError::CannotStart { .. })));
    reel.update(5.0);
    reel.update(5.0);
    assert_eq!(reel.current_index(), 3);
    assert!(matches!(
        reel.request_stop(30),
        Err(ReelError::CannotStop { .. })
    ));
}

#[test]
fn out_of_range_target_is_rejected() {
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(0.5);
    assert!(matches!(
        reel.request_stop(21),
        Err(ReelError::IndexOutOfRange { .. })
    ));
    assert_eq!(reel.state(), ReelState::Spinning);
}

#[test]
fn window_shows_upper_index_on_top_row() {
    let reel = reel();
    let s = strip();
    assert_eq!(reel.window_at(5), [s[6], s[5], s[4]]);
    assert_eq!(reel.window_at(0), [s[1], s[0], s[20]]);
    assert_eq!(reel.window_at(20), [s[0], s[20], s[19]]);
}

#[test]
fn next_index_is_the_symbol_about_to_reach_the_centre_row() {
    let mut reel = reel();
    assert_eq!(reel.next_index(), 0);
    reel.start().unwrap();
    reel.update(0.5);
    let position = reel.position();
    assert_eq!(reel.next_index(), position.ceil() as usize % 21);
}

#[test]
fn settling_offset_stays_small_and_vanishes() {
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(0.5);
    reel.request_stop(10).unwrap();
    let mut max_offset: f64 = 0.0;
    for _ in 0..2000 {
        reel.update(0.0005);
        if reel.state() == ReelState::Settling {
            let mut offset = reel.display_position() - 10.0;
            if offset > 10.0 {
                offset -= 21.0;
            }
            max_offset = max_offset.max(offset.abs());
        }
    }
    assert_eq!(reel.state(), ReelState::Stopped);
    assert!(max_offset > 0.0, "정지 진동이 표현되어야 한다");
    assert!(
        max_offset < 0.1,
        "정지 진동은 심볼 정렬을 해치지 않아야 한다"
    );
}

#[test]
fn invalid_construction_is_rejected() {
    assert!(matches!(
        Reel::new(1, Vec::new(), ReelTiming::default()),
        Err(ReelError::EmptyStrip { reel: 1 })
    ));
    let bad = ReelTiming {
        spin_speed: 0.0,
        ..ReelTiming::default()
    };
    assert!(matches!(
        Reel::new(0, strip(), bad),
        Err(ReelError::InvalidTiming(_))
    ));
}

#[test]
fn non_finite_or_negative_dt_is_ignored() {
    let mut reel = reel();
    reel.start().unwrap();
    reel.update(0.1);
    let before = reel.position();
    reel.update(-1.0);
    reel.update(f64::NAN);
    reel.update(f64::INFINITY);
    assert_eq!(reel.position(), before);
}
