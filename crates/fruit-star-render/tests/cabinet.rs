#![allow(clippy::unwrap_used, clippy::expect_used)]

//! 캐비닛 모델의 헤드리스 검사: 필수 부품이 빠짐없이 만들어지는지 확인한다.

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use fruit_star_config::AnimationSection;
use fruit_star_machine::{
    Clickable, DisplayKind, Lamp, LampKind, MachineCommand, PartKind, Pressable, SevenSegment,
};
use fruit_star_render::{
    ArtAssets, CabinetRoot, CabinetSpec, MaterialLibrary, REQUIRED_PARTS, ReelDrum, ShellPart,
    spawn_cabinet,
};

fn build(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let art = ArtAssets {
        strips: [Handle::default(), Handle::default(), Handle::default()],
        top_panel: Handle::default(),
        belly_panel: Handle::default(),
        star_lamp: Handle::default(),
        medal_face: Handle::default(),
        paytable: Handle::default(),
        labels: fruit_star_render::Label::ALL
            .into_iter()
            .map(|label| (label, Handle::default(), 3.0))
            .collect(),
    };
    let library = MaterialLibrary::new(&mut materials, &art);
    let spec = CabinetSpec {
        library: &library,
        art: &art,
        symbol_count: 21,
        reel_segments: 84,
        animation: &AnimationSection::default(),
    };
    spawn_cabinet(&mut commands, &mut meshes, &mut materials, &spec);
}

fn world() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Image>()
        .add_systems(Startup, build);
    app.update();
    app
}

#[test]
fn all_required_parts_exist_exactly_once() {
    let mut app = world();
    let mut query = app.world_mut().query::<&Name>();
    let names: Vec<String> = query.iter(app.world()).map(|n| n.to_string()).collect();
    for part in REQUIRED_PARTS {
        let count = names.iter().filter(|n| n.as_str() == part).count();
        assert_eq!(count, 1, "{part}: {count}개");
    }
    let mut roots = app.world_mut().query::<&CabinetRoot>();
    assert_eq!(roots.iter(app.world()).count(), 1);
}

#[test]
fn reels_are_independent_drums_at_their_layout_positions() {
    let mut app = world();
    let mut query = app.world_mut().query::<(&ReelDrum, &Transform, &Name)>();
    let mut drums: Vec<(usize, Vec3, String)> = query
        .iter(app.world())
        .map(|(drum, transform, name)| (drum.0, transform.translation, name.to_string()))
        .collect();
    drums.sort_by_key(|(index, _, _)| *index);
    assert_eq!(drums.len(), 3);
    for (index, (reel, position, name)) in drums.iter().enumerate() {
        assert_eq!(*reel, index);
        assert_eq!(*position, fruit_star_machine::layout::reel_center(index));
        assert_eq!(name, ["ReelLeft", "ReelCenter", "ReelRight"][index]);
    }
}

#[test]
fn end_plates_rotate_with_each_reel_and_the_axle_stays_on_the_frame() {
    let mut app = world();
    let mut parts = app.world_mut().query::<(&Name, &ChildOf, &Transform)>();
    let mut plate_offsets = [Vec::new(), Vec::new(), Vec::new()];
    let mut axles = 0;
    for (name, parent, transform) in parts.iter(app.world()) {
        match name.as_str() {
            "ReelFlange" => {
                let drum = app
                    .world()
                    .get::<ReelDrum>(parent.parent())
                    .expect("릴 끝판의 피벗");
                plate_offsets[drum.0].push(transform.translation.x);
            }
            "ReelAxle" => {
                let mount = app
                    .world()
                    .get::<Name>(parent.parent())
                    .expect("고정 프레임");
                assert_eq!(mount.as_str(), "ReelUnit");
                axles += 1;
            }
            _ => {}
        }
    }
    assert_eq!(axles, 1);
    for mut offsets in plate_offsets {
        offsets.sort_by(f32::total_cmp);
        assert_eq!(offsets.len(), 2);
        assert!(offsets[0] < 0.0 && offsets[0] == -offsets[1]);
    }
}

#[test]
fn controls_displays_and_lamps_are_wired() {
    let mut app = world();

    // 눌리는 부품: 레버, MAX BET, 정지 버튼 3개, 메달 투입구
    let mut pressables = app.world_mut().query::<&Pressable>();
    let mut kinds: Vec<PartKind> = pressables.iter(app.world()).map(|p| p.kind).collect();
    kinds.sort_by_key(|kind| format!("{kind:?}"));
    assert_eq!(
        kinds,
        vec![
            PartKind::Bet,
            PartKind::BetOne,
            PartKind::Lever,
            PartKind::MedalSlot,
            PartKind::Stop(0),
            PartKind::Stop(1),
            PartKind::Stop(2),
        ]
    );

    // 마우스로 누를 수 있는 부품: 모든 조작에 대해 하나 이상
    let mut clickables = app.world_mut().query::<&Clickable>();
    let commands: Vec<MachineCommand> = clickables.iter(app.world()).map(|c| c.0).collect();
    for command in [
        MachineCommand::Bet,
        MachineCommand::BetOne,
        MachineCommand::Lever,
        MachineCommand::InsertMedal,
        MachineCommand::Stop(0),
        MachineCommand::Stop(1),
        MachineCommand::Stop(2),
    ] {
        assert!(
            commands.contains(&command),
            "{command:?}를 누를 부품이 없다"
        );
    }

    // 7세그먼트: 표시기별 자릿수 × 7획
    let mut segments = app.world_mut().query::<&SevenSegment>();
    for kind in DisplayKind::ALL {
        let count = segments
            .iter(app.world())
            .filter(|s| s.display == kind)
            .count();
        assert_eq!(count, usize::from(kind.digits()) * 7, "{kind:?}");
    }

    // 램프: STAR CHANCE 1, 릴 조명 3, 정지 버튼 3, 베팅 1, 간판 1, 장식 패널 1, LED 다수
    let mut lamps = app.world_mut().query::<&Lamp>();
    let kinds: Vec<LampKind> = lamps.iter(app.world()).map(|l| l.kind).collect();
    let count = |pred: &dyn Fn(&LampKind) -> bool| kinds.iter().filter(|k| pred(k)).count();
    assert_eq!(count(&|k| matches!(k, LampKind::Chance)), 1);
    assert_eq!(count(&|k| matches!(k, LampKind::ReelBacklight(_))), 3);
    assert_eq!(count(&|k| matches!(k, LampKind::StopButton(_))), 3);
    assert_eq!(count(&|k| matches!(k, LampKind::BetButton)), 1);
    assert_eq!(count(&|k| matches!(k, LampKind::Marquee)), 1);
    assert_eq!(count(&|k| matches!(k, LampKind::Belly)), 1);
    assert!(count(&|k| matches!(k, LampKind::Led { .. })) >= 30);
    // 유효 라인 5개마다 막대 1개와 양끝 표시등 2개
    for line in 0..5u8 {
        assert_eq!(count(&|k| *k == LampKind::PayLine(line)), 1, "라인 {line}");
        assert_eq!(count(&|k| *k == LampKind::LineLamp(line)), 2, "라인 {line}");
    }
    assert_eq!(count(&|k| matches!(k, LampKind::BetOneButton)), 1);
    assert_eq!(count(&|k| matches!(k, LampKind::TopLamp)), 1);
    assert_eq!(count(&|k| matches!(k, LampKind::Status(_))), 3);

    // 내부 관찰 모드에서 숨길 외장이 있고, 릴과 조작부는 외장이 아니다
    let mut shell = app.world_mut().query::<(&ShellPart, &Name)>();
    let shell_names: Vec<String> = shell
        .iter(app.world())
        .map(|(_, n)| n.to_string())
        .collect();
    assert!(shell_names.contains(&"FrontGlass".to_owned()));
    assert!(shell_names.contains(&"TopPanel".to_owned()));
    for kept in ["ReelLeft", "StartLever", "MedalTray", "BetButton"] {
        assert!(
            !shell_names.contains(&kept.to_owned()),
            "{kept}는 숨기면 안 된다"
        );
    }
}
