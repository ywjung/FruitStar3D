#![allow(clippy::unwrap_used, clippy::expect_used)]

use fruit_star_config::Language;
use fruit_star_ui::i18n::{font_family, keys, tr};

#[test]
fn every_key_is_translated_in_every_language() {
    let mut seen = Vec::new();
    for key in keys() {
        assert!(!seen.contains(&key), "키 중복: {key}");
        seen.push(key);
        for language in Language::ALL {
            let text = tr(language, key);
            assert!(!text.trim().is_empty(), "{key} ({language:?})");
            assert_ne!(text, key, "{key} ({language:?})는 번역이 있어야 한다");
        }
    }
    assert!(seen.len() > 60);
}

#[test]
fn keys_used_by_the_ui_exist() {
    let used = [
        "title",
        "subtitle",
        "loading",
        "menu.classic",
        "menu.free_play",
        "menu.practice",
        "menu.mechanical",
        "menu.gallery",
        "menu.language",
        "menu.reset",
        "menu.reset_confirm",
        "menu.quit",
        "menu.resume",
        "menu.to_menu",
        "pause.title",
        "mode.classic",
        "mode.free_play",
        "mode.practice",
        "view.mechanical",
        "view.gallery",
        "hud.credit",
        "hud.payout",
        "hud.bet",
        "hud.bonus_left",
        "hud.win",
        "role.big_red",
        "role.big_blue",
        "role.regular",
        "menu.play_modes",
        "menu.views",
        "menu.machine",
        "menu.records",
        "menu.back",
        "hint.classic",
        "hint.free_play",
        "hint.practice",
        "hud.show_keys",
        "hud.hide_keys",
        "keys.max_bet",
        "keys.one_bet",
        "keys.start",
        "keys.left",
        "keys.center",
        "keys.right",
        "keys.medals",
        "keys.orbit",
        "keys.pause",
        "keys.camera",
        "keys.fullscreen",
        "keys.mute",
        "settings.sound",
        "settings.picture",
        "hud.need_medals",
        "hud.muted",
        "practice.flag",
        "practice.carried",
        "flag.blank",
        "flag.replay",
        "flag.grape",
        "flag.cherry",
        "flag.bell",
        "flag.melon",
        "flag.star",
        "flag.big",
        "flag.regular",
        "flag.none",
        "bonus.big",
        "bonus.regular",
        "settings.master",
        "settings.effects",
        "settings.ambient",
        "settings.led",
        "settings.reduce",
        "settings.quality",
        "on",
        "off",
        "stats.games",
        "stats.in_out",
        "stats.bonus",
        "stats.time",
        "stats.achievements",
        "stats.not_saved",
        "failed.title",
        "notice.history_reset",
        "notice.storage_error",
        "achievement.first_win",
        "achievement.first_replay",
        "achievement.first_big_bonus",
        "achievement.first_regular_bonus",
        "achievement.star_trio",
        "achievement.games_100",
        "achievement.games_1000",
    ];
    let all: Vec<&str> = keys().collect();
    for key in used {
        assert!(all.contains(&key), "번역표에 {key}가 없다");
    }
}

#[test]
fn unknown_keys_fall_back_to_the_key_itself() {
    assert_eq!(tr(Language::Ko, "no.such.key"), "no.such.key");
    for language in Language::ALL {
        assert!(!font_family(language).is_empty());
    }
}
