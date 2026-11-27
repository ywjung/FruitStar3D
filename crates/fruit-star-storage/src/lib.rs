//! FRUIT STAR 3D 로컬 저장소 (SQLite).
//!
//! 게임 결과와 가상 메달 원장 변경을 같은 트랜잭션으로 기록한다. 통계는 플레이
//! 기록 열람과 검증을 위한 것이며 당첨 예측이나 금전 관련 기능은 없다.

mod schema;
mod store;

pub use store::{
    Achievement, RecoveryReport, Statistics, Store, StoreError, default_database_path,
};
