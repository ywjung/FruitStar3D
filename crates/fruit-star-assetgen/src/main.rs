//! `fruit-star-assetgen` — regenerates every original 2D art and sound asset of
//! FRUIT STAR 3D from code, deterministically.
//!
//! Run from the workspace root (or pass `--root <path>`):
//!
//! ```sh
//! cargo run -p fruit-star-assetgen --release
//! ```

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod audio;
mod font;
mod icon;
mod manifest;
mod panels;
mod raster;
mod rng;
mod svg;
mod symbols;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use manifest::{Entry, Kind};
use raster::Image;

const USAGE: &str = "usage: fruit-star-assetgen [--root <workspace root>]";

fn parse_args() -> Result<Option<PathBuf>> {
    let mut root = PathBuf::from(".");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let Some(value) = args.next() else {
                    bail!("--root needs a path\n{USAGE}");
                };
                root = PathBuf::from(value);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(None);
            }
            other => bail!("unexpected argument {other:?}\n{USAGE}"),
        }
    }
    Ok(Some(root))
}

fn write_text(root: &Path, rel: &str, text: &str) -> Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

fn symbol_description(name: &str) -> &'static str {
    match name {
        "grape" => "보라색 포도송이 + 잎",
        "cherry" => "쌍둥이 체리 + 줄기와 잎",
        "red_seven" => "빨간 7 (흰 inner outline, gold drop edge)",
        "blue_seven" => "파란 7 (silver edge, 별 cut-out 과 stripe)",
        "bar" => "검은 plate + gold 테두리 + stroke font \"BAR\"",
        "bell" => "황금 종 + 빨간 리본",
        "melon" => "줄무늬 수박 + 빨간 조각",
        "star" => "bevel 음영의 gold 별",
        "replay" => "teal 리본 + stroke font \"REPLAY\" + 원형 화살표",
        _ => "심볼",
    }
}

fn generate_symbols(root: &Path, entries: &mut Vec<Entry>) -> Result<()> {
    let (w, h) = (symbols::WIDTH, symbols::HEIGHT);
    // Contact sheet on a reel-coloured background, with a small-size row to judge legibility.
    let mut sheet = Image::solid(1536, 768 + 112, [245, 240, 226]);
    for (i, name) in symbols::NAMES.iter().enumerate() {
        let art = symbols::build(name)?;
        let svg_text = symbols::document(name, &art);
        let svg_rel = format!("design/concept/symbols/{name}.svg");
        write_text(root, &svg_rel, &svg_text)?;

        let mut img = raster::render_svg(&svg_text, w, h)?;
        let Some((x0, y0, x1, y1)) = img.alpha_bounds(8) else {
            bail!("symbol {name} rendered empty");
        };
        ensure!(
            x0 >= 8 && y0 >= 8 && x1 + 8 < w && y1 + 8 < h,
            "symbol {name} touches the canvas edge ({x0},{y0})-({x1},{y1})"
        );
        ensure!(
            img.pixel(0, 0)[3] == 0 && img.pixel(w - 1, h - 1)[3] == 0,
            "symbol {name} has no transparent background"
        );

        let i = i as u32;
        sheet.composite(&img.downscale(2), (i % 3) * 512, (i / 3) * 256);
        sheet.composite(&img.downscale(8), 20 + i * 168, 768 + 24);

        img.bleed_edges(24);
        let png_rel = format!("assets/symbols/{name}.png");
        img.save_png(&root.join(&png_rel))?;
        println!(
            "  {png_rel}  {w}x{h}  art bounds x {x0}..{x1}, y {y0}..{y1} ({:.0}% of height)",
            (y1 - y0 + 1) as f32 * 100.0 / h as f32
        );

        entries.push(Entry {
            kind: Kind::Symbol,
            path: png_rel,
            format: format!("{w}×{h} RGBA PNG (투명 배경)"),
            module: "symbols.rs",
            content: symbol_description(name).into(),
        });
        entries.push(Entry {
            kind: Kind::Concept,
            path: svg_rel,
            format: format!("SVG, viewBox 0 0 {w} {h}"),
            module: "symbols.rs",
            content: format!("`{name}` 심볼의 vector 원본"),
        });
    }
    let sheet_rel = "design/concept/symbols_sheet.png";
    sheet.save_png(&root.join(sheet_rel))?;
    entries.push(Entry {
        kind: Kind::Concept,
        path: sheet_rel.into(),
        format: format!("{}×{} RGB PNG", sheet.width, sheet.height),
        module: "main.rs",
        content: "릴 배경색 위에 9개 심볼을 모아 본 contact sheet (가독성 확인용)".into(),
    });
    Ok(())
}

/// Bookkeeping for one texture written by [`generate_textures`].
struct TextureJob<'a> {
    stem: &'a str,
    svg_text: String,
    image: Image,
    alpha: &'a str,
    module_note: &'a str,
    content: String,
}

fn emit_texture(root: &Path, entries: &mut Vec<Entry>, job: TextureJob<'_>) -> Result<()> {
    let png_rel = format!("assets/textures/{}.png", job.stem);
    let svg_rel = format!("design/concept/panels/{}.svg", job.stem);
    write_text(root, &svg_rel, &job.svg_text)?;
    job.image.save_png(&root.join(&png_rel))?;
    let (w, h) = (job.image.width, job.image.height);
    println!("  {png_rel}  {w}x{h}");
    entries.push(Entry {
        kind: Kind::Texture,
        path: png_rel,
        format: format!("{w}×{h} PNG ({})", job.alpha),
        module: "panels.rs",
        content: job.content.clone(),
    });
    entries.push(Entry {
        kind: Kind::Concept,
        path: svg_rel,
        format: format!("SVG, viewBox 0 0 {w} {h}"),
        module: "panels.rs",
        content: format!("`{}.png` 의 vector 원본{}", job.stem, job.module_note),
    });
    Ok(())
}

fn generate_textures(root: &Path, entries: &mut Vec<Entry>) -> Result<()> {
    let svg_text = panels::menu_marquee_svg()?;
    let image = raster::render_svg(&svg_text, 960, 336)?;
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "menu_marquee",
            svg_text,
            image,
            alpha: "불투명, sRGB",
            module_note: "",
            content: "메뉴·로딩 간판에 쓰는 FRUIT STAR 3D 레터링과 체리".into(),
        },
    )?;

    let svg_text = panels::top_panel_svg()?;
    let mut image = raster::render_svg(&svg_text, 2048, 640)?;
    image.flatten([0, 0, 0]);
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "top_panel",
            svg_text,
            image,
            alpha: "불투명, sRGB",
            module_note: "",
            content:
                "상단 backlit marquee: stroke font \"FRUIT STAR\" 타이틀, 별, 과일 (emissive 용)"
                    .into(),
        },
    )?;

    let svg_text = panels::belly_panel_svg()?;
    let mut image = raster::render_svg(&svg_text, 2048, 900)?;
    image.flatten([0, 0, 0]);
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "belly_panel",
            svg_text,
            image,
            alpha: "불투명, sRGB",
            module_note: "",
            content: "하단 장식 패널: sunburst, 큰 과일 그림, \"FRUIT CLASSIC\" 리본".into(),
        },
    )?;

    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "reel_band",
            svg_text: panels::reel_band_svg(),
            image: panels::reel_band_image(),
            alpha: "불투명, sRGB, tileable",
            module_note: " (종이 질감 noise 는 raster 단계에서 추가)",
            content: "릴 띠 배경: cream 색 + 미세한 noise + 좌우 가장자리 선 (세로 반복)".into(),
        },
    )?;

    for (stem, text) in panels::LABELS {
        let (svg_text, width) = panels::label_svg(text)?;
        let mut image = raster::render_svg(&svg_text, width, panels::LABEL_HEIGHT)?;
        // Pure white everywhere: the engine tints the label, alpha carries the shape.
        image.force_rgb([255, 255, 255]);
        emit_texture(
            root,
            entries,
            TextureJob {
                stem: &format!("labels/{stem}"),
                svg_text,
                image,
                alpha: "RGBA, 투명 배경",
                module_note: "",
                content: format!("흰색 stroke font 라벨 \"{text}\""),
            },
        )?;
    }

    let svg_text = panels::paytable_svg()?;
    let (width, height) = panels::PAYTABLE_SIZE;
    let image = panels::render_cutout(&svg_text, width, height)?;
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "paytable",
            svg_text,
            image,
            alpha: "RGBA, 투명 배경",
            module_note: "",
            content: "릴 패널에 인쇄되는 배당표: 심볼 9종과 세 개가 늘어섰을 때의 배당".into(),
        },
    )?;

    let svg_text = panels::star_lamp_svg();
    let image = panels::render_cutout(&svg_text, 512, 512)?;
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "star_lamp",
            svg_text,
            image,
            alpha: "RGBA, 투명 배경",
            module_note: "",
            content: "STAR CHANCE 램프 렌즈용 흰색 별 (부드러운 gradient)".into(),
        },
    )?;

    let svg_text = panels::medal_face_svg();
    let image = panels::render_cutout(&svg_text, 512, 512)?;
    emit_texture(
        root,
        entries,
        TextureJob {
            stem: "medal_face",
            svg_text,
            image,
            alpha: "RGBA, 투명 배경",
            module_note: "",
            content:
                "게임 메달 앞면: brass 원판, 돋은 테두리, 점 장식, 양각 별 (실제 주화 모방 없음)"
                    .into(),
        },
    )?;
    Ok(())
}

fn generate_audio(root: &Path, entries: &mut Vec<Entry>) -> Result<()> {
    let dir = root.join("assets/audio");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    for spec in &audio::SPECS {
        let rel = format!("assets/audio/{}.wav", spec.name);
        let path = root.join(&rel);
        let bytes = audio::wav_bytes(&audio::render(spec));
        std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;

        // Read the file back and sanity-check what actually landed on disk.
        let on_disk =
            std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let report = audio::verify_bytes(spec, &on_disk)?;
        println!(
            "  {rel}  {:.3} s  peak {:.1} dBFS  rms {:.1} dBFS{}",
            report.seconds,
            report.peak_db,
            report.rms_db,
            if spec.looped { "  (loop, seam ok)" } else { "" }
        );
        entries.push(Entry {
            kind: Kind::Audio,
            path: rel,
            format: format!(
                "{:.2} s, 44.1 kHz 16-bit mono WAV, peak {:.0} dBFS{}",
                report.seconds,
                spec.peak_db,
                if spec.looped { ", seamless loop" } else { "" }
            ),
            module: "audio.rs",
            content: spec.summary.into(),
        });
    }
    Ok(())
}

/// Writes the application icon: vector source, a 1024 px master, and the `.iconset`
/// directory that `iconutil` turns into `AppIcon.icns` when packaging.
fn generate_icon(root: &Path, entries: &mut Vec<Entry>) -> Result<()> {
    let svg_text = icon::app_icon_svg()?;
    let svg_rel = "design/concept/icon/app_icon.svg";
    write_text(root, svg_rel, &svg_text)?;
    entries.push(Entry {
        kind: Kind::Concept,
        path: svg_rel.into(),
        format: format!("SVG, viewBox 0 0 {0} {0}", icon::CANVAS),
        module: "icon.rs",
        content: "앱 아이콘의 vector 원본".into(),
    });

    // iPhone: 불투명한 1024px 정사각형 하나. 패키징 때 actool이 필요한 크기를 만든다.
    let ios_svg = icon::app_icon_ios_svg()?;
    write_text(root, "design/concept/icon/app_icon_ios.svg", &ios_svg)?;
    let ios_rel = "assets/icons/ios/AppIcon-1024.png";
    let mut ios_icon = raster::render_svg(&ios_svg, icon::CANVAS, icon::CANVAS)?;
    ios_icon.flatten([0x5e, 0x04, 0x10]);
    if let Some(parent) = root.join(ios_rel).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    ios_icon.save_png(&root.join(ios_rel))?;
    println!("  {ios_rel}  {0}x{0}", icon::CANVAS);
    entries.push(Entry {
        kind: Kind::Concept,
        path: "design/concept/icon/app_icon_ios.svg".into(),
        format: format!("SVG, viewBox 0 0 {0} {0}", icon::CANVAS),
        module: "icon.rs",
        content: "iPhone 앱 아이콘의 vector 원본".into(),
    });
    entries.push(Entry {
        kind: Kind::Texture,
        path: ios_rel.into(),
        format: format!("{0}×{0} PNG (불투명)", icon::CANVAS),
        module: "icon.rs",
        content: "iPhone 앱 아이콘: 모서리를 깎지 않은 정사각형 (iOS가 둥근 모서리를 씌운다)"
            .into(),
    });

    let master_rel = "assets/icons/app_icon.png";
    let master = raster::render_svg(&svg_text, icon::CANVAS, icon::CANVAS)?;
    if let Some(parent) = root.join(master_rel).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    master.save_png(&root.join(master_rel))?;
    println!("  {master_rel}  {0}x{0}", icon::CANVAS);
    entries.push(Entry {
        kind: Kind::Texture,
        path: master_rel.into(),
        format: format!("{0}×{0} PNG (투명 배경)", icon::CANVAS),
        module: "icon.rs",
        content: "앱 아이콘: 빨간 타일 위의 금색 별과 빨간 7".into(),
    });

    let iconset = root.join("assets/icons/AppIcon.iconset");
    std::fs::create_dir_all(&iconset).with_context(|| format!("creating {}", iconset.display()))?;
    for (name, size) in icon::ICONSET {
        // Each size is rendered from the vector source rather than scaled from the master.
        raster::render_svg(&svg_text, size, size)?.save_png(&iconset.join(name))?;
    }
    println!(
        "  assets/icons/AppIcon.iconset  {} files",
        icon::ICONSET.len()
    );
    entries.push(Entry {
        kind: Kind::Texture,
        path: "assets/icons/AppIcon.iconset/".into(),
        format: format!("PNG {}개 (16~1024 px)", icon::ICONSET.len()),
        module: "icon.rs",
        content: "macOS iconset. 패키징 때 `iconutil`로 `AppIcon.icns`를 만든다".into(),
    });
    Ok(())
}

fn main() -> Result<()> {
    let Some(root) = parse_args()? else {
        return Ok(());
    };
    ensure!(
        root.join("Cargo.toml").is_file() && root.join("crates").is_dir(),
        "{} does not look like the workspace root (no Cargo.toml + crates/); use --root",
        root.display()
    );

    let mut entries = Vec::new();
    println!("symbols:");
    generate_symbols(&root, &mut entries)?;
    println!("textures:");
    generate_textures(&root, &mut entries)?;
    println!("icon:");
    generate_icon(&root, &mut entries)?;
    println!("audio:");
    generate_audio(&root, &mut entries)?;
    manifest::write(&root, &entries)?;
    println!(
        "wrote assets/ASSET_SOURCES.md ({} files listed)",
        entries.len()
    );
    Ok(())
}
