//! 스키마와 마이그레이션.

/// (버전, SQL). 버전 순서대로 한 번씩 적용된다. 이미 배포된 항목은 수정하지 않고
/// 새 항목을 추가한다.
pub(crate) const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    r#"
CREATE TABLE machine_profiles (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    symbol_count INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE game_sessions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    machine_id    TEXT NOT NULL REFERENCES machine_profiles(id),
    mode          TEXT NOT NULL,
    started_at    INTEGER NOT NULL,
    ended_at      INTEGER,
    clean_exit    INTEGER NOT NULL DEFAULT 0,
    start_balance INTEGER NOT NULL,
    end_balance   INTEGER,
    games         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE game_history (
    game_id        INTEGER PRIMARY KEY,
    session_id     INTEGER NOT NULL REFERENCES game_sessions(id),
    played_at      INTEGER NOT NULL,
    mode           TEXT NOT NULL,
    bet            INTEGER NOT NULL,
    replay_game    INTEGER NOT NULL,
    in_bonus       INTEGER NOT NULL,
    flag           TEXT NOT NULL,
    wins           TEXT NOT NULL,
    payout         INTEGER NOT NULL,
    replay_awarded INTEGER NOT NULL,
    bonus_started  TEXT,
    balance_after  INTEGER NOT NULL
);
CREATE INDEX idx_game_history_session ON game_history(session_id);

CREATE TABLE reel_results (
    game_id    INTEGER NOT NULL REFERENCES game_history(game_id) ON DELETE CASCADE,
    reel       INTEGER NOT NULL,
    stop_index INTEGER NOT NULL,
    slip       INTEGER NOT NULL,
    stop_order INTEGER NOT NULL,
    top        TEXT NOT NULL,
    middle     TEXT NOT NULL,
    bottom     TEXT NOT NULL,
    PRIMARY KEY (game_id, reel)
);

CREATE TABLE credit_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id    INTEGER NOT NULL REFERENCES game_sessions(id),
    game_id       INTEGER,
    kind          TEXT NOT NULL,
    amount        INTEGER NOT NULL,
    balance_after INTEGER NOT NULL,
    created_at    INTEGER NOT NULL
);
CREATE INDEX idx_credit_history_session ON credit_history(session_id);

CREATE TABLE user_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE achievements (
    id          TEXT PRIMARY KEY,
    unlocked_at INTEGER NOT NULL,
    game_id     INTEGER
);
"#,
)];

/// 이 빌드가 지원하는 최신 스키마 버전.
pub(crate) fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |(version, _)| *version)
}
