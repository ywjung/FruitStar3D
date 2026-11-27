//! 카메라 리그: 정면 플레이, 좌우 회전, 확대, 내부 관찰.

use bevy::anti_alias::smaa::Smaa;
use bevy::camera::Hdr;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::picking::mesh_picking::MeshPickingCamera;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;
use fruit_star_machine::layout::{REEL_AXIS_Z, REEL_CENTER_Y, REEL_RADIUS};
use fruit_star_machine::{
    AppCommand, AppState, CameraControl, MainCamera, OffscreenTarget, Settings, ViewMode,
};

use crate::lighting::EnvironmentMap;

/// 주시점을 중심으로 한 궤도 카메라 자세.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitPose {
    pub target: Vec3,
    /// 좌우 회전 (라디안). 0이면 정면, 양수면 플레이어 기준 오른쪽에서 본다.
    pub yaw: f32,
    /// 상하 회전 (라디안). 양수면 위에서 내려다본다.
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitPose {
    pub const fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            target,
            yaw,
            pitch,
            distance,
        }
    }

    /// 카메라 위치.
    pub fn eye(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        self.target + self.distance * Vec3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch)
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.target, Vec3::Y)
    }

    pub fn lerp(&self, other: &OrbitPose, t: f32) -> OrbitPose {
        OrbitPose {
            target: self.target.lerp(other.target, t),
            yaw: self.yaw + (other.yaw - self.yaw) * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
            distance: self.distance + (other.distance - self.distance) * t,
        }
    }

    /// 조작 한계 안으로 값을 제한한다.
    pub fn clamped(mut self) -> OrbitPose {
        // 옆으로 너무 돌면 먼 거리(세로 화면)에서 카메라가 뒷벽 뒤로 넘어간다.
        self.yaw = self.yaw.clamp(-MAX_YAW, MAX_YAW);
        self.pitch = self.pitch.clamp(-0.12, 1.25);
        self.distance = self.distance.clamp(0.42, 3.2);
        self
    }
}

const FRONT_TARGET: Vec3 = Vec3::new(0.0, 0.900, 0.10);
/// 플레이 시점의 주시점: 조작부 선반부터 간판까지가 화면에 들어오고,
/// 정지 버튼이 화면 아래쪽 안내 문구에 가리지 않는 높이.
const PLAY_TARGET: Vec3 = Vec3::new(0.0, 0.960, 0.14);
const REEL_TARGET: Vec3 = Vec3::new(0.0, REEL_CENTER_Y - 0.01, REEL_AXIS_Z + REEL_RADIUS);
const GALLERY_POSE: OrbitPose = OrbitPose::new(Vec3::new(0.0, 0.90, 0.05), 0.0, 0.10, 2.20);
// 메뉴의 왼쪽 인쇄 패널 옆에 캐비닛을 전시한다. 갤러리에서는 다시 중앙에 둔다.
const MENU_POSE: OrbitPose = OrbitPose::new(Vec3::new(-0.34, 0.88, 0.05), 0.30, 0.10, 2.30);

/// 보기 방식별 카메라 프리셋. V 키로 순환한다.
pub fn presets(view: ViewMode) -> &'static [OrbitPose] {
    const PLAY: [OrbitPose; 5] = [
        // 기본: 릴과 조작부가 화면을 채우는 플레이 시점
        OrbitPose::new(PLAY_TARGET, 0.0, 0.10, 1.14),
        // 게임기 전체
        OrbitPose::new(FRONT_TARGET, 0.0, 0.085, 1.98),
        OrbitPose::new(FRONT_TARGET, -0.55, 0.13, 2.10),
        OrbitPose::new(FRONT_TARGET, 0.55, 0.13, 2.10),
        // 릴 확대
        OrbitPose::new(REEL_TARGET, 0.0, 0.02, 0.62),
    ];
    const MECHANICAL: [OrbitPose; 4] = [
        OrbitPose::new(Vec3::new(0.0, 0.88, 0.0), 0.62, 0.30, 1.50),
        OrbitPose::new(Vec3::new(0.0, 0.88, 0.0), -0.62, 0.30, 1.50),
        OrbitPose::new(Vec3::new(0.0, REEL_CENTER_Y, REEL_AXIS_Z), 0.95, 0.18, 0.62),
        OrbitPose::new(Vec3::new(0.0, 0.80, 0.0), 0.0, 1.05, 1.45),
    ];
    const GALLERY: [OrbitPose; 3] = [
        GALLERY_POSE,
        OrbitPose::new(FRONT_TARGET, -0.9, 0.2, 1.7),
        OrbitPose::new(FRONT_TARGET, 0.9, 0.2, 1.7),
    ];
    match view {
        ViewMode::Play => &PLAY,
        ViewMode::Mechanical => &MECHANICAL,
        ViewMode::Gallery => &GALLERY,
    }
}

/// 카메라 상태.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CameraRig {
    pub pose: OrbitPose,
    from: OrbitPose,
    to: OrbitPose,
    elapsed: f32,
    duration: f32,
    /// 현재 프리셋 번호.
    pub preset: usize,
    /// 자동으로 천천히 도는 중인지 여부 (메뉴·갤러리).
    auto_orbit: bool,
    auto_phase: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            pose: MENU_POSE,
            from: MENU_POSE,
            to: MENU_POSE,
            elapsed: 0.0,
            duration: 0.0,
            preset: 0,
            auto_orbit: true,
            auto_phase: 0.0,
        }
    }
}

impl CameraRig {
    /// `to` 자세로 `duration`초 동안 전환한다.
    pub fn transition_to(&mut self, to: OrbitPose, duration: f32) {
        self.from = self.pose;
        self.to = to;
        self.elapsed = 0.0;
        self.duration = duration.max(0.0);
    }

    pub fn is_transitioning(&self) -> bool {
        self.elapsed < self.duration
    }

    fn advance(&mut self, dt: f32) {
        if self.is_transitioning() {
            self.elapsed = (self.elapsed + dt).min(self.duration);
            let t = if self.duration > 0.0 {
                self.elapsed / self.duration
            } else {
                1.0
            };
            // smoothstep: 출발과 도착에서 속도가 0
            let eased = t * t * (3.0 - 2.0 * t);
            self.pose = self.from.lerp(&self.to, eased);
        } else if self.auto_orbit {
            self.auto_phase += dt;
            let mut pose = self.to;
            pose.yaw += (self.auto_phase * 0.16).sin() * 0.55;
            pose.pitch += (self.auto_phase * 0.11).sin() * 0.04;
            self.pose = pose;
        } else {
            self.pose = self.to;
        }
    }
}

pub(crate) fn spawn_camera(
    mut commands: Commands,
    settings: Res<Settings>,
    environment: Res<EnvironmentMap>,
    offscreen: Option<Res<OffscreenTarget>>,
) {
    let graphics = &settings.config.graphics;
    let rig = CameraRig::default();
    let mut camera = commands.spawn((
        MainCamera,
        Name::new("MainCamera"),
        Camera3d::default(),
        Hdr,
        Msaa::Off,
        Projection::Perspective(PerspectiveProjection {
            fov: graphics.camera.fov_degrees.to_radians(),
            near: 0.05,
            far: 60.0,
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Smaa::default(),
        MeshPickingCamera,
        GeneratedEnvironmentMapLight {
            environment_map: environment.0.clone(),
            intensity: 900.0,
            ..default()
        },
        rig.pose.transform(),
    ));
    if let Some(target) = offscreen {
        // 화면 UI도 이 카메라(화면 밖 이미지)에 그리게 한다.
        camera.insert((
            RenderTarget::Image(target.0.clone().into()),
            bevy::ui::IsDefaultUiCamera,
        ));
    }
    if graphics.render.bloom {
        camera.insert(Bloom {
            intensity: 0.12,
            ..Bloom::NATURAL
        });
    }
    if graphics.render.ambient_occlusion {
        camera.insert(ScreenSpaceAmbientOcclusion::default());
    }
    if graphics.render.motion_blur {
        camera.insert(MotionBlur {
            shutter_angle: 0.5,
            samples: 3,
        });
    }
    commands.insert_resource(rig);
}

/// 좌우 회전 한도 (라디안, 약 72°).
pub const MAX_YAW: f32 = 1.25;

/// 세로로 긴 화면(휴대폰)에서도 게임기 폭이 잘리지 않도록 프리셋의 거리를 늘린다.
///
/// 가로 화면에서는 그대로 돌려준다. 릴 확대처럼 가까운 프리셋은 표시창 폭을,
/// 나머지는 게임기 앞면 폭을 화면 안에 넣는다.
pub fn fit_to_aspect(pose: OrbitPose, aspect: f32, fov_y: f32) -> OrbitPose {
    if !(aspect > 0.0 && aspect < 1.0) {
        return pose;
    }
    let width = if pose.distance < 0.8 { 0.36 } else { 0.50 };
    let needed = width / (2.0 * (fov_y / 2.0).tan() * aspect);
    let mut fitted = pose;
    fitted.distance = fitted.distance.max(needed);
    fitted.clamped()
}

/// 주 카메라의 가로세로 비. 아직 크기가 정해지지 않았으면 `None`.
fn camera_aspect(camera: &Camera) -> Option<f32> {
    let size = camera.logical_viewport_size()?;
    (size.y > 0.0).then(|| size.x / size.y)
}

/// 화면 상태나 보기 방식이 바뀌면 그에 맞는 프리셋으로 전환한다.
pub(crate) fn follow_view_mode(
    state: Res<State<AppState>>,
    view: Res<ViewMode>,
    settings: Res<Settings>,
    camera: Single<&Camera, With<MainCamera>>,
    mut rig: ResMut<CameraRig>,
) {
    if !state.is_changed() && !view.is_changed() {
        return;
    }
    let duration = settings.config.graphics.camera.transition_ms as f32 / 1000.0;
    let in_game = *state.get() == AppState::InGame;
    rig.preset = 0;
    rig.auto_orbit = !in_game || *view == ViewMode::Gallery;
    let target = if in_game {
        presets(*view)[0]
    } else {
        MENU_POSE
    };
    let fov = settings.config.graphics.camera.fov_degrees.to_radians();
    let target = camera_aspect(&camera).map_or(target, |aspect| fit_to_aspect(target, aspect, fov));
    rig.transition_to(target, duration);
}

pub(crate) fn control_camera(
    time: Res<Time>,
    state: Res<State<AppState>>,
    view: Res<ViewMode>,
    settings: Res<Settings>,
    mut commands: MessageReader<AppCommand>,
    mut controls: MessageReader<CameraControl>,
    mut rig: ResMut<CameraRig>,
    mut camera: Single<(&mut Transform, &Camera), With<MainCamera>>,
) {
    let in_game = *state.get() == AppState::InGame;
    let fov = settings.config.graphics.camera.fov_degrees.to_radians();
    let aspect = camera_aspect(camera.1);
    for command in commands.read() {
        if *command == AppCommand::CycleCamera && in_game {
            let list = presets(*view);
            rig.preset = (rig.preset + 1) % list.len();
            let duration = settings.config.graphics.camera.transition_ms as f32 / 1000.0;
            rig.auto_orbit = *view == ViewMode::Gallery && rig.preset == 0;
            let target = list[rig.preset];
            let target = aspect.map_or(target, |aspect| fit_to_aspect(target, aspect, fov));
            rig.transition_to(target, duration);
        }
    }
    for control in controls.read() {
        if !in_game {
            continue;
        }
        // 사용자가 직접 움직이면 전환과 자동 회전을 멈추고 그 자리에서 조작한다.
        let mut pose = rig.pose;
        pose.yaw -= control.orbit.x;
        pose.pitch += control.orbit.y;
        pose.distance *= (1.0 - control.zoom).clamp(0.5, 1.5);
        let pose = pose.clamped();
        rig.auto_orbit = false;
        rig.from = pose;
        rig.to = pose;
        rig.pose = pose;
        rig.elapsed = rig.duration;
    }
    rig.advance(time.delta_secs());
    *camera.0 = rig.pose.transform();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 뒷벽 앞면의 Z 좌표 (lighting.rs의 BackWall: 중심 wall_z − 0.05, 두께 0.1).
    const BACK_WALL_FRONT_Z: f32 = -fruit_star_machine::layout::CABINET_DEPTH / 2.0 - 0.06;

    #[test]
    fn manual_orbit_never_takes_the_camera_behind_the_back_wall() {
        let fov = 32f32.to_radians();
        // 세로 화면에 맞춰 멀어진 거리와 최대 줌아웃 거리 모두에서 확인한다
        for base in [1.14, 3.2] {
            for aspect in [402.0 / 874.0, 16.0 / 9.0] {
                for yaw in [-10.0, -2.0, 2.0, 10.0] {
                    for pitch in [-1.0, 0.0, 2.0] {
                        let start = fit_to_aspect(
                            OrbitPose::new(Vec3::new(0.0, 0.96, 0.14), 0.0, 0.1, base),
                            aspect,
                            fov,
                        );
                        let mut pose = start;
                        pose.yaw += yaw;
                        pose.pitch += pitch;
                        let eye = pose.clamped().eye();
                        assert!(
                            eye.z > BACK_WALL_FRONT_Z + 0.1,
                            "yaw {yaw}, pitch {pitch}, distance {}: eye {eye}",
                            start.distance
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn landscape_presets_are_unchanged() {
        let pose = OrbitPose::new(Vec3::ZERO, 0.0, 0.1, 1.2);
        assert_eq!(fit_to_aspect(pose, 16.0 / 9.0, 0.56), pose);
        assert_eq!(fit_to_aspect(pose, 1.0, 0.56), pose);
    }

    #[test]
    fn portrait_presets_back_off_until_the_cabinet_fits() {
        let fov = 32f32.to_radians();
        let aspect = 402.0 / 874.0;
        let fitted = fit_to_aspect(OrbitPose::new(Vec3::ZERO, 0.0, 0.1, 1.14), aspect, fov);
        let visible_width = 2.0 * fitted.distance * (fov / 2.0).tan() * aspect;
        assert!((visible_width - 0.50).abs() < 1e-3, "{visible_width}");
        // 이미 충분히 먼 프리셋은 그대로 둔다
        let far = OrbitPose::new(Vec3::ZERO, 0.0, 0.1, 3.0);
        assert_eq!(fit_to_aspect(far, aspect, fov).distance, 3.0);
    }
}
