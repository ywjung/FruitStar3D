//! 그림 리소스 로딩과 가공.

use bevy::asset::LoadState;
use bevy::prelude::*;
use fruit_star_config::missing_assets;
use fruit_star_core::{REEL_COUNT, Symbol};
use fruit_star_machine::layout::{REEL_RADIUS, REEL_WIDTH};
use fruit_star_machine::{AppState, FailureMessage, Machine, Settings};

use crate::image_ops::{Pixels, Wrap, compose_strip, textured_image};
use crate::reel_mesh::cell_aspect;

/// 캐비닛 라벨 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Label {
    Credit,
    Payout,
    Count,
    StarChance,
    MaxBet,
    Start,
    Stop,
    Insert,
    OneBet,
    Replay,
    Games,
    Big,
    Reg,
    Total,
}

impl Label {
    pub const ALL: [Label; 14] = [
        Label::Credit,
        Label::Payout,
        Label::Count,
        Label::StarChance,
        Label::MaxBet,
        Label::Start,
        Label::Stop,
        Label::Insert,
        Label::OneBet,
        Label::Replay,
        Label::Games,
        Label::Big,
        Label::Reg,
        Label::Total,
    ];

    const fn file(self) -> &'static str {
        match self {
            Label::Credit => "credit",
            Label::Payout => "payout",
            Label::Count => "count",
            Label::StarChance => "star_chance",
            Label::MaxBet => "max_bet",
            Label::Start => "start",
            Label::Stop => "stop",
            Label::Insert => "insert",
            Label::OneBet => "one_bet",
            Label::Replay => "replay",
            Label::Games => "games",
            Label::Big => "big",
            Label::Reg => "reg",
            Label::Total => "total",
        }
    }
}

/// 로딩 중인 원본 이미지 핸들.
#[derive(Resource)]
pub(crate) struct PendingArt {
    symbols: Vec<(Symbol, Handle<Image>)>,
    band: Handle<Image>,
    top_panel: Handle<Image>,
    belly_panel: Handle<Image>,
    star_lamp: Handle<Image>,
    medal_face: Handle<Image>,
    paytable: Handle<Image>,
    labels: Vec<(Label, Handle<Image>)>,
}

impl PendingArt {
    fn all(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.symbols
            .iter()
            .map(|(_, h)| h)
            .chain(self.labels.iter().map(|(_, h)| h))
            .chain([
                &self.band,
                &self.top_panel,
                &self.belly_panel,
                &self.star_lamp,
                &self.medal_face,
                &self.paytable,
            ])
    }
}

/// 가공이 끝난 텍스처.
#[derive(Resource, Debug, Clone)]
pub struct ArtAssets {
    /// 릴별 띠 텍스처.
    pub strips: [Handle<Image>; REEL_COUNT],
    pub top_panel: Handle<Image>,
    pub belly_panel: Handle<Image>,
    pub star_lamp: Handle<Image>,
    pub medal_face: Handle<Image>,
    /// 릴 패널에 인쇄되는 배당표.
    pub paytable: Handle<Image>,
    /// (라벨, 텍스처, 너비÷높이).
    pub labels: Vec<(Label, Handle<Image>, f32)>,
}

impl ArtAssets {
    pub fn label(&self, label: Label) -> Option<(Handle<Image>, f32)> {
        self.labels
            .iter()
            .find(|(l, _, _)| *l == label)
            .map(|(_, handle, aspect)| (handle.clone(), *aspect))
    }
}

pub(crate) fn begin_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    mut failure: ResMut<FailureMessage>,
    mut next: ResMut<NextState<AppState>>,
) {
    let missing = missing_assets(&settings.root.join("assets"));
    if !missing.is_empty() {
        failure.0 = format!(
            "필수 리소스 {}개가 없습니다: {}\n`cargo run -p fruit-star-assetgen --release`로 다시 생성할 수 있습니다.",
            missing.len(),
            missing.join(", ")
        );
        error!("{}", failure.0);
        next.set(AppState::Failed);
        return;
    }
    commands.insert_resource(PendingArt {
        symbols: Symbol::ALL
            .into_iter()
            .map(|s| (s, asset_server.load(format!("symbols/{}.png", s.key()))))
            .collect(),
        band: asset_server.load("textures/reel_band.png"),
        top_panel: asset_server.load("textures/top_panel.png"),
        belly_panel: asset_server.load("textures/belly_panel.png"),
        star_lamp: asset_server.load("textures/star_lamp.png"),
        medal_face: asset_server.load("textures/medal_face.png"),
        paytable: asset_server.load("textures/paytable.png"),
        labels: Label::ALL
            .into_iter()
            .map(|l| {
                (
                    l,
                    asset_server.load(format!("textures/labels/{}.png", l.file())),
                )
            })
            .collect(),
    });
}

/// 모든 이미지가 로딩되면 릴 띠를 합성하고 밉맵을 붙인 뒤 메뉴로 넘어간다.
pub(crate) fn finish_loading(
    mut commands: Commands,
    pending: Option<Res<PendingArt>>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    machine: Res<Machine>,
    settings: Res<Settings>,
    mut failure: ResMut<FailureMessage>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(pending) = pending else {
        return;
    };
    for handle in pending.all() {
        match asset_server.load_state(handle) {
            LoadState::Failed(error) => {
                failure.0 = format!("리소스를 읽을 수 없습니다: {error}");
                error!("{}", failure.0);
                commands.remove_resource::<PendingArt>();
                next.set(AppState::Failed);
                return;
            }
            LoadState::Loaded => {}
            _ => return,
        }
    }

    let started = std::time::Instant::now();
    let read = |images: &Assets<Image>, handle: &Handle<Image>| {
        images.get(handle).and_then(Pixels::from_image)
    };
    let fail = |what: &str| format!("이미지 '{what}'의 픽셀 형식을 처리할 수 없습니다.");

    let result = (|| -> Result<ArtAssets, String> {
        let profile = machine.profile();
        let band = read(&images, &pending.band).ok_or_else(|| fail("reel_band"))?;
        let mut symbol_pixels = Vec::with_capacity(pending.symbols.len());
        for (symbol, handle) in &pending.symbols {
            symbol_pixels.push((
                *symbol,
                read(&images, handle).ok_or_else(|| fail(symbol.key()))?,
            ));
        }
        let cell_width = settings
            .config
            .graphics
            .render
            .quality
            .symbol_texture_width();
        let aspect = cell_aspect(REEL_RADIUS, REEL_WIDTH, profile.symbol_count);
        let cell_height = (cell_width as f32 / aspect).round().max(1.0) as u32;

        let mut strips = Vec::with_capacity(REEL_COUNT);
        for strip in &profile.strips {
            let mut cells = Vec::with_capacity(strip.len());
            for symbol in strip {
                let art = symbol_pixels
                    .iter()
                    .find(|(s, _)| s == symbol)
                    .map(|(_, pixels)| pixels)
                    .ok_or_else(|| fail(symbol.key()))?;
                cells.push(art);
            }
            let composed = compose_strip(&cells, &band, cell_width, cell_height);
            strips.push(textured_image(composed, Wrap::RepeatV));
        }

        let single = |handle: &Handle<Image>, name: &str| {
            read(&images, handle)
                .map(|pixels| textured_image(pixels, Wrap::Clamp))
                .ok_or_else(|| fail(name))
        };
        let top_panel = single(&pending.top_panel, "top_panel")?;
        let belly_panel = single(&pending.belly_panel, "belly_panel")?;
        let star_lamp = single(&pending.star_lamp, "star_lamp")?;
        let medal_face = single(&pending.medal_face, "medal_face")?;
        let paytable = single(&pending.paytable, "paytable")?;
        let mut labels = Vec::with_capacity(pending.labels.len());
        for (label, handle) in &pending.labels {
            let image = single(handle, label.file())?;
            let aspect = image.width() as f32 / image.height().max(1) as f32;
            labels.push((*label, image, aspect));
        }

        let mut strips = strips.into_iter().map(|image| images.add(image));
        let (Some(left), Some(center), Some(right)) = (strips.next(), strips.next(), strips.next())
        else {
            return Err("릴 띠 텍스처를 만들지 못했습니다.".to_owned());
        };
        Ok(ArtAssets {
            strips: [left, center, right],
            top_panel: images.add(top_panel),
            belly_panel: images.add(belly_panel),
            star_lamp: images.add(star_lamp),
            medal_face: images.add(medal_face),
            paytable: images.add(paytable),
            labels: labels
                .into_iter()
                .map(|(label, image, aspect)| (label, images.add(image), aspect))
                .collect(),
        })
    })();

    // 원본 이미지는 더 필요 없다 (핸들을 버리면 해제된다).
    commands.remove_resource::<PendingArt>();
    match result {
        Ok(art) => {
            info!(
                "텍스처 가공 완료: {:.0} ms",
                started.elapsed().as_secs_f64() * 1000.0
            );
            commands.insert_resource(art);
            // 장면을 만들고 렌더 파이프라인을 미리 컴파일한 뒤 메뉴로 넘어간다 (warmup.rs).
            commands.insert_resource(crate::warmup::Warmup::default());
        }
        Err(message) => {
            error!("{message}");
            failure.0 = message;
            next.set(AppState::Failed);
        }
    }
}
