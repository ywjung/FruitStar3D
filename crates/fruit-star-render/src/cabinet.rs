//! 게임기 캐비닛을 절차적으로 만든다.
//!
//! 계획서의 Blender/GLB 모델을 대신하는 코드 생성 모델이다. 부품 이름
//! ([`REQUIRED_PARTS`])과 독립 피벗(레버, 버튼, 릴)은 GLB 노드 명세를 그대로 따른다.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use fruit_star_config::AnimationSection;
use fruit_star_machine::layout::*;
use fruit_star_machine::{
    Clickable, DisplayKind, Lamp, LampKind, LedStrip, MachineCommand, PartKind, Pressable,
    SevenSegment, StatusLamp,
};

use crate::assets::{ArtAssets, Label};
use crate::materials::{
    MaterialLibrary, backlit_material, button_material, gloss_material, printed_material,
    reel_print_material,
};
use crate::reel_mesh::{reel_band_mesh, reel_flange_mesh, window_shade_mesh};

/// 반드시 존재해야 하는 부품 이름 (모델링 명세의 메시 목록).
pub const REQUIRED_PARTS: [&str; 16] = [
    "Cabinet",
    "FrontGlass",
    "ChromeFrame",
    "TopPanel",
    "ReelLeft",
    "ReelCenter",
    "ReelRight",
    "StopButtonLeft",
    "StopButtonCenter",
    "StopButtonRight",
    "StartLever",
    "BetButton",
    "MedalSlot",
    "MedalTray",
    "LEDPanel",
    "Speaker",
];

/// 게임기 전체의 루트.
#[derive(Component, Debug, Clone, Copy)]
pub struct CabinetRoot;

/// 릴 원통. X축 회전으로 위치를 표현한다.
#[derive(Component, Debug, Clone, Copy)]
pub struct ReelDrum(pub usize);

/// 내부 관찰 모드에서 숨기는 외장 부품.
#[derive(Component, Debug, Clone, Copy)]
pub struct ShellPart;

/// 표시창을 가로지르는 당첨 라인 막대. 당첨된 동안에만 보인다.
#[derive(Component, Debug, Clone, Copy)]
pub struct PayLineBar;

/// 눌렸을 때의 움직임. `Pressable::amount`에 비례한다.
#[derive(Component, Debug, Clone, Copy)]
pub enum PressMotion {
    /// 끝까지 눌렸을 때의 이동량.
    Translate(Vec3),
    /// 끝까지 당겼을 때의 X축 회전각 (라디안).
    RotateX(f32),
}

/// 눌리지 않았을 때의 자세.
#[derive(Component, Debug, Clone, Copy)]
pub struct RestPose(pub Transform);

/// 캐비닛을 만드는 데 필요한 입력.
pub struct CabinetSpec<'a> {
    pub library: &'a MaterialLibrary,
    pub art: &'a ArtAssets,
    pub symbol_count: usize,
    pub reel_segments: u32,
    pub animation: &'a AnimationSection,
}

struct Builder<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
}

/// Z축을 향하는 원기둥·원판용 회전 (기본 축은 Y).
fn axis_z() -> Quat {
    Quat::from_rotation_x(FRAC_PI_2)
}

/// X축을 향하는 원기둥·원환용 회전.
fn axis_x() -> Quat {
    Quat::from_rotation_z(FRAC_PI_2)
}

impl Builder<'_, '_, '_> {
    fn group(&mut self, parent: Entity, name: &str, transform: Transform) -> Entity {
        self.commands
            .spawn((
                Name::new(name.to_owned()),
                transform,
                Visibility::Inherited,
                ChildOf(parent),
            ))
            .id()
    }

    fn mesh(
        &mut self,
        parent: Entity,
        name: &str,
        mesh: impl Into<Mesh>,
        material: &Handle<StandardMaterial>,
        transform: Transform,
    ) -> Entity {
        let mesh = self.meshes.add(mesh);
        self.shared_mesh(parent, name, &mesh, material, transform)
    }

    fn shared_mesh(
        &mut self,
        parent: Entity,
        name: &str,
        mesh: &Handle<Mesh>,
        material: &Handle<StandardMaterial>,
        transform: Transform,
    ) -> Entity {
        self.commands
            .spawn((
                Name::new(name.to_owned()),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                transform,
                ChildOf(parent),
            ))
            .id()
    }

    fn cuboid(
        &mut self,
        parent: Entity,
        name: &str,
        size: Vec3,
        center: Vec3,
        material: &Handle<StandardMaterial>,
    ) -> Entity {
        self.mesh(
            parent,
            name,
            Cuboid::new(size.x, size.y, size.z),
            material,
            Transform::from_translation(center),
        )
    }

    /// 정면(+Z)을 향하는 텍스처 사각형.
    fn quad(
        &mut self,
        parent: Entity,
        name: &str,
        size: Vec2,
        center: Vec3,
        material: &Handle<StandardMaterial>,
    ) -> Entity {
        self.mesh(
            parent,
            name,
            Rectangle::new(size.x, size.y),
            material,
            Transform::from_translation(center),
        )
    }

    /// 라벨 사각형. `height`에 맞춰 너비를 정한다. `face_up`이면 윗면에 눕힌다.
    fn label(
        &mut self,
        parent: Entity,
        art: &ArtAssets,
        label: Label,
        height: f32,
        center: Vec3,
        face_up: bool,
    ) {
        let Some((texture, aspect)) = art.label(label) else {
            return;
        };
        let mut material = backlit_material(texture, true);
        material.emissive = LinearRgba::rgb(1.6, 1.5, 1.3);
        let material = self.materials.add(material);
        let mut transform = Transform::from_translation(center);
        if face_up {
            transform.rotation = Quat::from_rotation_x(-FRAC_PI_2);
        }
        self.mesh(
            parent,
            "Label",
            Rectangle::new(height * aspect, height),
            &material,
            transform,
        );
    }

    fn lamp_material(&mut self, color: Color) -> Handle<StandardMaterial> {
        self.materials.add(button_material(color))
    }
}

/// 캐비닛 전체를 만들고 루트 엔티티를 돌려준다.
pub fn spawn_cabinet(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: &CabinetSpec,
) -> Entity {
    let root = commands
        .spawn((
            CabinetRoot,
            Name::new("FruitStarMachine"),
            Transform::IDENTITY,
            Visibility::Visible,
        ))
        .id();
    let mut b = Builder {
        commands,
        meshes,
        materials,
    };
    body(&mut b, root, spec);
    top_panel(&mut b, root, spec);
    reel_panel(&mut b, root, spec);
    reels(&mut b, root, spec);
    displays(&mut b, root, spec);
    paylines(&mut b, root, spec);
    status_lamps(&mut b, root, spec);
    data_counter(&mut b, root, spec);
    chance_lamp(&mut b, root, spec);
    deck(&mut b, root, spec);
    tray(&mut b, root, spec);
    leds(&mut b, root, spec);
    interior(&mut b, root, spec);
    root
}

/// 본체 외장, 받침대, 크롬 테두리.
fn body(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let (w, h, d) = (CABINET_WIDTH, CABINET_HEIGHT, CABINET_DEPTH);
    let cy = CABINET_BOTTOM + h / 2.0;

    let pedestal = b.group(root, "Pedestal", Transform::IDENTITY);
    b.cuboid(
        pedestal,
        "PedestalBody",
        PEDESTAL_SIZE,
        Vec3::new(0.0, PEDESTAL_HEIGHT / 2.0, 0.0),
        &lib.dark_grey,
    );
    b.cuboid(
        pedestal,
        "PedestalKick",
        Vec3::new(PEDESTAL_SIZE.x + 0.004, 0.05, PEDESTAL_SIZE.z + 0.004),
        Vec3::new(0.0, 0.025, 0.0),
        &lib.stainless,
    );

    let cabinet = b.group(root, "Cabinet", Transform::IDENTITY);
    b.cuboid(
        cabinet,
        "Back",
        Vec3::new(w, h, WALL),
        Vec3::new(0.0, cy, -d / 2.0 + WALL / 2.0),
        &lib.red_plastic,
    );
    b.cuboid(
        cabinet,
        "Bottom",
        Vec3::new(w, WALL, d),
        Vec3::new(0.0, CABINET_BOTTOM + WALL / 2.0, 0.0),
        &lib.red_plastic,
    );
    for (name, sign) in [("SideLeft", -1.0), ("SideRight", 1.0)] {
        let side = b.cuboid(
            cabinet,
            name,
            Vec3::new(WALL, h, d),
            Vec3::new(sign * (w / 2.0 - WALL / 2.0), cy, 0.0),
            &lib.red_plastic,
        );
        b.commands.entity(side).insert(ShellPart);
    }
    let top = b.cuboid(
        cabinet,
        "Top",
        Vec3::new(w, WALL, d),
        Vec3::new(0.0, CABINET_TOP - WALL / 2.0, 0.0),
        &lib.red_plastic,
    );
    b.commands.entity(top).insert(ShellPart);

    // 앞면 세로 기둥 (LED가 박히는 자리)
    for (name, sign) in [("StileLeft", -1.0), ("StileRight", 1.0)] {
        let stile = b.cuboid(
            cabinet,
            name,
            Vec3::new(0.030, BELLY_TOP + 0.012 - CABINET_BOTTOM, 0.012),
            Vec3::new(
                sign * (w / 2.0 - 0.015),
                (CABINET_BOTTOM + BELLY_TOP + 0.012) / 2.0,
                FRONT_Z - 0.006,
            ),
            &lib.red_plastic,
        );
        b.commands.entity(stile).insert(ShellPart);
        let upper = b.cuboid(
            cabinet,
            name,
            Vec3::new(0.030, CABINET_TOP - DECK_TOP, 0.012),
            Vec3::new(
                sign * (w / 2.0 - 0.015),
                (CABINET_TOP + DECK_TOP) / 2.0,
                FRONT_Z - 0.006,
            ),
            &lib.red_plastic,
        );
        b.commands.entity(upper).insert(ShellPart);
    }

    // 앞면 아랫부분 (메달 배출구 주변)과 장식 패널 뒤판
    let lower = b.cuboid(
        cabinet,
        "FrontLower",
        Vec3::new(w - 0.06, DECK_BOTTOM - CABINET_BOTTOM, 0.010),
        Vec3::new(0.0, (DECK_BOTTOM + CABINET_BOTTOM) / 2.0, FRONT_Z - 0.006),
        &lib.red_plastic,
    );
    b.commands.entity(lower).insert(ShellPart);

    // 하단 장식 패널 (백라이트)
    let mut belly = backlit_material(spec.art.belly_panel.clone(), false);
    belly.emissive = LinearRgba::rgb(0.8, 0.8, 0.8);
    let belly = b.materials.add(belly);
    let panel = b.quad(
        cabinet,
        "BellyPanel",
        Vec2::new(BELLY_WIDTH, BELLY_TOP - BELLY_BOTTOM),
        Vec3::new(0.0, (BELLY_TOP + BELLY_BOTTOM) / 2.0, FRONT_Z + 0.0006),
        &belly,
    );
    b.commands
        .entity(panel)
        .insert((ShellPart, Lamp::new(LampKind::Belly)));

    // 조작부 선반
    let deck_height = DECK_TOP - DECK_BOTTOM;
    b.cuboid(
        cabinet,
        "Deck",
        Vec3::new(w, deck_height, DECK_DEPTH),
        Vec3::new(
            0.0,
            (DECK_TOP + DECK_BOTTOM) / 2.0,
            FRONT_Z + DECK_DEPTH / 2.0,
        ),
        &lib.red_plastic,
    );
    b.cuboid(
        cabinet,
        "DeckFacePlate",
        Vec3::new(w - 0.05, deck_height - 0.014, 0.003),
        Vec3::new(0.0, (DECK_TOP + DECK_BOTTOM) / 2.0, DECK_FRONT_Z + 0.0015),
        &lib.black_gloss,
    );
    b.cuboid(
        cabinet,
        "DeckTopPlate",
        Vec3::new(w - 0.04, 0.002, DECK_DEPTH - 0.02),
        Vec3::new(0.0, DECK_TOP + 0.001, FRONT_Z + DECK_DEPTH / 2.0),
        &lib.black_gloss,
    );

    // 상단 장식 (크라운)
    let crown = b.cuboid(
        cabinet,
        "Crown",
        Vec3::new(w - 0.04, CROWN_HEIGHT, 0.12),
        Vec3::new(0.0, CABINET_TOP + CROWN_HEIGHT / 2.0, FRONT_Z - 0.06),
        &lib.red_plastic,
    );
    b.commands.entity(crown).insert(ShellPart);

    // 크롬 테두리
    let chrome = b.group(root, "ChromeFrame", Transform::IDENTITY);
    let rail = |b: &mut Builder, name: &str, y: f32, height: f32, z: f32, shell: bool| {
        let entity = b.cuboid(
            chrome,
            name,
            Vec3::new(w + 0.006, height, 0.010),
            Vec3::new(0.0, y, z),
            &lib.chrome,
        );
        if shell {
            b.commands.entity(entity).insert(ShellPart);
        }
    };
    rail(
        b,
        "RailTop",
        CABINET_TOP - 0.004,
        0.008,
        FRONT_Z - 0.002,
        true,
    );
    rail(
        b,
        "RailMarquee",
        (REEL_PANEL_TOP + TOP_PANEL_BOTTOM) / 2.0,
        TOP_PANEL_BOTTOM - REEL_PANEL_TOP,
        FRONT_Z - 0.002,
        true,
    );
    rail(
        b,
        "RailDeck",
        (DECK_TOP + REEL_PANEL_BOTTOM) / 2.0 + 0.001,
        REEL_PANEL_BOTTOM - DECK_TOP - 0.002,
        FRONT_Z - 0.002,
        true,
    );
    rail(
        b,
        "RailDeckEdge",
        DECK_TOP - 0.003,
        0.006,
        DECK_FRONT_Z + 0.001,
        false,
    );
    rail(
        b,
        "RailBelly",
        (BELLY_TOP + DECK_BOTTOM) / 2.0,
        0.008,
        FRONT_Z + 0.001,
        true,
    );
    rail(
        b,
        "RailBase",
        CABINET_BOTTOM + 0.004,
        0.008,
        FRONT_Z - 0.002,
        false,
    );
    let crown_cap = b.cuboid(
        chrome,
        "CrownCap",
        Vec3::new(w - 0.03, 0.006, 0.126),
        Vec3::new(0.0, CABINET_TOP + CROWN_HEIGHT + 0.003, FRONT_Z - 0.06),
        &lib.chrome,
    );
    b.commands.entity(crown_cap).insert(ShellPart);
    // 장식 패널 좌우 세로 테두리
    for sign in [-1.0, 1.0] {
        let trim = b.cuboid(
            chrome,
            "BellyTrim",
            Vec3::new(0.005, BELLY_TOP - BELLY_BOTTOM + 0.008, 0.006),
            Vec3::new(
                sign * (BELLY_WIDTH / 2.0 + 0.0025),
                (BELLY_TOP + BELLY_BOTTOM) / 2.0,
                FRONT_Z + 0.001,
            ),
            &lib.chrome,
        );
        b.commands.entity(trim).insert(ShellPart);
    }
    let trim = b.cuboid(
        chrome,
        "BellyTrimBottom",
        Vec3::new(BELLY_WIDTH + 0.010, 0.005, 0.006),
        Vec3::new(0.0, BELLY_BOTTOM - 0.0025, FRONT_Z + 0.001),
        &lib.chrome,
    );
    b.commands.entity(trim).insert(ShellPart);
}

/// 상단 패널: 백라이트 간판과 스피커.
fn top_panel(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let height = TOP_PANEL_TOP - TOP_PANEL_BOTTOM;
    let cy = (TOP_PANEL_TOP + TOP_PANEL_BOTTOM) / 2.0;
    let panel = b.group(root, "TopPanel", Transform::IDENTITY);
    b.commands.entity(panel).insert(ShellPart);
    b.cuboid(
        panel,
        "TopBacking",
        Vec3::new(CABINET_WIDTH - 0.06, height, 0.008),
        Vec3::new(0.0, cy, FRONT_Z - 0.008),
        &lib.black_gloss,
    );
    let marquee = b
        .materials
        .add(backlit_material(spec.art.top_panel.clone(), false));
    let quad = b.quad(
        panel,
        "Marquee",
        Vec2::new(MARQUEE_WIDTH, MARQUEE_WIDTH / MARQUEE_ASPECT),
        Vec3::new(0.0, cy, FRONT_Z - 0.0035),
        &marquee,
    );
    b.commands.entity(quad).insert(Lamp::new(LampKind::Marquee));

    let speakers = b.group(root, "Speaker", Transform::IDENTITY);
    b.commands.entity(speakers).insert(ShellPart);
    for (name, sign) in [("SpeakerLeft", -1.0), ("SpeakerRight", 1.0)] {
        let center = Vec3::new(sign * SPEAKER_X, cy, FRONT_Z - 0.003);
        b.mesh(
            speakers,
            name,
            Cylinder::new(SPEAKER_RADIUS, 0.004),
            &lib.black_matte,
            Transform::from_translation(center).with_rotation(axis_z()),
        );
        b.mesh(
            speakers,
            "SpeakerRing",
            Torus::new(SPEAKER_RADIUS - 0.002, SPEAKER_RADIUS + 0.002),
            &lib.chrome,
            Transform::from_translation(center + Vec3::Z * 0.002).with_rotation(axis_z()),
        );
        // 그릴 무늬: 동심원 고리
        for ring in 1..=3 {
            let radius = SPEAKER_RADIUS * ring as f32 / 4.0;
            b.mesh(
                speakers,
                "SpeakerGrill",
                Torus::new(radius - 0.0008, radius + 0.0008),
                &lib.dark_grey,
                Transform::from_translation(center + Vec3::Z * 0.0022).with_rotation(axis_z()),
            );
        }
    }
}

/// 릴 패널: 표시창이 뚫린 검은 패널과 앞 유리.
fn reel_panel(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let panel_width = CABINET_WIDTH - 0.06;
    let z = FRONT_Z - 0.008;
    let thickness = 0.006;
    let window_top = REEL_CENTER_Y + WINDOW_HEIGHT / 2.0;
    let window_bottom = REEL_CENTER_Y - WINDOW_HEIGHT / 2.0;
    let side_width = (panel_width - WINDOW_WIDTH) / 2.0;

    let bezel = b.group(root, "ReelPanel", Transform::IDENTITY);
    b.commands.entity(bezel).insert(ShellPart);
    let shade = b
        .meshes
        .add(window_shade_mesh(REEL_RADIUS + 0.001, REEL_WIDTH));
    let shade_material = b.materials.add(StandardMaterial {
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for index in 0..3 {
        let shadow = b.shared_mesh(
            bezel,
            "WindowShade",
            &shade,
            &shade_material,
            Transform::from_translation(reel_center(index)),
        );
        b.commands
            .entity(shadow)
            .insert((NotShadowCaster, Pickable::IGNORE));
    }
    b.cuboid(
        bezel,
        "BezelTop",
        Vec3::new(panel_width, REEL_PANEL_TOP - window_top, thickness),
        Vec3::new(0.0, (REEL_PANEL_TOP + window_top) / 2.0, z),
        &lib.black_gloss,
    );
    b.cuboid(
        bezel,
        "BezelBottom",
        Vec3::new(panel_width, window_bottom - REEL_PANEL_BOTTOM, thickness),
        Vec3::new(0.0, (REEL_PANEL_BOTTOM + window_bottom) / 2.0, z),
        &lib.black_gloss,
    );
    for sign in [-1.0, 1.0] {
        b.cuboid(
            bezel,
            "BezelSide",
            Vec3::new(side_width, WINDOW_HEIGHT, thickness),
            Vec3::new(
                sign * (WINDOW_WIDTH / 2.0 + side_width / 2.0),
                REEL_CENTER_Y,
                z,
            ),
            &lib.black_gloss,
        );
    }
    // 표시창 테두리 (크롬)
    let frame = 0.003;
    for (size, center) in [
        (
            Vec3::new(WINDOW_WIDTH + 2.0 * frame, frame, 0.004),
            Vec3::new(0.0, window_top + frame / 2.0, z + 0.004),
        ),
        (
            Vec3::new(WINDOW_WIDTH + 2.0 * frame, frame, 0.004),
            Vec3::new(0.0, window_bottom - frame / 2.0, z + 0.004),
        ),
        (
            Vec3::new(frame, WINDOW_HEIGHT, 0.004),
            Vec3::new(-WINDOW_WIDTH / 2.0 - frame / 2.0, REEL_CENTER_Y, z + 0.004),
        ),
        (
            Vec3::new(frame, WINDOW_HEIGHT, 0.004),
            Vec3::new(WINDOW_WIDTH / 2.0 + frame / 2.0, REEL_CENTER_Y, z + 0.004),
        ),
    ] {
        b.cuboid(bezel, "WindowFrame", size, center, &lib.chrome);
    }

    // 배당표: 표시창 위에 인쇄된 심볼별 배당 (백라이트)
    let mut paytable = backlit_material(spec.art.paytable.clone(), true);
    paytable.emissive = LinearRgba::rgb(0.9, 0.9, 0.9);
    let paytable = b.materials.add(paytable);
    b.quad(
        bezel,
        "PayTable",
        Vec2::new(PAYTABLE_SIZE.x, PAYTABLE_SIZE.y),
        Vec3::new(
            0.0,
            (window_top + REEL_PANEL_TOP) / 2.0 + 0.001,
            FRONT_Z - 0.0046,
        ),
        &paytable,
    );

    let glass = b.cuboid(
        root,
        "FrontGlass",
        Vec3::new(panel_width, REEL_PANEL_TOP - REEL_PANEL_BOTTOM, 0.003),
        Vec3::new(
            0.0,
            (REEL_PANEL_TOP + REEL_PANEL_BOTTOM) / 2.0,
            FRONT_Z - 0.002,
        ),
        &lib.glass,
    );
    b.commands.entity(glass).insert(ShellPart);
}

/// 릴 세 개와 릴 유닛.
fn reels(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let band = b.meshes.add(reel_band_mesh(
        REEL_RADIUS,
        REEL_WIDTH,
        spec.symbol_count,
        spec.reel_segments,
    ));
    let sleeve = b.meshes.add(
        Extrusion::new(
            Annulus::new(REEL_RADIUS - 0.007, REEL_RADIUS - 0.003),
            REEL_WIDTH,
        )
        .mesh()
        .resolution(spec.reel_segments),
    );
    let flange = b
        .meshes
        .add(reel_flange_mesh(REEL_RADIUS - 0.005, 0.010, 0.004));
    let satin = b.materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.49, 0.43),
        metallic: 0.65,
        perceptual_roughness: 0.38,
        ..default()
    });
    let rim = b
        .meshes
        .add(Torus::new(REEL_RADIUS - 0.006, REEL_RADIUS - 0.0005));
    let bead = b
        .meshes
        .add(Torus::new(REEL_RADIUS - 0.018, REEL_RADIUS - 0.015));
    let bearing = b
        .meshes
        .add(Extrusion::new(Annulus::new(0.007, 0.019), 0.006));
    let rivet = b.meshes.add(Sphere::new(0.0036).mesh().uv(16, 8));
    let tube_rotation = Quat::from_rotation_y(FRAC_PI_2);
    let names = ["ReelLeft", "ReelCenter", "ReelRight"];
    for (index, name) in names.into_iter().enumerate() {
        let center = reel_center(index);
        let drum = b.group(root, name, Transform::from_translation(center));
        b.commands.entity(drum).insert(ReelDrum(index));

        let material = b
            .materials
            .add(reel_print_material(spec.art.strips[index].clone()));
        b.commands.spawn((
            Name::new("ReelBand"),
            Mesh3d(band.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            ChildOf(drum),
            Lamp::new(LampKind::ReelBacklight(index)),
            Clickable(MachineCommand::Stop(index)),
            Pickable::default(),
        ));
        b.shared_mesh(
            drum,
            "ReelSleeve",
            &sleeve,
            &lib.dark_grey,
            Transform::from_rotation(tube_rotation),
        );
        for sign in [-1.0, 1.0] {
            let end = |inset| Transform::from_xyz(sign * (REEL_WIDTH / 2.0 - inset), 0., 0.);
            let plate = b.shared_mesh(drum, "ReelFlange", &flange, &satin, end(0.005));
            b.commands
                .entity(plate)
                .insert((Clickable(MachineCommand::Stop(index)), Pickable::default()));
            b.shared_mesh(
                drum,
                "ReelRim",
                &rim,
                &lib.stainless,
                end(0.).with_rotation(axis_x()),
            );
            b.shared_mesh(
                drum,
                "PressedRib",
                &bead,
                &satin,
                end(0.003).with_rotation(axis_x()),
            );
            b.shared_mesh(
                drum,
                "ReelBushing",
                &bearing,
                &lib.copper,
                end(0.002).with_rotation(tube_rotation),
            );
            for i in 0..3 {
                let (sin, cos) = (i as f32 * TAU / 3.0).sin_cos();
                let head = end(0.003).translation + Vec3::new(0., cos, sin) * 0.031;
                b.shared_mesh(
                    drum,
                    "HubRivet",
                    &rivet,
                    &lib.stainless,
                    Transform::from_translation(head).with_scale(Vec3::new(0.42, 1., 1.)),
                );
            }
            // 회전 위상이 보이도록 바깥 비드에 작은 기준점을 찍는다.
            b.mesh(
                drum,
                "ReelIndex",
                Cylinder::new(0.004, 0.0015),
                &lib.copper,
                Transform::from_xyz(sign * (REEL_WIDTH / 2.0 - 0.0025), REEL_RADIUS - 0.013, 0.)
                    .with_rotation(axis_x()),
            );
        }
    }

    reel_frame(b, root, lib);
}

/// 고정축과 베어링 지지대는 회전하는 드럼의 바깥에 둔다.
fn reel_frame(b: &mut Builder, root: Entity, lib: &MaterialLibrary) {
    let unit = b.group(root, "ReelUnit", Transform::IDENTITY);
    let span = 3.0 * REEL_WIDTH + 2.0 * REEL_GAP;
    b.cuboid(
        unit,
        "ReelFrameBase",
        Vec3::new(span + 0.05, 0.004, REEL_RADIUS * 2.0 + 0.03),
        Vec3::new(0.0, REEL_CENTER_Y - REEL_RADIUS - 0.014, REEL_AXIS_Z),
        &lib.stainless,
    );
    b.cuboid(
        unit,
        "ReelBackShield",
        Vec3::new(span + 0.05, REEL_RADIUS * 2.0 + 0.03, 0.003),
        Vec3::new(0.0, REEL_CENTER_Y, REEL_AXIS_Z - REEL_RADIUS - 0.012),
        &lib.black_matte,
    );
    let axis = reel_center(1);
    let support_x = span / 2.0 + 0.018;
    b.mesh(
        unit,
        "ReelAxle",
        Cylinder::new(0.0065, 2.0 * support_x + 0.018),
        &lib.stainless,
        Transform::from_translation(axis).with_rotation(axis_x()),
    );
    for sign in [-1.0, 1.0] {
        let bearing_center = axis + Vec3::X * sign * support_x;
        for depth in [-0.083, 0.083] {
            let foot = bearing_center + Vec3::new(0., -REEL_RADIUS - 0.012, depth);
            let leg = bearing_center - foot;
            b.mesh(
                unit,
                "BearingSupport",
                Cuboid::new(0.008, leg.length(), 0.013),
                &lib.motor_black,
                Transform::from_translation(foot + leg * 0.5)
                    .with_rotation(Quat::from_rotation_x(leg.z.atan2(leg.y))),
            );
        }
        b.mesh(
            unit,
            "BearingHousing",
            Extrusion::new(Annulus::new(0.0068, 0.015), 0.010),
            &lib.stainless,
            Transform::from_translation(bearing_center)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        );
    }
    for index in 0..3 {
        let center = reel_center(index);
        b.mesh(
            unit,
            "StepperMotor",
            Cylinder::new(0.038, REEL_WIDTH * 0.42),
            &lib.motor_black,
            Transform::from_translation(center - Vec3::X * 0.012).with_rotation(axis_x()),
        );
        b.mesh(
            unit,
            "StepperCoil",
            Torus::new(0.033, 0.041),
            &lib.copper,
            Transform::from_translation(center - Vec3::X * 0.012).with_rotation(axis_x()),
        );
    }
}

/// 7세그먼트 표시기 하나를 만든다: 어두운 바탕, 자릿수만큼의 숫자, 이름표.
///
/// `center`는 숫자 줄의 중심이고, 이름표는 `label_above`에 따라 그 위나 아래에 붙는다.
fn segment_display(
    b: &mut Builder,
    group: Entity,
    spec: &CabinetSpec,
    kind: DisplayKind,
    center: Vec3,
    label: Label,
    label_above: bool,
) {
    let lib = spec.library;
    let (w, h, t) = (DIGIT_WIDTH, DIGIT_HEIGHT, 0.0026);
    let horizontal = b.meshes.add(Rectangle::new(w - t * 1.3, t));
    let vertical = b.meshes.add(Rectangle::new(t, h / 2.0 - t * 1.3));
    // 획 a~g의 중심
    let segments: [(Vec2, bool); 7] = [
        (Vec2::new(0.0, h / 2.0 - t / 2.0), true),
        (Vec2::new(w / 2.0 - t / 2.0, h / 4.0), false),
        (Vec2::new(w / 2.0 - t / 2.0, -h / 4.0), false),
        (Vec2::new(0.0, -h / 2.0 + t / 2.0), true),
        (Vec2::new(-w / 2.0 + t / 2.0, -h / 4.0), false),
        (Vec2::new(-w / 2.0 + t / 2.0, h / 4.0), false),
        (Vec2::new(0.0, 0.0), true),
    ];
    let backing = b.materials.add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.0, 0.0),
        perceptual_roughness: 0.3,
        ..default()
    });
    let digits = kind.digits();
    let width = f32::from(digits) * DIGIT_PITCH + 0.006;
    b.quad(
        group,
        "DisplayBacking",
        Vec2::new(width, h + 0.010),
        center - Vec3::Z * 0.0003,
        &backing,
    );
    for digit in 0..digits {
        // digit 0이 일의 자리(맨 오른쪽)
        let x = center.x + (f32::from(digits) - 1.0) / 2.0 * DIGIT_PITCH
            - f32::from(digit) * DIGIT_PITCH;
        for (segment, (offset, is_horizontal)) in segments.iter().enumerate() {
            let mesh = if *is_horizontal {
                horizontal.clone()
            } else {
                vertical.clone()
            };
            b.commands.spawn((
                Name::new("Segment"),
                Mesh3d(mesh),
                MeshMaterial3d(lib.segment_off.clone()),
                Transform::from_xyz(x + offset.x, center.y + offset.y, center.z),
                ChildOf(group),
                SevenSegment {
                    display: kind,
                    digit,
                    segment: segment as u8,
                    lit: false,
                },
            ));
        }
    }
    let offset = h / 2.0 + 0.0115;
    let label_y = if label_above {
        center.y + offset
    } else {
        center.y - offset
    };
    b.label(
        group,
        spec.art,
        label,
        0.0068,
        Vec3::new(center.x, label_y, center.z),
        false,
    );
}

/// 릴 패널의 CREDIT / COUNT / PAYOUT 표시기.
fn displays(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let group = b.group(root, "Displays", Transform::IDENTITY);
    b.commands.entity(group).insert(ShellPart);
    let z = FRONT_Z - 0.0045;
    for (kind, x, label) in [
        (DisplayKind::Credit, DISPLAY_CREDIT_X, Label::Credit),
        (DisplayKind::Count, DISPLAY_COUNT_X, Label::Count),
        (DisplayKind::Payout, DISPLAY_PAYOUT_X, Label::Payout),
    ] {
        segment_display(
            b,
            group,
            spec,
            kind,
            Vec3::new(x, DISPLAY_Y, z),
            label,
            false,
        );
    }
}

/// 데이터 카운터와 톱 램프: 게임기 위에 얹힌 집계 표시기.
/// 마지막 보너스 이후 게임 수, BIG·REGULAR 횟수, 총 게임 수를 보여 준다.
fn data_counter(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let group = b.group(root, "DataCounter", Transform::IDENTITY);
    b.commands.entity(group).insert(ShellPart);
    let size = DATA_COUNTER_SIZE;
    let center = DATA_COUNTER_CENTER;
    b.cuboid(group, "DataCounterBody", size, center, &lib.black_gloss);
    b.cuboid(
        group,
        "DataCounterTrim",
        Vec3::new(size.x + 0.006, 0.004, size.z + 0.006),
        center - Vec3::Y * (size.y / 2.0 + 0.002),
        &lib.chrome,
    );
    b.cuboid(
        group,
        "DataCounterCap",
        Vec3::new(size.x + 0.006, 0.004, size.z + 0.006),
        center + Vec3::Y * (size.y / 2.0 + 0.002),
        &lib.chrome,
    );
    let face_z = center.z + size.z / 2.0 + 0.0006;
    let row_y = center.y - 0.008;
    for (kind, x, label) in [
        (DisplayKind::Games, -0.118, Label::Games),
        (DisplayKind::Big, -0.030, Label::Big),
        (DisplayKind::Regular, 0.034, Label::Reg),
        (DisplayKind::Total, 0.118, Label::Total),
    ] {
        segment_display(
            b,
            group,
            spec,
            kind,
            Vec3::new(x, row_y, face_z),
            label,
            true,
        );
    }

    // 톱 램프: 받침 위의 반구형 등
    let top = center + Vec3::Y * (size.y / 2.0 + 0.004);
    b.mesh(
        group,
        "TopLampBase",
        Cylinder::new(TOP_LAMP_RADIUS + 0.004, 0.006),
        &lib.chrome,
        Transform::from_translation(top + Vec3::Y * 0.003),
    );
    let material = b.lamp_material(Color::srgb(0.55, 0.06, 0.05));
    let dome = b.mesh(
        group,
        "TopLamp",
        Sphere::new(TOP_LAMP_RADIUS),
        &material,
        Transform::from_translation(top + Vec3::Y * 0.006).with_scale(Vec3::new(1.0, 0.85, 1.0)),
    );
    b.commands.entity(dome).insert(Lamp::new(LampKind::TopLamp));
}

/// 유효 라인 표시: 표시창을 가로지르는 발광 막대 5개와 양끝의 작은 표시등.
///
/// 막대는 그 라인에서 당첨되었을 때만 보이고, 표시등은 항상 보이다가 당첨되면 켜진다.
fn paylines(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let group = b.group(root, "PayLines", Transform::IDENTITY);
    b.commands.entity(group).insert(ShellPart);

    let pitch = REEL_WIDTH + REEL_GAP;
    // 위·아래 줄 심볼 중심의 높이 차 (릴 표면이 한 칸 돌아간 만큼)
    let row_rise = REEL_RADIUS * (std::f32::consts::TAU / spec.symbol_count as f32).sin();
    let row_y = |row: usize| match row {
        0 => row_rise,
        1 => 0.0,
        _ => -row_rise,
    };
    let bar_z = FRONT_Z - 0.0044;
    let dot_z = FRONT_Z - 0.0046;
    let dot_x = WINDOW_WIDTH / 2.0 + 0.0064;

    for (line, rows) in fruit_star_game::PAYLINES.iter().enumerate() {
        // 왼쪽 릴 중심에서 오른쪽 릴 중심으로 이어지는 선
        let (left, right) = (row_y(rows[0]), row_y(rows[2]));
        let slope = (right - left) / (2.0 * pitch);
        let angle = slope.atan();
        let centre_y = REEL_CENTER_Y + (left + right) / 2.0;
        let length = (WINDOW_WIDTH - 0.008) / angle.cos();

        let material = b.materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.18, 0.05),
            perceptual_roughness: 0.4,
            ..default()
        });
        let bar = b.mesh(
            group,
            "PayLineBar",
            Cuboid::new(length, 0.0030, 0.0008),
            &material,
            Transform::from_xyz(0.0, centre_y, bar_z).with_rotation(Quat::from_rotation_z(angle)),
        );
        b.commands.entity(bar).insert((
            PayLineBar,
            Lamp::new(LampKind::PayLine(line as u8)),
            Visibility::Hidden,
        ));

        for sign in [-1.0, 1.0] {
            let x = sign * dot_x;
            let y = centre_y + slope * x;
            b.mesh(
                group,
                "PayLineLampSeat",
                Circle::new(0.0042),
                &lib.black_matte,
                Transform::from_xyz(x, y, dot_z - 0.0001),
            );
            let material = b.lamp_material(Color::srgb(0.32, 0.20, 0.04));
            let dot = b.mesh(
                group,
                "PayLineLamp",
                Sphere::new(0.0030),
                &material,
                Transform::from_xyz(x, y, dot_z).with_scale(Vec3::new(1.0, 1.0, 0.35)),
            );
            b.commands
                .entity(dot)
                .insert(Lamp::new(LampKind::LineLamp(line as u8)));
        }
    }
}

/// STAR CHANCE 램프.
fn chance_lamp(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let group = b.group(root, "StarChanceLamp", Transform::IDENTITY);
    b.commands.entity(group).insert(ShellPart);
    let pos = CHANCE_LAMP_POS;
    b.mesh(
        group,
        "ChanceBase",
        Circle::new(CHANCE_LAMP_RADIUS + 0.003),
        &lib.black_matte,
        Transform::from_translation(pos - Vec3::Z * 0.0006),
    );
    b.mesh(
        group,
        "ChanceRing",
        Torus::new(CHANCE_LAMP_RADIUS + 0.0015, CHANCE_LAMP_RADIUS + 0.004),
        &lib.chrome,
        Transform::from_translation(pos - Vec3::Z * 0.0004).with_rotation(axis_z()),
    );
    let mut lens = backlit_material(spec.art.star_lamp.clone(), true);
    lens.base_color = Color::srgb(0.25, 0.32, 0.42);
    let lens = b.materials.add(lens);
    let star = b.mesh(
        group,
        "ChanceLens",
        Rectangle::new(CHANCE_LAMP_RADIUS * 2.0, CHANCE_LAMP_RADIUS * 2.0),
        &lens,
        Transform::from_translation(pos),
    );
    b.commands.entity(star).insert(Lamp::new(LampKind::Chance));
    b.label(
        group,
        spec.art,
        Label::StarChance,
        0.0066,
        Vec3::new(pos.x, pos.y - CHANCE_LAMP_RADIUS - 0.010, pos.z),
        false,
    );
}

/// 조작부: 정지 버튼, 레버, MAX BET, 메달 투입구.
///
/// 실제 기기의 부품 구성을 따른다: 버튼은 크롬 링 안에 들어앉은 돔형 캡이고,
/// 레버는 고무 부트에서 나온 크롬 축 끝에 둥근 손잡이가 달려 있다.
fn deck(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    stop_buttons(b, root, spec);
    start_lever(b, root, spec);
    bet_button(b, root, spec);
    medal_slot(b, root, spec);

    // 도어 키: 앞문을 여는 자물쇠 (장식)
    let key = Vec3::new(0.212, (DECK_TOP + DECK_BOTTOM) / 2.0, DECK_FRONT_Z + 0.003);
    b.mesh(
        root,
        "DoorKeyBezel",
        Cylinder::new(0.0125, 0.005),
        &spec.library.chrome,
        Transform::from_translation(key + Vec3::Z * 0.0025).with_rotation(axis_z()),
    );
    b.mesh(
        root,
        "DoorKeyCylinder",
        Cylinder::new(0.0085, 0.004),
        &spec.library.stainless,
        Transform::from_translation(key + Vec3::Z * 0.0055).with_rotation(axis_z()),
    );
    b.cuboid(
        root,
        "DoorKeySlot",
        Vec3::new(0.0016, 0.0085, 0.0012),
        key + Vec3::Z * 0.0078,
        &spec.library.black_matte,
    );

    // 선반 앞판을 고정하는 나사
    let lib = spec.library;
    let details = b.group(root, "DeckDetails", Transform::IDENTITY);
    let half = (CABINET_WIDTH - 0.05) / 2.0 - 0.008;
    for x in [-half, half] {
        for y in [DECK_BOTTOM + 0.013, DECK_TOP - 0.013] {
            b.mesh(
                details,
                "DeckScrew",
                Cylinder::new(0.0032, 0.002),
                &lib.stainless,
                Transform::from_xyz(x, y, DECK_FRONT_Z + 0.0035).with_rotation(axis_z()),
            );
        }
    }
}

/// 정지 버튼 세 개: 크롬 링, 검은 받침, 돔형 발광 캡.
fn stop_buttons(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let radius = STOP_BUTTON_RADIUS;
    let names = ["StopButtonLeft", "StopButtonCenter", "StopButtonRight"];
    for (index, name) in names.into_iter().enumerate() {
        let pos = stop_button_pos(index);
        let group = b.group(root, name, Transform::IDENTITY);
        // 받침과 크롬 링 (고정)
        b.mesh(
            group,
            "StopCollar",
            Cylinder::new(radius + 0.0075, 0.005),
            &lib.black_gloss,
            Transform::from_translation(pos + Vec3::Z * 0.0045).with_rotation(axis_z()),
        );
        b.mesh(
            group,
            "StopBezel",
            Torus::new(radius + 0.0008, radius + 0.0066),
            &lib.chrome,
            Transform::from_translation(pos + Vec3::Z * 0.0072).with_rotation(axis_z()),
        );
        // 캡 (눌리는 부분): 원통 몸통 위에 납작한 돔
        let material = b.lamp_material(Color::srgb(0.85, 0.30, 0.04));
        let rest = Transform::from_translation(pos + Vec3::Z * 0.0075);
        let cap = b.group(group, "StopCap", rest);
        b.commands.entity(cap).insert((
            Pressable::new(PartKind::Stop(index), spec.animation.stop_button_ms),
            PressMotion::Translate(Vec3::new(0.0, 0.0, -0.0060)),
            RestPose(rest),
        ));
        let skirt = b.mesh(
            cap,
            "StopCapSkirt",
            Cylinder::new(radius - 0.0006, 0.009),
            &material,
            Transform::from_translation(Vec3::Z * 0.0045).with_rotation(axis_z()),
        );
        b.commands.entity(skirt).insert((
            Lamp::new(LampKind::StopButton(index)),
            Clickable(MachineCommand::Stop(index)),
            Pickable::default(),
        ));
        let dome = b.mesh(
            cap,
            "StopCapDome",
            Sphere::new(radius - 0.0006),
            &material,
            Transform::from_translation(Vec3::Z * 0.0088).with_scale(Vec3::new(1.0, 1.0, 0.34)),
        );
        b.commands
            .entity(dome)
            .insert((Clickable(MachineCommand::Stop(index)), Pickable::default()));
        if index == 1 {
            b.label(
                group,
                spec.art,
                Label::Stop,
                0.0058,
                Vec3::new(pos.x, DECK_BOTTOM + 0.0115, DECK_FRONT_Z + 0.0032),
                false,
            );
        }
    }
}

/// START 레버: 받침판, 고무 부트, 크롬 축, 둥근 손잡이.
fn start_lever(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let lever = b.group(root, "StartLever", Transform::IDENTITY);
    b.mesh(
        lever,
        "LeverPlate",
        Cylinder::new(0.0285, 0.004),
        &lib.chrome,
        Transform::from_translation(LEVER_PIVOT + Vec3::Z * 0.0035).with_rotation(axis_z()),
    );
    b.mesh(
        lever,
        "LeverPlateRing",
        Torus::new(0.0235, 0.0285),
        &lib.chrome,
        Transform::from_translation(LEVER_PIVOT + Vec3::Z * 0.0055).with_rotation(axis_z()),
    );
    // 고무 부트: 축이 나오는 구멍을 덮는 원뿔대
    b.mesh(
        lever,
        "LeverBoot",
        ConicalFrustum {
            radius_top: 0.0085,
            radius_bottom: 0.0205,
            height: 0.016,
        },
        &lib.black_matte,
        Transform::from_translation(LEVER_PIVOT + Vec3::Z * 0.0135).with_rotation(axis_z()),
    );

    let rest = Transform::from_translation(LEVER_PIVOT);
    let pivot = b.group(lever, "LeverPivot", rest);
    b.commands.entity(pivot).insert((
        Pressable::new(PartKind::Lever, spec.animation.lever_ms),
        PressMotion::RotateX(0.50),
        RestPose(rest),
    ));
    b.mesh(
        pivot,
        "LeverShaft",
        Cylinder::new(0.0048, LEVER_LENGTH),
        &lib.chrome,
        Transform::from_translation(Vec3::Z * (LEVER_LENGTH / 2.0)).with_rotation(axis_z()),
    );
    b.mesh(
        pivot,
        "LeverCollar",
        Cylinder::new(0.0085, 0.007),
        &lib.chrome,
        Transform::from_translation(Vec3::Z * (LEVER_LENGTH - LEVER_KNOB_RADIUS * 0.82))
            .with_rotation(axis_z()),
    );
    let knob_material = b
        .materials
        .add(gloss_material(Color::srgb(0.035, 0.035, 0.04)));
    let knob = b.mesh(
        pivot,
        "LeverKnob",
        Sphere::new(LEVER_KNOB_RADIUS),
        &knob_material,
        Transform::from_translation(Vec3::Z * LEVER_LENGTH),
    );
    b.commands
        .entity(knob)
        .insert((Clickable(MachineCommand::Lever), Pickable::default()));
    b.label(
        lever,
        spec.art,
        Label::Start,
        0.0058,
        Vec3::new(LEVER_PIVOT.x, DECK_BOTTOM + 0.0115, DECK_FRONT_Z + 0.0032),
        false,
    );
}

/// 선반 윗면의 사각 발광 버튼 하나를 만드는 데 필요한 값.
struct DeckButton {
    name: &'static str,
    position: Vec3,
    size: Vec3,
    part: PartKind,
    command: MachineCommand,
    lamp: LampKind,
    label: Label,
    color: Color,
}

/// 베팅 버튼 두 개: MAX BET(최대 베팅)과 1 BET(한 장씩).
fn bet_button(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    deck_button(
        b,
        root,
        spec,
        &DeckButton {
            name: "BetButton",
            position: BET_BUTTON_POS,
            size: BET_BUTTON_SIZE,
            part: PartKind::Bet,
            command: MachineCommand::Bet,
            lamp: LampKind::BetButton,
            label: Label::MaxBet,
            color: Color::srgb(0.90, 0.60, 0.07),
        },
    );
    deck_button(
        b,
        root,
        spec,
        &DeckButton {
            name: "BetOneButton",
            position: BET_ONE_BUTTON_POS,
            size: BET_ONE_BUTTON_SIZE,
            part: PartKind::BetOne,
            command: MachineCommand::BetOne,
            lamp: LampKind::BetOneButton,
            label: Label::OneBet,
            color: Color::srgb(0.20, 0.62, 0.90),
        },
    );
}

/// 크롬 테두리 안의 발광 사각 캡. 글자는 캡 윗면에 인쇄되어 함께 움직인다.
fn deck_button(b: &mut Builder, root: Entity, spec: &CabinetSpec, button: &DeckButton) {
    let lib = spec.library;
    let size = button.size;
    let group = b.group(root, button.name, Transform::IDENTITY);
    b.cuboid(
        group,
        "ButtonBezel",
        Vec3::new(size.x + 0.014, 0.006, size.z + 0.014),
        button.position + Vec3::Y * 0.005,
        &lib.chrome,
    );
    b.cuboid(
        group,
        "ButtonWell",
        Vec3::new(size.x + 0.004, 0.0066, size.z + 0.004),
        button.position + Vec3::Y * 0.0051,
        &lib.black_gloss,
    );
    let material = b.lamp_material(button.color);
    let rest = Transform::from_translation(button.position + Vec3::Y * (0.0068 + size.y / 2.0));
    let cap = b.group(group, "ButtonCap", rest);
    b.commands.entity(cap).insert((
        Pressable::new(button.part, spec.animation.bet_button_ms),
        PressMotion::Translate(Vec3::new(0.0, -0.0052, 0.0)),
        RestPose(rest),
    ));
    let body = b.mesh(
        cap,
        "ButtonCapBody",
        Cuboid::new(size.x, size.y, size.z),
        &material,
        Transform::IDENTITY,
    );
    b.commands.entity(body).insert((
        Lamp::new(button.lamp),
        Clickable(button.command),
        Pickable::default(),
    ));
    // 윗면 모서리를 한 단 접어 사출 캡의 모따기를 흉내 낸다.
    let top = b.mesh(
        cap,
        "ButtonCapTop",
        Cuboid::new(size.x - 0.007, 0.0024, size.z - 0.007),
        &material,
        Transform::from_translation(Vec3::Y * (size.y / 2.0 + 0.0012)),
    );
    b.commands
        .entity(top)
        .insert((Clickable(button.command), Pickable::default()));
    if let Some((texture, aspect)) = spec.art.label(button.label) {
        let ink = b
            .materials
            .add(printed_material(texture, Color::srgb(0.06, 0.03, 0.0)));
        // 글자는 캡 윗면 너비 안에 들어가도록 높이를 정한다.
        let height = (0.0105f32).min((size.x - 0.014) / aspect);
        b.mesh(
            cap,
            "ButtonCapPrint",
            Rectangle::new(height * aspect, height),
            &ink,
            Transform::from_translation(Vec3::Y * (size.y / 2.0 + 0.0026))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        );
    }
}

/// 상태 표시등: 표시창 오른쪽의 REPLAY / START / INSERT MEDAL 글자판.
/// 꺼져 있을 때는 글자가 어둡게만 보이고, 해당 상태가 되면 색을 띠고 빛난다.
fn status_lamps(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let group = b.group(root, "StatusLamps", Transform::IDENTITY);
    b.commands.entity(group).insert(ShellPart);
    let z = FRONT_Z - 0.0045;
    let size = STATUS_LAMP_SIZE;
    for (index, (status, label)) in [
        (StatusLamp::Replay, Label::Replay),
        (StatusLamp::Start, Label::Start),
        (StatusLamp::Insert, Label::Insert),
    ]
    .into_iter()
    .enumerate()
    {
        let y = STATUS_LAMP_TOP_Y - index as f32 * STATUS_LAMP_PITCH;
        b.quad(
            group,
            "StatusPlate",
            Vec2::new(size.x + 0.003, size.y + 0.003),
            Vec3::new(STATUS_LAMP_X, y, z - 0.0002),
            &lib.black_gloss,
        );
        let Some((texture, aspect)) = spec.art.label(label) else {
            continue;
        };
        let mut material = backlit_material(texture, true);
        // 꺼졌을 때는 글자가 희미하게만 보인다.
        material.base_color = Color::srgb(0.34, 0.34, 0.38);
        let material = b.materials.add(material);
        let height = (0.0085f32).min((size.x - 0.004) / aspect);
        let plate = b.mesh(
            group,
            "StatusText",
            Rectangle::new(height * aspect, height),
            &material,
            Transform::from_xyz(STATUS_LAMP_X, y, z),
        );
        b.commands
            .entity(plate)
            .insert(Lamp::new(LampKind::Status(status)));
    }
}

/// 메달 투입구: 받침판 위에 플레이어 쪽으로 기운 크롬 머리와 투입 홈.
fn medal_slot(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let slot = b.group(root, "MedalSlot", Transform::IDENTITY);
    b.cuboid(
        slot,
        "SlotPlate",
        Vec3::new(0.064, 0.004, 0.044),
        MEDAL_SLOT_POS + Vec3::Y * 0.004,
        &lib.stainless,
    );
    let tilt = Quat::from_rotation_x(0.30);
    let head_center = MEDAL_SLOT_POS + Vec3::Y * 0.0135;
    let head = b.mesh(
        slot,
        "SlotHead",
        Cuboid::new(0.052, 0.015, 0.030),
        &lib.chrome,
        Transform::from_translation(head_center).with_rotation(tilt),
    );
    b.commands.entity(head).insert((
        Pressable::new(PartKind::MedalSlot, spec.animation.medal_insert_ms),
        Clickable(MachineCommand::InsertMedal),
        Pickable::default(),
    ));
    // 머리 윗면의 투입 홈과 양쪽 안내 턱
    let top = head_center + tilt * Vec3::Y * 0.0076;
    b.mesh(
        slot,
        "SlotOpening",
        Cuboid::new(MEDAL_RADIUS * 2.0 + 0.004, 0.0012, 0.0036),
        &lib.black_matte,
        Transform::from_translation(top).with_rotation(tilt),
    );
    for sign in [-1.0, 1.0] {
        b.mesh(
            slot,
            "SlotGuide",
            Cuboid::new(0.003, 0.004, 0.016),
            &lib.stainless,
            Transform::from_translation(
                top + tilt * Vec3::new(sign * (MEDAL_RADIUS + 0.0045), 0.002, 0.0),
            )
            .with_rotation(tilt),
        );
    }
    b.label(
        slot,
        spec.art,
        Label::Insert,
        0.0062,
        MEDAL_SLOT_POS + Vec3::new(0.0, 0.0024, 0.031),
        true,
    );
}

/// 메달 받침과 배출구.
fn tray(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let tray = b.group(root, "MedalTray", Transform::IDENTITY);
    let depth = TRAY_FRONT_Z - FRONT_Z;
    let mid_z = (TRAY_FRONT_Z + FRONT_Z) / 2.0;
    b.cuboid(
        tray,
        "TrayFloor",
        Vec3::new(TRAY_HALF_WIDTH * 2.0, 0.006, depth),
        Vec3::new(0.0, TRAY_FLOOR_Y - 0.003, mid_z),
        &lib.stainless,
    );
    b.cuboid(
        tray,
        "TrayLip",
        Vec3::new(TRAY_HALF_WIDTH * 2.0 + 0.012, TRAY_LIP_HEIGHT, 0.006),
        Vec3::new(
            0.0,
            TRAY_FLOOR_Y - 0.006 + TRAY_LIP_HEIGHT / 2.0,
            TRAY_FRONT_Z + 0.003,
        ),
        &lib.chrome,
    );
    for sign in [-1.0, 1.0] {
        b.cuboid(
            tray,
            "TraySide",
            Vec3::new(0.006, TRAY_LIP_HEIGHT, depth),
            Vec3::new(
                sign * (TRAY_HALF_WIDTH + 0.003),
                TRAY_FLOOR_Y - 0.006 + TRAY_LIP_HEIGHT / 2.0,
                mid_z,
            ),
            &lib.chrome,
        );
    }
    b.cuboid(
        tray,
        "ChuteOpening",
        Vec3::new(0.120, 0.034, 0.003),
        Vec3::new(CHUTE_POS.x, CHUTE_POS.y, FRONT_Z + 0.0005),
        &lib.black_matte,
    );
    b.cuboid(
        tray,
        "ChuteHood",
        Vec3::new(0.132, 0.006, 0.020),
        Vec3::new(CHUTE_POS.x, CHUTE_POS.y + 0.020, FRONT_Z + 0.010),
        &lib.chrome,
    );
}

/// 외곽 LED 띠.
fn leds(b: &mut Builder, root: Entity, _spec: &CabinetSpec) {
    let panel = b.group(root, "LEDPanel", Transform::IDENTITY);
    b.commands.entity(panel).insert(ShellPart);
    let bulb = b.meshes.add(Sphere::new(LED_RADIUS));
    let spawn = |b: &mut Builder, strip: LedStrip, index: u8, count: u8, position: Vec3| {
        let material = b.lamp_material(Color::srgb(0.12, 0.10, 0.07));
        b.commands.spawn((
            Name::new("Led"),
            Mesh3d(bulb.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            ChildOf(panel),
            Lamp::new(LampKind::Led {
                strip,
                index,
                count,
            }),
        ));
    };
    for index in 0..LED_SIDE_COUNT {
        let y = led_side_y(index);
        // 선반이 가리는 높이는 건너뛴다.
        if y > DECK_BOTTOM - 0.012 && y < DECK_TOP + 0.012 {
            continue;
        }
        spawn(
            b,
            LedStrip::Left,
            index,
            LED_SIDE_COUNT,
            Vec3::new(-LED_SIDE_X, y, FRONT_Z + 0.001),
        );
        spawn(
            b,
            LedStrip::Right,
            index,
            LED_SIDE_COUNT,
            Vec3::new(LED_SIDE_X, y, FRONT_Z + 0.001),
        );
    }
    for index in 0..LED_TOP_COUNT {
        spawn(
            b,
            LedStrip::Top,
            index,
            LED_TOP_COUNT,
            Vec3::new(
                led_top_x(index),
                CABINET_TOP + CROWN_HEIGHT / 2.0,
                FRONT_Z + 0.001,
            ),
        );
    }
}

/// 내부 구조물 (내부 관찰 모드에서 보인다).
fn interior(b: &mut Builder, root: Entity, spec: &CabinetSpec) {
    let lib = spec.library;
    let inside = b.group(root, "Interior", Transform::IDENTITY);
    let floor = CABINET_BOTTOM + WALL;
    // 메달 호퍼
    b.cuboid(
        inside,
        "HopperBucket",
        Vec3::new(0.20, 0.13, 0.17),
        Vec3::new(0.07, floor + 0.065, -0.03),
        &lib.stainless,
    );
    b.mesh(
        inside,
        "HopperDisc",
        Cylinder::new(0.06, 0.012),
        &lib.motor_black,
        Transform::from_xyz(0.07, floor + 0.136, -0.03),
    );
    b.cuboid(
        inside,
        "HopperChute",
        Vec3::new(0.11, 0.012, 0.16),
        Vec3::new(0.0, CHUTE_POS.y - 0.01, 0.09),
        &lib.stainless,
    );
    // 전원부
    b.cuboid(
        inside,
        "PowerSupply",
        Vec3::new(0.12, 0.09, 0.15),
        Vec3::new(-0.16, floor + 0.045, -0.06),
        &lib.motor_black,
    );
    b.cuboid(
        inside,
        "PowerSupplyFins",
        Vec3::new(0.10, 0.004, 0.13),
        Vec3::new(-0.16, floor + 0.092, -0.06),
        &lib.copper,
    );
    // 주 기판
    b.cuboid(
        inside,
        "MainBoard",
        Vec3::new(0.30, 0.18, 0.004),
        Vec3::new(0.0, 0.66, -CABINET_DEPTH / 2.0 + WALL + 0.012),
        &lib.pcb_green,
    );
    for (x, y) in [(-0.08, 0.68), (0.04, 0.63), (0.1, 0.70)] {
        b.cuboid(
            inside,
            "Chip",
            Vec3::new(0.035, 0.035, 0.005),
            Vec3::new(x, y, -CABINET_DEPTH / 2.0 + WALL + 0.016),
            &lib.motor_black,
        );
    }
    // 스피커 뒤쪽 유닛
    for sign in [-1.0, 1.0] {
        b.mesh(
            inside,
            "SpeakerMagnet",
            Cylinder::new(0.022, 0.03),
            &lib.motor_black,
            Transform::from_xyz(
                sign * SPEAKER_X,
                (TOP_PANEL_TOP + TOP_PANEL_BOTTOM) / 2.0,
                FRONT_Z - 0.03,
            )
            .with_rotation(axis_z()),
        );
    }
}
