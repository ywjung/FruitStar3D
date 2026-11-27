//! Bookkeeping of generated files and the `assets/ASSET_SOURCES.md` provenance table.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

/// Category of a generated file (decides the section in the provenance document).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Symbol,
    Texture,
    Audio,
    Concept,
}

/// One generated file.
#[derive(Debug, Clone)]
pub struct Entry {
    pub kind: Kind,
    /// Path relative to the workspace root, with forward slashes.
    pub path: String,
    /// Pixel size or duration, e.g. `1024×512 RGBA PNG`.
    pub format: String,
    /// Source module inside `crates/fruit-star-assetgen/src`.
    pub module: &'static str,
    /// What the file contains.
    pub content: String,
}

const ORIGIN: &str = "이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음)";

fn section(out: &mut String, title: &str, entries: &[Entry], kind: Kind) {
    let _ = writeln!(out, "## {title}\n");
    let _ = writeln!(out, "| 파일 | 형식 | 생성 모듈 | 내용 | 출처 |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for e in entries.iter().filter(|e| e.kind == kind) {
        let _ = writeln!(
            out,
            "| `{}` | {} | `{}` | {} | {ORIGIN} |",
            e.path, e.format, e.module, e.content
        );
    }
    out.push('\n');
}

/// Renders the Korean provenance document.
pub fn render(entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str(
        "# ASSET_SOURCES — 에셋 출처 목록\n\n\
         > 이 문서는 `fruit-star-assetgen` 이 자동으로 생성합니다. 직접 수정하지 말고 \
         워크스페이스 루트에서 `cargo run -p fruit-star-assetgen --release` 를 다시 실행하세요.\n\n\
         ## 요약\n\n\
         - 아래 표의 모든 파일은 `crates/fruit-star-assetgen` 의 Rust 코드가 **절차적으로 생성한 \
         독자적(original) 저작물**이며, FRUIT STAR 3D 프로젝트를 위해 새로 만들었습니다.\n\
         - 제3자의 그림, 사진, 음원, 샘플, 폰트, 로고, 상표, 캐릭터, 기존 슬롯머신 아트는 \
         **전혀 사용하지 않았습니다.** 인터넷에서 내려받은 자료도 없습니다.\n\
         - 글자는 시스템 폰트가 아니라 도구에 내장된 자체 제작 stroke font \
         (`src/font.rs`, \"Fruit Star Stroke\")로 그립니다. `resvg` 의 text / system-fonts 기능은 \
         빌드에서 꺼 두었습니다.\n\
         - 그림은 코드가 만든 SVG(`design/concept/`)를 `resvg` 로 rasterize 한 PNG 이고, \
         효과음은 oscillator · filtered noise · envelope 만으로 합성한 WAV 입니다. \
         멜로디(`bonus_start`, `bonus_end` 등)는 이 프로젝트를 위해 새로 지은 것입니다.\n\
         - 난수는 고정 seed 의 xorshift(`src/rng.rs`)만 사용하므로 같은 toolchain 에서 다시 \
         실행하면 동일한 결과가 나옵니다.\n\
         - 라이선스: 프로젝트 본체와 동일한 라이선스(워크스페이스 `Cargo.toml` 의 `license` \
         항목)를 따릅니다.\n\n",
    );
    section(
        &mut out,
        "릴 심볼 (`assets/symbols/`)",
        entries,
        Kind::Symbol,
    );
    section(
        &mut out,
        "캐비닛 텍스처 (`assets/textures/`)",
        entries,
        Kind::Texture,
    );
    section(&mut out, "효과음 (`assets/audio/`)", entries, Kind::Audio);
    section(
        &mut out,
        "디자인 원본 · 참고 자료 (`design/concept/`)",
        entries,
        Kind::Concept,
    );
    out.push_str(
        "## 재생성 방법\n\n\
         ```sh\n\
         cargo run -p fruit-star-assetgen --release            # 워크스페이스 루트에서 실행\n\
         cargo run -p fruit-star-assetgen --release -- --root <path>\n\
         ```\n\n\
         생성 직후 도구가 모든 WAV 의 sample rate / channel / 길이 / peak / loop 연속성과 \
         심볼 PNG 의 여백(잘림 여부)을 자동으로 검사하며, 같은 검사가 \
         `cargo test -p fruit-star-assetgen` 에도 들어 있습니다.\n",
    );
    out
}

/// Writes `assets/ASSET_SOURCES.md` under `root`.
pub fn write(root: &Path, entries: &[Entry]) -> Result<()> {
    let path = root.join("assets/ASSET_SOURCES.md");
    std::fs::write(&path, render(entries)).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_lists_every_entry_once() {
        let entries = vec![
            Entry {
                kind: Kind::Symbol,
                path: "assets/symbols/star.png".into(),
                format: "1024×512 RGBA PNG".into(),
                module: "symbols.rs",
                content: "star".into(),
            },
            Entry {
                kind: Kind::Audio,
                path: "assets/audio/reel_stop.wav".into(),
                format: "0.15 s".into(),
                module: "audio.rs",
                content: "thunk".into(),
            },
        ];
        let doc = render(&entries);
        assert_eq!(doc.matches("assets/symbols/star.png").count(), 1);
        assert_eq!(doc.matches("assets/audio/reel_stop.wav").count(), 1);
        assert!(doc.contains("제3자 자료 없음"));
    }
}
