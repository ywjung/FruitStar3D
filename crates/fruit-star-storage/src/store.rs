//! 저장소 구현.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fruit_star_core::{LedgerEntry, REEL_COUNT};
use fruit_star_game::{BonusKind, GameOutcome, MachineProfile, PlayMode, Role};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use thiserror::Error;

use crate::schema::{MIGRATIONS, latest_version};

/// Classic 모드의 보유 메달을 보관하는 설정 키.
const CLASSIC_BALANCE_KEY: &str = "internal.classic_balance";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("데이터베이스 오류: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("저장 경로를 준비할 수 없습니다 ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "데이터베이스 스키마 버전 {found}은(는) 이 프로그램이 지원하는 버전 {supported}보다 새롭습니다"
    )]
    SchemaTooNew { found: i64, supported: i64 },
    #[error("게임 {0}은(는) 이미 기록되어 있습니다")]
    DuplicateGame(u64),
    #[error("세션 {0}이(가) 없습니다")]
    UnknownSession(i64),
}

/// 달성 기록.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Achievement {
    FirstWin,
    FirstReplay,
    FirstBigBonus,
    FirstRegularBonus,
    StarTrio,
    Games100,
    Games1000,
}

impl Achievement {
    pub const ALL: [Achievement; 7] = [
        Achievement::FirstWin,
        Achievement::FirstReplay,
        Achievement::FirstBigBonus,
        Achievement::FirstRegularBonus,
        Achievement::StarTrio,
        Achievement::Games100,
        Achievement::Games1000,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Achievement::FirstWin => "first_win",
            Achievement::FirstReplay => "first_replay",
            Achievement::FirstBigBonus => "first_big_bonus",
            Achievement::FirstRegularBonus => "first_regular_bonus",
            Achievement::StarTrio => "star_trio",
            Achievement::Games100 => "games_100",
            Achievement::Games1000 => "games_1000",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.key() == key)
    }
}

/// 누적 통계.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Statistics {
    pub sessions: u64,
    pub games: u64,
    /// 종료된 세션의 플레이 시간 합계 (초).
    pub play_seconds: u64,
    pub total_bet: u64,
    pub total_payout: u64,
    pub replays: u64,
    pub big_bonuses: u64,
    pub regular_bonuses: u64,
    pub max_payout: u64,
}

/// 비정상 종료 복구 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecoveryReport {
    /// 정상 종료되지 않아 이번에 닫은 세션 수.
    pub unclean_sessions: u64,
    /// 보유 메달 값을 원장 기록으로부터 바로잡았는지 여부.
    pub balance_repaired: bool,
}

/// macOS Application Support 아래의 기본 데이터베이스 경로.
pub fn default_database_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("FruitStar3D")
            .join("fruitstar.sqlite3"),
    )
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// SQLite 저장소.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    /// 파일 데이터베이스를 열고 마이그레이션한다. 상위 디렉터리가 없으면 만든다.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let conn = Connection::open(path)?;
        // WAL 모드: 기록 도중 프로세스가 죽어도 커밋된 트랜잭션은 보존된다.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        Self::init(conn)
    }

    /// 메모리 데이터베이스 (테스트·저장 불가 환경용).
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                 version    INTEGER PRIMARY KEY,
                 applied_at INTEGER NOT NULL
             );",
        )?;
        let current = self.schema_version()?;
        let supported = latest_version();
        if current > supported {
            return Err(StoreError::SchemaTooNew {
                found: current,
                supported,
            });
        }
        for (version, sql) in MIGRATIONS {
            if *version <= current {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                params![version, now()],
            )?;
            tx.commit()?;
        }
        Ok(())
    }

    /// 현재 스키마 버전.
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )?)
    }

    // ---- 복구 ----------------------------------------------------------

    /// 비정상 종료 흔적을 정리한다. 앱 시작 시 한 번 호출한다.
    ///
    /// 게임 결과와 원장 변경은 한 트랜잭션으로 기록되므로 중간 상태는 남지 않는다.
    /// 여기서는 닫히지 않은 세션을 닫고, 보유 메달 값이 원장의 마지막 기록과
    /// 다르면 원장을 기준으로 바로잡는다.
    pub fn recover(&mut self) -> Result<RecoveryReport, StoreError> {
        let tx = self.conn.transaction()?;
        let timestamp = now();
        let unclean = tx.execute(
            "UPDATE game_sessions
                SET ended_at = ?1,
                    clean_exit = 0,
                    end_balance = COALESCE(
                        (SELECT balance_after FROM credit_history
                          WHERE session_id = game_sessions.id ORDER BY id DESC LIMIT 1),
                        start_balance)
              WHERE ended_at IS NULL",
            params![timestamp],
        )? as u64;

        let ledger_balance: Option<i64> = tx
            .query_row(
                "SELECT c.balance_after FROM credit_history c
                   JOIN game_sessions s ON s.id = c.session_id
                  WHERE s.mode = ?1 ORDER BY c.id DESC LIMIT 1",
                params![PlayMode::Classic.key()],
                |row| row.get(0),
            )
            .optional()?;
        let stored: Option<i64> =
            read_setting(&tx, CLASSIC_BALANCE_KEY)?.and_then(|value| value.parse().ok());
        let mut balance_repaired = false;
        if let Some(expected) = ledger_balance
            && stored != Some(expected)
        {
            write_setting(&tx, CLASSIC_BALANCE_KEY, &expected.to_string(), timestamp)?;
            balance_repaired = true;
        }
        tx.commit()?;
        Ok(RecoveryReport {
            unclean_sessions: unclean,
            balance_repaired,
        })
    }

    // ---- 세션 ----------------------------------------------------------

    /// 기종 정보를 등록(갱신)한다.
    pub fn upsert_machine(&mut self, profile: &MachineProfile) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO machine_profiles (id, name, symbol_count, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 symbol_count = excluded.symbol_count,
                 updated_at = excluded.updated_at",
            params![profile.id, profile.name, profile.symbol_count as i64, now()],
        )?;
        Ok(())
    }

    /// 새 플레이 세션을 시작하고 세션 ID를 돌려준다.
    pub fn begin_session(
        &mut self,
        machine_id: &str,
        mode: PlayMode,
        start_balance: u32,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO game_sessions (machine_id, mode, started_at, start_balance)
             VALUES (?1, ?2, ?3, ?4)",
            params![machine_id, mode.key(), now(), start_balance],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 세션을 정상 종료로 닫는다.
    pub fn end_session(&mut self, session_id: i64, end_balance: u32) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE game_sessions SET ended_at = ?1, clean_exit = 1, end_balance = ?2
              WHERE id = ?3 AND ended_at IS NULL",
            params![now(), end_balance, session_id],
        )?;
        if changed == 0 {
            let exists: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM game_sessions WHERE id = ?1",
                    params![session_id],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(StoreError::UnknownSession(session_id));
            }
        }
        Ok(())
    }

    // ---- 기록 ----------------------------------------------------------

    /// 게임 한 판의 결과와 그 게임의 원장 변경(베팅·지급)을 한 트랜잭션으로 기록한다.
    /// 새로 달성한 기록을 돌려준다.
    pub fn record_game(
        &mut self,
        session_id: i64,
        outcome: &GameOutcome,
    ) -> Result<Vec<Achievement>, StoreError> {
        let tx = self.conn.transaction()?;
        let timestamp = now();

        let wins = outcome
            .wins
            .iter()
            .map(|win| format!("{}@{}", win.role.key(), win.line))
            .collect::<Vec<_>>()
            .join(",");
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO game_history
                 (game_id, session_id, played_at, mode, bet, replay_game, in_bonus, flag, wins,
                  payout, replay_awarded, bonus_started, balance_after)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                outcome.game_id as i64,
                session_id,
                timestamp,
                outcome.mode.key(),
                outcome.bet,
                outcome.replay_game,
                outcome.in_bonus,
                outcome.flag.key(),
                wins,
                outcome.payout,
                outcome.replay_awarded,
                outcome.bonus_started.map(BonusKind::key),
                outcome.balance_after,
            ],
        )?;
        if inserted == 0 {
            // 게임 ID는 고유 키다. 같은 게임을 두 번 기록하지 않는다.
            return Err(StoreError::DuplicateGame(outcome.game_id));
        }

        for reel in 0..REEL_COUNT {
            let order = outcome
                .stop_order
                .iter()
                .position(|r| *r == reel)
                .unwrap_or(reel);
            let [top, middle, bottom] = outcome.windows[reel];
            tx.execute(
                "INSERT INTO reel_results
                     (game_id, reel, stop_index, slip, stop_order, top, middle, bottom)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    outcome.game_id as i64,
                    reel as i64,
                    outcome.stops[reel] as i64,
                    outcome.slips[reel] as i64,
                    order as i64,
                    top.key(),
                    middle.key(),
                    bottom.key(),
                ],
            )?;
        }

        for entry in outcome
            .bet_entries
            .iter()
            .chain(outcome.payout_entry.iter())
        {
            insert_credit(&tx, session_id, entry, timestamp)?;
        }
        if outcome.mode == PlayMode::Classic {
            write_setting(
                &tx,
                CLASSIC_BALANCE_KEY,
                &outcome.balance_after.to_string(),
                timestamp,
            )?;
        }
        let updated = tx.execute(
            "UPDATE game_sessions SET games = games + 1 WHERE id = ?1",
            params![session_id],
        )?;
        if updated == 0 {
            return Err(StoreError::UnknownSession(session_id));
        }

        let unlocked = unlock_achievements(&tx, outcome, timestamp)?;
        tx.commit()?;
        Ok(unlocked)
    }

    /// 게임과 무관한 원장 변경(메달 투입)을 기록한다.
    pub fn record_credit(
        &mut self,
        session_id: i64,
        mode: PlayMode,
        entry: &LedgerEntry,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        let timestamp = now();
        insert_credit(&tx, session_id, entry, timestamp)?;
        if mode == PlayMode::Classic {
            write_setting(
                &tx,
                CLASSIC_BALANCE_KEY,
                &entry.balance_after.to_string(),
                timestamp,
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 저장된 Classic 모드 보유 메달. 기록이 없으면 `None`.
    pub fn classic_balance(&self) -> Result<Option<u32>, StoreError> {
        Ok(read_setting(&self.conn, CLASSIC_BALANCE_KEY)?.and_then(|v| v.parse().ok()))
    }

    /// 다음에 부여할 게임 ID (기록된 최대 ID + 1).
    pub fn next_game_id(&self) -> Result<u64, StoreError> {
        let max: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(game_id), 0) FROM game_history",
            [],
            |row| row.get(0),
        )?;
        Ok(max as u64 + 1)
    }

    // ---- 설정 ----------------------------------------------------------

    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(read_setting(&self.conn, key)?)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        write_setting(&self.conn, key, value, now())?;
        Ok(())
    }

    // ---- 통계·달성 ------------------------------------------------------

    pub fn statistics(&self) -> Result<Statistics, StoreError> {
        let (sessions, play_seconds): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(MAX(ended_at - started_at, 0)), 0)
               FROM game_sessions WHERE games > 0 OR ended_at IS NULL",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let stats = self.conn.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN replay_game = 0 THEN bet ELSE 0 END), 0),
                    COALESCE(SUM(payout), 0),
                    COALESCE(SUM(replay_awarded), 0),
                    COALESCE(SUM(bonus_started = 'big'), 0),
                    COALESCE(SUM(bonus_started = 'regular'), 0),
                    COALESCE(MAX(payout), 0)
               FROM game_history",
            [],
            |row| {
                Ok(Statistics {
                    sessions: sessions as u64,
                    play_seconds: play_seconds as u64,
                    games: row.get::<_, i64>(0)? as u64,
                    total_bet: row.get::<_, i64>(1)? as u64,
                    total_payout: row.get::<_, i64>(2)? as u64,
                    replays: row.get::<_, i64>(3)? as u64,
                    big_bonuses: row.get::<_, i64>(4)? as u64,
                    regular_bonuses: row.get::<_, i64>(5)? as u64,
                    max_payout: row.get::<_, i64>(6)? as u64,
                })
            },
        )?;
        Ok(stats)
    }

    /// 달성한 기록 목록 (달성 순).
    pub fn achievements(&self) -> Result<Vec<Achievement>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT id FROM achievements ORDER BY unlocked_at, id")?;
        let keys = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys
            .iter()
            .filter_map(|key| Achievement::from_key(key))
            .collect())
    }

    /// 플레이 기록·통계·달성·보유 메달을 모두 지운다. 사용자 설정은 유지한다.
    pub fn reset_history(&mut self) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DELETE FROM reel_results;
             DELETE FROM credit_history;
             DELETE FROM game_history;
             DELETE FROM game_sessions;
             DELETE FROM achievements;",
        )?;
        tx.execute(
            "DELETE FROM user_settings WHERE key = ?1",
            params![CLASSIC_BALANCE_KEY],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn read_setting(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT value FROM user_settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
}

fn write_setting(
    conn: &Connection,
    key: &str,
    value: &str,
    timestamp: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO user_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, timestamp],
    )?;
    Ok(())
}

fn insert_credit(
    tx: &Transaction<'_>,
    session_id: i64,
    entry: &LedgerEntry,
    timestamp: i64,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO credit_history (session_id, game_id, kind, amount, balance_after, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session_id,
            entry.game_id.map(|id| id as i64),
            entry.kind.key(),
            entry.amount,
            entry.balance_after,
            timestamp,
        ],
    )?;
    Ok(())
}

fn unlock_achievements(
    tx: &Transaction<'_>,
    outcome: &GameOutcome,
    timestamp: i64,
) -> Result<Vec<Achievement>, rusqlite::Error> {
    let games: i64 = tx.query_row("SELECT COUNT(*) FROM game_history", [], |row| row.get(0))?;
    let mut candidates = Vec::new();
    if outcome.payout > 0 {
        candidates.push(Achievement::FirstWin);
    }
    if outcome.replay_awarded {
        candidates.push(Achievement::FirstReplay);
    }
    match outcome.bonus_started {
        Some(BonusKind::Big) => candidates.push(Achievement::FirstBigBonus),
        Some(BonusKind::Regular) => candidates.push(Achievement::FirstRegularBonus),
        None => {}
    }
    if outcome.wins.iter().any(|win| win.role == Role::Star) {
        candidates.push(Achievement::StarTrio);
    }
    if games >= 100 {
        candidates.push(Achievement::Games100);
    }
    if games >= 1000 {
        candidates.push(Achievement::Games1000);
    }

    let mut unlocked = Vec::new();
    for achievement in candidates {
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO achievements (id, unlocked_at, game_id) VALUES (?1, ?2, ?3)",
            params![achievement.key(), timestamp, outcome.game_id as i64],
        )?;
        if inserted > 0 {
            unlocked.push(achievement);
        }
    }
    Ok(unlocked)
}
