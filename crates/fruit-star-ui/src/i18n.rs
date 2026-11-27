//! 다국어 문자열 (한국어·일본어·영어).

use fruit_star_config::Language;

/// 문자열 키에 해당하는 번역. 없는 키는 키 자체를 돌려준다.
pub fn tr(language: Language, key: &str) -> &str {
    let index = match language {
        Language::Ko => 0,
        Language::Ja => 1,
        Language::En => 2,
    };
    TABLE
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, texts)| texts[index])
}

/// 번역표의 모든 키.
pub fn keys() -> impl Iterator<Item = &'static str> {
    TABLE.iter().map(|(key, _)| *key)
}

/// 언어별 UI 글꼴 (macOS 시스템 글꼴).
pub fn font_family(language: Language) -> &'static str {
    match language {
        Language::Ko => "Apple SD Gothic Neo",
        Language::Ja => "Hiragino Sans",
        Language::En => "Helvetica Neue",
    }
}

/// (키, [한국어, 일본어, 영어])
const TABLE: &[(&str, [&str; 3])] = &[
    ("title", ["FRUIT STAR 3D", "FRUIT STAR 3D", "FRUIT STAR 3D"]),
    (
        "subtitle",
        [
            "기계식 릴 시뮬레이터 · 가상 메달 전용",
            "メカニカルリール · 仮想メダル専用",
            "Mechanical reels · Virtual medals only",
        ],
    ),
    (
        "menu.play_modes",
        ["플레이 모드", "プレイモード", "Play mode"],
    ),
    (
        "menu.views",
        ["게임기 둘러보기", "筐体を見る", "Explore the cabinet"],
    ),
    ("menu.machine", ["기종", "機種", "Cabinet"]),
    ("menu.records", ["플레이 기록", "プレイ記録", "Records"]),
    ("menu.back", ["모드 선택", "モード選択", "Modes"]),
    (
        "hint.classic",
        [
            "보유한 메달로 시작합니다",
            "手持ちのメダルで遊ぶ",
            "Play with your saved medals",
        ],
    ),
    (
        "hint.free_play",
        [
            "메달을 자동으로 채워 계속 플레이합니다",
            "メダルを自動で補充して遊ぶ",
            "Keep playing with automatic medal refills",
        ],
    ),
    (
        "hint.practice",
        [
            "내부 추첨을 보며 릴 정지를 연습합니다",
            "抽選結果を見ながらリール停止を練習",
            "See each draw and practice stopping the reels",
        ],
    ),
    (
        "hud.show_keys",
        ["조작표 열기", "操作表を開く", "Show controls"],
    ),
    (
        "hud.hide_keys",
        ["조작표 닫기", "操作表を閉じる", "Hide controls"],
    ),
    ("keys.max_bet", ["최대 베팅", "最大ベット", "Max bet"]),
    (
        "keys.one_bet",
        ["메달 한 장 베팅", "1枚ベット", "Bet one medal"],
    ),
    ("keys.start", ["레버 당기기", "レバーを引く", "Pull lever"]),
    (
        "keys.left",
        ["왼쪽 릴 정지", "左リール停止", "Stop left reel"],
    ),
    (
        "keys.center",
        ["가운데 릴 정지", "中リール停止", "Stop center reel"],
    ),
    (
        "keys.right",
        ["오른쪽 릴 정지", "右リール停止", "Stop right reel"],
    ),
    ("keys.medals", ["메달 추가", "メダル補充", "Add medals"]),
    (
        "keys.orbit",
        [
            "드래그로 회전 · 스크롤이나 핀치로 확대",
            "ドラッグで回転 · スクロールやピンチで拡大",
            "Drag to orbit · Scroll or pinch to zoom",
        ],
    ),
    ("keys.pause", ["일시정지", "一時停止", "Pause"]),
    ("keys.camera", ["시점 전환", "視点切替", "Camera view"]),
    ("keys.fullscreen", ["전체 화면", "全画面", "Fullscreen"]),
    ("keys.mute", ["음소거 전환", "消音切替", "Toggle mute"]),
    ("settings.sound", ["음향", "サウンド", "Sound"]),
    (
        "settings.picture",
        ["화면 · 조명", "画面・照明", "Display & lighting"],
    ),
    ("loading", ["불러오는 중…", "読み込み中…", "Loading…"]),
    (
        "menu.classic",
        ["클래식 플레이", "クラシックプレイ", "Play Classic"],
    ),
    (
        "menu.free_play",
        ["프리 플레이", "フリープレイ", "Free Play"],
    ),
    ("menu.practice", ["연습 플레이", "練習プレイ", "Practice"]),
    ("menu.mechanical", ["내부 관찰", "内部を見る", "Mechanism"]),
    ("menu.gallery", ["갤러리", "ギャラリー", "Gallery"]),
    ("menu.language", ["한국어", "日本語", "English"]),
    (
        "menu.reset",
        ["기록 초기화", "記録を初期化", "Reset records"],
    ),
    (
        "menu.reset_confirm",
        [
            "한 번 더 누르면 모든 기록이 지워집니다",
            "もう一度押すと全記録が消えます",
            "Press again to erase all records",
        ],
    ),
    ("menu.quit", ["종료", "終了", "Quit"]),
    ("menu.resume", ["계속하기", "再開", "Resume"]),
    (
        "menu.to_menu",
        ["메인 메뉴로", "メインメニューへ", "Main menu"],
    ),
    ("pause.title", ["일시정지", "一時停止", "Paused"]),
    (
        "settings.silent_hint",
        [
            "소리가 안 나면 무음 모드와 기기 볼륨을 확인하세요",
            "音が出ないときはマナーモードと本体の音量を確認してください",
            "No sound? Check silent mode and the device volume",
        ],
    ),
    ("mode.classic", ["클래식", "クラシック", "Classic"]),
    (
        "mode.free_play",
        ["프리 플레이", "フリープレイ", "Free Play"],
    ),
    ("mode.practice", ["연습", "練習", "Practice"]),
    ("view.mechanical", ["내부 관찰", "内部観察", "Mechanical"]),
    ("view.gallery", ["둘러보기", "鑑賞", "Gallery"]),
    ("hud.credit", ["크레딧", "クレジット", "Credit"]),
    ("hud.payout", ["지급", "払出", "Payout"]),
    ("hud.bet", ["베팅", "ベット", "Bet"]),
    (
        "hud.bonus_left",
        ["보너스 남은 지급", "ボーナス残り払出", "Bonus payout left"],
    ),
    ("touch.medal", ["+ 메달", "+ メダル", "+ MEDALS"]),
    ("touch.bet_one", ["1 BET", "1 BET", "1 BET"]),
    ("touch.max_bet", ["MAX BET", "MAX BET", "MAX BET"]),
    ("touch.start", ["START", "START", "START"]),
    ("touch.stop", ["STOP", "STOP", "STOP"]),
    ("touch.view", ["시점", "視点", "VIEW"]),
    ("touch.pause", ["일시정지", "一時停止", "PAUSE"]),
    ("hud.win", ["당첨!", "当たり!", "wins!"]),
    ("role.big_red", ["빨간 7", "赤7", "Red 7"]),
    ("role.big_blue", ["파란 7", "青7", "Blue 7"]),
    ("role.regular", ["BAR", "BAR", "BAR"]),
    (
        "hud.need_medals",
        [
            "메달을 추가하세요",
            "メダルを追加してください",
            "Add medals to play",
        ],
    ),
    ("hud.muted", ["음소거", "消音中", "Muted"]),
    ("practice.flag", ["내부 추첨", "内部抽選", "Internal draw"]),
    (
        "practice.carried",
        ["이월 보너스", "持ち越しボーナス", "Carried bonus"],
    ),
    ("flag.blank", ["꽝", "ハズレ", "Blank"]),
    ("flag.replay", ["리플레이", "リプレイ", "Replay"]),
    ("flag.grape", ["포도", "ブドウ", "Grape"]),
    ("flag.cherry", ["체리", "チェリー", "Cherry"]),
    ("flag.bell", ["벨", "ベル", "Bell"]),
    ("flag.melon", ["수박", "スイカ", "Melon"]),
    ("flag.star", ["스타", "スター", "Star"]),
    (
        "flag.big",
        ["BIG 보너스 (7)", "BIGボーナス (7)", "BIG bonus (7)"],
    ),
    (
        "flag.regular",
        [
            "REGULAR 보너스 (BAR)",
            "REGULARボーナス (BAR)",
            "REGULAR bonus (BAR)",
        ],
    ),
    ("flag.none", ["없음", "なし", "None"]),
    (
        "bonus.big",
        ["★ BIG BONUS ★", "★ BIG BONUS ★", "★ BIG BONUS ★"],
    ),
    (
        "bonus.regular",
        [
            "★ REGULAR BONUS ★",
            "★ REGULAR BONUS ★",
            "★ REGULAR BONUS ★",
        ],
    ),
    (
        "settings.master",
        ["전체 음량", "全体音量", "Master volume"],
    ),
    ("settings.effects", ["효과음", "効果音", "Effects"]),
    ("settings.ambient", ["배경음", "環境音", "Ambient"]),
    ("settings.led", ["LED 밝기", "LED明るさ", "LED brightness"]),
    (
        "settings.reduce",
        ["점멸 줄이기", "点滅を減らす", "Reduce flashing"],
    ),
    (
        "settings.quality",
        [
            "그래픽 품질 (재시작 후 적용)",
            "画質 (再起動後に適用)",
            "Graphics quality (after restart)",
        ],
    ),
    ("on", ["켬", "オン", "On"]),
    ("off", ["끔", "オフ", "Off"]),
    ("stats.games", ["플레이", "プレイ", "Games"]),
    ("stats.in_out", ["투입 / 지급", "投入 / 払出", "In / Out"]),
    (
        "stats.bonus",
        ["보너스 BIG / REG", "ボーナス BIG / REG", "Bonus BIG / REG"],
    ),
    ("stats.time", ["플레이 시간", "プレイ時間", "Play time"]),
    ("stats.achievements", ["달성", "実績", "Achievements"]),
    (
        "stats.not_saved",
        [
            "이번 실행의 기록만 표시합니다",
            "記録は今回の起動中のみ有効です",
            "Records are kept for this run only",
        ],
    ),
    (
        "failed.title",
        ["실행할 수 없습니다", "実行できません", "Cannot start"],
    ),
    (
        "notice.history_reset",
        [
            "기록을 초기화했습니다",
            "記録を初期化しました",
            "Records were reset",
        ],
    ),
    (
        "notice.storage_error",
        [
            "기록을 저장하지 못했습니다",
            "記録を保存できませんでした",
            "Could not save records",
        ],
    ),
    (
        "achievement.first_win",
        ["달성: 첫 당첨", "実績: 初当たり", "Achievement: first win"],
    ),
    (
        "achievement.first_replay",
        [
            "달성: 첫 리플레이",
            "実績: 初リプレイ",
            "Achievement: first replay",
        ],
    ),
    (
        "achievement.first_big_bonus",
        [
            "달성: 첫 BIG 보너스",
            "実績: 初BIGボーナス",
            "Achievement: first BIG bonus",
        ],
    ),
    (
        "achievement.first_regular_bonus",
        [
            "달성: 첫 REGULAR 보너스",
            "実績: 初REGULARボーナス",
            "Achievement: first REGULAR bonus",
        ],
    ),
    (
        "achievement.star_trio",
        [
            "달성: 스타 세 개",
            "実績: スター揃い",
            "Achievement: three stars",
        ],
    ),
    (
        "achievement.games_100",
        ["달성: 100게임", "実績: 100ゲーム", "Achievement: 100 games"],
    ),
    (
        "achievement.games_1000",
        [
            "달성: 1000게임",
            "実績: 1000ゲーム",
            "Achievement: 1000 games",
        ],
    ),
];
