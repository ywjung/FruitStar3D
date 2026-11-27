//! 릴에 인쇄되는 심볼.

use std::fmt;

/// 릴 심볼 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    Grape,
    Cherry,
    RedSeven,
    BlueSeven,
    Bar,
    Bell,
    Melon,
    Star,
    Replay,
}

impl Symbol {
    /// 모든 심볼 (정의 순서).
    pub const ALL: [Symbol; 9] = [
        Symbol::Grape,
        Symbol::Cherry,
        Symbol::RedSeven,
        Symbol::BlueSeven,
        Symbol::Bar,
        Symbol::Bell,
        Symbol::Melon,
        Symbol::Star,
        Symbol::Replay,
    ];

    /// 설정 파일과 리소스 파일명에 쓰는 snake_case 키.
    pub const fn key(self) -> &'static str {
        match self {
            Symbol::Grape => "grape",
            Symbol::Cherry => "cherry",
            Symbol::RedSeven => "red_seven",
            Symbol::BlueSeven => "blue_seven",
            Symbol::Bar => "bar",
            Symbol::Bell => "bell",
            Symbol::Melon => "melon",
            Symbol::Star => "star",
            Symbol::Replay => "replay",
        }
    }

    /// [`Symbol::key`]의 역변환.
    pub fn from_key(key: &str) -> Option<Symbol> {
        Symbol::ALL.into_iter().find(|s| s.key() == key)
    }

    /// `Symbol::ALL` 안에서의 위치.
    pub const fn ordinal(self) -> usize {
        self as usize
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}
