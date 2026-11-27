#![allow(clippy::unwrap_used, clippy::expect_used)]

use fruit_star_core::Symbol;
use fruit_star_game::{
    Allowed, BonusKind, Flag, LOTTERY_DENOMINATOR, LotteryTable, MachineProfile, ProfileError,
    Role, choose_stop, evaluate, windows_at,
};

fn profile() -> MachineProfile {
    MachineProfile::fruit_classic()
}

/// 릴 `reel`에서 `symbol`이 `row`(0=위, 1=가운데, 2=아래)에 오는 정지 인덱스.
fn stop_showing(profile: &MachineProfile, reel: usize, symbol: Symbol, row: usize) -> usize {
    let strip = &profile.strips[reel];
    let len = strip.len();
    let at = strip.iter().position(|s| *s == symbol).unwrap();
    // 표시창은 위에서부터 index+1, index, index-1 을 보여 준다.
    (at + len + row - 1) % len
}

#[test]
fn builtin_profile_is_valid_and_well_formed() {
    let p = profile();
    p.validate().unwrap();
    for strip in &p.strips {
        assert_eq!(strip.len(), 21);
        for symbol in Symbol::ALL {
            assert!(strip.contains(&symbol), "{symbol} 누락");
        }
    }
    let total: u32 = p.lottery.entries().iter().map(|(_, w)| *w).sum();
    assert!(total > 0 && total < LOTTERY_DENOMINATOR);
    assert!(p.bonus.lottery.weight(Flag::Small(Role::Grape)) > 0);
}

#[test]
fn always_pullable_roles_are_spaced_within_slip_range() {
    // 포도와 리플레이는 어디서 눌러도 끌어들일 수 있도록 5칸 이내 간격으로 배치한다.
    let p = profile();
    for strip in &p.strips {
        for symbol in [Symbol::Grape, Symbol::Replay] {
            let positions: Vec<usize> = (0..strip.len()).filter(|&i| strip[i] == symbol).collect();
            for (k, &at) in positions.iter().enumerate() {
                let next = positions[(k + 1) % positions.len()];
                let gap = (next + strip.len() - at) % strip.len();
                assert!(gap <= p.max_slip + 1, "{symbol} 간격 {gap}");
            }
        }
    }
}

#[test]
fn three_of_a_kind_on_each_payline_pays_once() {
    let p = profile();
    // 가운데 줄 포도
    let stops = [
        stop_showing(&p, 0, Symbol::Grape, 1),
        stop_showing(&p, 1, Symbol::Grape, 1),
        stop_showing(&p, 2, Symbol::Grape, 1),
    ];
    let result = evaluate(&p, &windows_at(&p, stops));
    let grape: Vec<_> = result
        .wins
        .iter()
        .filter(|w| w.role == Role::Grape)
        .collect();
    assert_eq!(grape.len(), 1);
    assert_eq!(grape[0].line, 0);
    assert!(result.payout >= 8);

    // 대각선(위→아래) 벨
    let stops = [
        stop_showing(&p, 0, Symbol::Bell, 0),
        stop_showing(&p, 1, Symbol::Bell, 1),
        stop_showing(&p, 2, Symbol::Bell, 2),
    ];
    let result = evaluate(&p, &windows_at(&p, stops));
    assert!(
        result
            .wins
            .iter()
            .any(|w| w.role == Role::Bell && w.line == 3)
    );
}

#[test]
fn replay_and_bonus_roles_are_reported_without_payout() {
    let p = profile();
    let replay = [
        stop_showing(&p, 0, Symbol::Replay, 1),
        stop_showing(&p, 1, Symbol::Replay, 1),
        stop_showing(&p, 2, Symbol::Replay, 1),
    ];
    let result = evaluate(&p, &windows_at(&p, replay));
    assert!(result.replay);
    assert_eq!(
        result
            .wins
            .iter()
            .filter(|w| w.role == Role::Replay)
            .map(|w| w.payout)
            .sum::<u32>(),
        0
    );

    for (symbol, kind) in [
        (Symbol::RedSeven, BonusKind::Big),
        (Symbol::BlueSeven, BonusKind::Big),
        (Symbol::Bar, BonusKind::Regular),
    ] {
        let stops = [
            stop_showing(&p, 0, symbol, 1),
            stop_showing(&p, 1, symbol, 1),
            stop_showing(&p, 2, symbol, 1),
        ];
        let result = evaluate(&p, &windows_at(&p, stops));
        assert_eq!(result.bonus, Some(kind), "{symbol}");
    }
}

#[test]
fn every_symbol_maps_to_exactly_one_role() {
    for symbol in Symbol::ALL {
        let role = Role::for_symbol(symbol).unwrap();
        assert_eq!(role.symbol(), symbol);
    }
    // 두 개만 늘어선 것은 역할이 아니다
    let p = profile();
    let stops = [
        stop_showing(&p, 0, Symbol::Cherry, 1),
        stop_showing(&p, 1, Symbol::Cherry, 1),
        stop_showing(&p, 2, Symbol::Bar, 1),
    ];
    let result = evaluate(&p, &windows_at(&p, stops));
    assert!(!result.has_role(Role::Cherry));
}

#[test]
fn payout_is_capped_per_game() {
    let mut p = profile();
    p.max_payout = 9;
    for role in [Role::Grape, Role::Bell, Role::Melon, Role::Star] {
        p.paytable.set(role, 9);
    }
    // 위·아래 두 줄에 포도가 동시에 늘어서는 정지 위치를 만든다: 각 릴에서 포도 두 칸 위의
    // 리플레이를 포도로 바꾸면, 그 사이 칸에 멈췄을 때 위와 아래가 모두 포도다.
    let mut stops = [0usize; 3];
    for (reel, strip) in p.strips.iter_mut().enumerate() {
        let grape = (0..strip.len() - 2)
            .find(|&i| strip[i] == Symbol::Grape && strip[i + 2] == Symbol::Replay)
            .unwrap();
        strip[grape + 2] = Symbol::Grape;
        stops[reel] = grape + 1;
    }
    p.validate().unwrap();
    let result = evaluate(&p, &windows_at(&p, stops));
    let grape_lines = result.wins.iter().filter(|w| w.role == Role::Grape).count();
    assert!(
        grape_lines >= 2,
        "두 줄 이상 성립해야 한다: {:?}",
        result.wins
    );
    assert!(result.wins.iter().map(|w| w.payout).sum::<u32>() > 9);
    assert_eq!(result.payout, 9, "지급은 게임당 상한으로 제한된다");

    // 어떤 정지 위치에서도 상한을 넘지 않는다
    for a in 0..21 {
        for b in 0..21 {
            for c in 0..21 {
                assert!(evaluate(&p, &windows_at(&p, [a, b, c])).payout <= 9);
            }
        }
    }
}

#[test]
fn lottery_draw_follows_cumulative_weights() {
    struct Fixed(u32);
    impl fruit_star_core::RandomSource for Fixed {
        fn next_u32(&mut self) -> u32 {
            self.0 << 16
        }
    }
    let table = LotteryTable::new(vec![
        (Flag::Small(Role::Replay), 100),
        (Flag::Small(Role::Grape), 200),
        (Flag::Bonus(BonusKind::Big), 50),
    ])
    .unwrap();
    assert_eq!(table.draw(&mut Fixed(0)), Flag::Small(Role::Replay));
    assert_eq!(table.draw(&mut Fixed(99)), Flag::Small(Role::Replay));
    assert_eq!(table.draw(&mut Fixed(100)), Flag::Small(Role::Grape));
    assert_eq!(table.draw(&mut Fixed(299)), Flag::Small(Role::Grape));
    assert_eq!(table.draw(&mut Fixed(300)), Flag::Bonus(BonusKind::Big));
    assert_eq!(table.draw(&mut Fixed(349)), Flag::Bonus(BonusKind::Big));
    assert_eq!(table.draw(&mut Fixed(350)), Flag::Blank);
    assert_eq!(table.draw(&mut Fixed(65_535)), Flag::Blank);
}

#[test]
fn invalid_profiles_are_rejected() {
    assert!(matches!(
        LotteryTable::new(vec![(Flag::Small(Role::Grape), 70_000)]),
        Err(ProfileError::LotteryOverflow { .. })
    ));
    assert!(matches!(
        LotteryTable::new(vec![(Flag::Blank, 1), (Flag::Blank, 1)]),
        Err(ProfileError::DuplicateLotteryEntry(_))
    ));

    let mut p = profile();
    p.strips[1].pop();
    assert!(matches!(
        p.validate(),
        Err(ProfileError::StripLength { reel: 1, .. })
    ));

    let mut p = profile();
    p.paytable.set(Role::Bell, 99);
    assert!(matches!(
        p.validate(),
        Err(ProfileError::PayoutTooLarge { .. })
    ));

    let mut p = profile();
    p.max_slip = 21;
    assert!(matches!(
        p.validate(),
        Err(ProfileError::SlipTooLarge { .. })
    ));

    let mut p = profile();
    for s in p.strips[2].iter_mut() {
        if *s == Symbol::Bell {
            *s = Symbol::Grape;
        }
    }
    assert!(matches!(
        p.validate(),
        Err(ProfileError::MissingSymbol {
            reel: 2,
            symbol: "bell"
        })
    ));

    let mut p = profile();
    p.bet = 0;
    assert!(matches!(
        p.validate(),
        Err(ProfileError::MustBePositive("bet"))
    ));
}

#[test]
fn flag_keys_round_trip() {
    for flag in [
        Flag::Blank,
        Flag::Small(Role::Replay),
        Flag::Small(Role::Cherry),
        Flag::Bonus(BonusKind::Big),
        Flag::Bonus(BonusKind::Regular),
    ] {
        assert_eq!(Flag::from_key(flag.key()), Some(flag));
    }
    // 보너스 역할 이름은 소역 플래그가 될 수 없다
    assert_eq!(Flag::from_key("big_red"), None);
    assert_eq!(Flag::from_key("nope"), None);
}

const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fn all_allowed() -> Vec<Allowed> {
    let smalls = [
        None,
        Some(Role::Replay),
        Some(Role::Grape),
        Some(Role::Cherry),
        Some(Role::Bell),
        Some(Role::Melon),
        Some(Role::Star),
    ];
    let bonuses = [None, Some(BonusKind::Big), Some(BonusKind::Regular)];
    let mut out = Vec::new();
    for small in smalls {
        for bonus in bonuses {
            out.push(Allowed { small, bonus });
        }
    }
    out
}

/// 한 (허용 역할, 정지 순서)에 대해 모든 누름 위치를 진행하며 `check`를 호출한다.
/// 앞선 릴의 정지 결정을 재사용한다.
fn for_each_game(
    p: &MachineProfile,
    allowed: Allowed,
    order: [usize; 3],
    mut check: impl FnMut([usize; 3], &fruit_star_game::Evaluation, usize),
) {
    let [r1, r2, r3] = order;
    for p1 in 0..21 {
        let d1 = choose_stop(p, allowed, [None; 3], r1, p1);
        assert_eq!(d1.index, (p1 + d1.slip) % 21);
        let mut s1 = [None; 3];
        s1[r1] = Some(d1.index);
        for p2 in 0..21 {
            let d2 = choose_stop(p, allowed, s1, r2, p2);
            let mut s2 = s1;
            s2[r2] = Some(d2.index);
            for p3 in 0..21 {
                let d3 = choose_stop(p, allowed, s2, r3, p3);
                let mut stops = [0usize; 3];
                stops[r1] = d1.index;
                stops[r2] = d2.index;
                stops[r3] = d3.index;
                let mut pressed = [0usize; 3];
                pressed[r1] = p1;
                pressed[r2] = p2;
                pressed[r3] = p3;
                let result = evaluate(p, &windows_at(p, stops));
                check(pressed, &result, d1.slip.max(d2.slip).max(d3.slip));
            }
        }
    }
}

/// 모든 허용 역할 조합 × 모든 정지 순서 × 모든 누름 위치에 대해
/// 릴 제어가 허용되지 않은 역할을 절대 성립시키지 않고, 미끄러짐이 한도 안임을 전수 검사한다.
#[test]
fn reel_control_never_produces_a_disallowed_win() {
    let p = profile();
    let mut max_slip_seen = 0;
    let mut checked = 0u64;
    for allowed in all_allowed() {
        for order in ORDERS {
            for_each_game(&p, allowed, order, |pressed, result, slip| {
                max_slip_seen = max_slip_seen.max(slip);
                for win in &result.wins {
                    assert!(
                        allowed.permits(win.role),
                        "허용 {allowed:?}, 순서 {order:?}, 누름 {pressed:?} → {win:?}"
                    );
                }
                checked += 1;
            });
        }
    }
    assert!(
        max_slip_seen <= p.max_slip,
        "미끄러짐 {max_slip_seen}칸 발생"
    );
    assert_eq!(checked, 21 * 6 * 9_261);
}

/// 포도와 리플레이는 어디서 어떤 순서로 눌러도 반드시 성립한다.
#[test]
fn grape_and_replay_are_always_pulled_in() {
    let p = profile();
    for role in [Role::Grape, Role::Replay] {
        for bonus in [None, Some(BonusKind::Big), Some(BonusKind::Regular)] {
            let allowed = Allowed {
                small: Some(role),
                bonus,
            };
            for order in ORDERS {
                for_each_game(&p, allowed, order, |pressed, result, _| {
                    assert!(
                        result.has_role(role),
                        "{role:?} 불성립: 보너스 {bonus:?}, 순서 {order:?}, 누름 {pressed:?}"
                    );
                });
            }
        }
    }
}

/// 보너스 심볼을 정확히 노리면 맞출 수 있다.
#[test]
fn bonus_symbols_can_be_aligned_when_aimed() {
    let p = profile();
    for (symbol, kind) in [
        (Symbol::RedSeven, BonusKind::Big),
        (Symbol::BlueSeven, BonusKind::Big),
        (Symbol::Bar, BonusKind::Regular),
    ] {
        let allowed = Allowed {
            small: None,
            bonus: Some(kind),
        };
        let pressed = [
            stop_showing(&p, 0, symbol, 1),
            stop_showing(&p, 1, symbol, 1),
            stop_showing(&p, 2, symbol, 1),
        ];
        let mut stops = [None; 3];
        for reel in 0..3 {
            let d = choose_stop(&p, allowed, stops, reel, pressed[reel]);
            stops[reel] = Some(d.index);
        }
        let stops = [stops[0].unwrap(), stops[1].unwrap(), stops[2].unwrap()];
        let result = evaluate(&p, &windows_at(&p, stops));
        assert_eq!(result.bonus, Some(kind), "{symbol}");
    }
}

/// 꽝일 때는 필요한 만큼만 미끄러진다: 누른 위치가 합법이면 그대로 멈춘다.
#[test]
fn blank_games_stop_where_pressed_when_legal() {
    let p = profile();
    let mut zero_slip = 0;
    let mut total = 0;
    for pressed in 0..21 {
        let d = choose_stop(&p, Allowed::NONE, [None; 3], 1, pressed);
        total += 1;
        if d.slip == 0 {
            zero_slip += 1;
        }
    }
    // 첫 번째로 멈추는 가운데 릴은 어떤 위치도 불법이 아니므로 항상 그대로 멈춘다.
    assert_eq!(zero_slip, total);
}

/// 1메달(가운데 한 줄)과 2메달(가로 세 줄) 게임에서도 릴 제어의 두 보장이 성립한다:
/// 허용되지 않은 역할은 유효 라인에서 성립하지 않고, 포도·리플레이는 반드시 성립한다.
#[test]
fn guarantees_hold_with_fewer_active_lines() {
    use fruit_star_game::{choose_stop_lines, evaluate_lines};
    let p = profile();
    for lines in [1usize, 3] {
        for allowed in all_allowed() {
            for order in ORDERS {
                let [r1, r2, r3] = order;
                for p1 in 0..21 {
                    let d1 = choose_stop_lines(&p, allowed, lines, [None; 3], r1, p1);
                    let mut s1 = [None; 3];
                    s1[r1] = Some(d1.index);
                    for p2 in 0..21 {
                        let d2 = choose_stop_lines(&p, allowed, lines, s1, r2, p2);
                        let mut s2 = s1;
                        s2[r2] = Some(d2.index);
                        for p3 in 0..21 {
                            let d3 = choose_stop_lines(&p, allowed, lines, s2, r3, p3);
                            let mut stops = [0usize; 3];
                            stops[r1] = d1.index;
                            stops[r2] = d2.index;
                            stops[r3] = d3.index;
                            assert!(d1.slip.max(d2.slip).max(d3.slip) <= p.max_slip);
                            let result = evaluate_lines(&p, &windows_at(&p, stops), lines);
                            for win in &result.wins {
                                assert!((win.line as usize) < lines);
                                assert!(
                                    allowed.permits(win.role),
                                    "라인 {lines}, 허용 {allowed:?}, 순서 {order:?} → {win:?}"
                                );
                            }
                            if let Some(role @ (Role::Grape | Role::Replay)) = allowed.small {
                                assert!(
                                    result.has_role(role),
                                    "라인 {lines}: {role:?} 불성립 (순서 {order:?}, 누름 {p1},{p2},{p3})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn active_lines_follow_the_bet() {
    use fruit_star_game::active_lines;
    assert_eq!(active_lines(0), 0);
    assert_eq!(active_lines(1), 1);
    assert_eq!(active_lines(2), 3);
    assert_eq!(active_lines(3), 5);
    // 유효 라인은 가운데 → 위 → 아래 → 대각선 순으로 늘어난다
    assert_eq!(fruit_star_game::PAYLINES[0], [1, 1, 1]);
    assert_eq!(fruit_star_game::PAYLINES[1], [0, 0, 0]);
    assert_eq!(fruit_star_game::PAYLINES[2], [2, 2, 2]);
}
