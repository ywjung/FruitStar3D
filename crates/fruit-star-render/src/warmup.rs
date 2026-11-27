//! 로딩 화면 뒤에서 렌더 파이프라인을 미리 컴파일한다.
//!
//! macOS·iOS에서 Bevy는 렌더 파이프라인을 프레임 안에서 동기식으로 만든다. 셰이더 캐시가 빈 첫
//! 실행에서는 게임기 부품이 처음 그려지는 프레임이 수 초 멈춘다. 그래서 장면을 로딩 상태에서
//! 미리 만들고, 평소 숨어 있는 부품(당첨 라인, 옆자리 게임기)과 나중에 생기는 메달까지 불투명한
//! 로딩 화면 뒤에서 한 번씩 그린다. 대기 중인 파이프라인이 없어지면 메뉴로 넘어간다.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use bevy::render::render_resource::PipelineCache;
use fruit_star_machine::{AppState, MainCamera};

use crate::effects::MedalMesh;
use crate::materials::MaterialLibrary;

/// 렌더 월드가 기록하는 파이프라인 현황. 상위 32비트는 전체 수, 하위 32비트는 대기 수다.
#[derive(Resource, Debug, Clone, Default)]
pub struct PipelineProgress(Arc<AtomicU64>);

/// 아직 렌더 월드가 한 번도 기록하지 않았음을 뜻한다.
const UNREPORTED: u64 = u64::MAX;

impl PipelineProgress {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(UNREPORTED)))
    }

    fn store(&self, total: usize, waiting: usize) {
        let total = u64::try_from(total)
            .unwrap_or(u64::from(u32::MAX))
            .min(u64::from(u32::MAX));
        let waiting = u64::try_from(waiting)
            .unwrap_or(u64::from(u32::MAX))
            .min(u64::from(u32::MAX));
        self.0.store((total << 32) | waiting, Ordering::Relaxed);
    }

    /// (전체, 대기) 수. 렌더 월드가 없거나 아직 기록하지 않았으면 `None`.
    pub fn read(&self) -> Option<(u32, u32)> {
        let packed = self.0.load(Ordering::Relaxed);
        (packed != UNREPORTED).then_some(((packed >> 32) as u32, (packed & 0xFFFF_FFFF) as u32))
    }
}

/// 렌더 월드: 매 프레임 렌더링 뒤에 파이프라인 현황을 기록한다.
pub(crate) fn report_pipelines(cache: Res<PipelineCache>, progress: Res<PipelineProgress>) {
    progress.store(cache.pipelines().count(), cache.waiting_pipelines().count());
}

/// 예열 진행 상태. 이 리소스가 있는 동안은 로딩 화면에 머문다.
#[derive(Resource, Debug, Default)]
pub struct Warmup {
    started: Option<Instant>,
    frames: u32,
    baseline: Option<u32>,
    last_total: u32,
    stable_frames: u32,
    revealed: Vec<Entity>,
    unculled: Vec<Entity>,
    probe: Option<Entity>,
}

/// 장면을 만든 뒤 최소한 이만큼 프레임을 그린다 (메시·재질이 GPU에 올라가는 시간).
const MIN_FRAMES: u32 = 6;
/// 대기 중인 파이프라인 없이 전체 수가 이만큼 연속으로 그대로면 끝난 것으로 본다.
const STABLE_FRAMES: u32 = 4;
/// 어떤 이유로든 끝나지 않으면 이 시간 뒤에 메뉴로 넘어간다.
const TIMEOUT: Duration = Duration::from_secs(30);

/// 예열 판정. 렌더 월드 보고가 없으면(헤드리스) 바로 끝난다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Continue,
    Done,
}

fn judge(warmup: &mut Warmup, report: Option<(u32, u32)>, elapsed: Duration) -> Verdict {
    if elapsed >= TIMEOUT {
        return Verdict::Done;
    }
    let Some((total, waiting)) = report else {
        return if warmup.frames >= MIN_FRAMES {
            Verdict::Done
        } else {
            Verdict::Continue
        };
    };
    let baseline = *warmup.baseline.get_or_insert(total);
    if waiting == 0 && total == warmup.last_total && total > baseline {
        warmup.stable_frames += 1;
    } else {
        warmup.stable_frames = 0;
    }
    warmup.last_total = total;
    if warmup.frames >= MIN_FRAMES && warmup.stable_frames >= STABLE_FRAMES {
        Verdict::Done
    } else {
        Verdict::Continue
    }
}

/// 로딩 상태에서 예열을 진행하고, 끝나면 원래 상태로 되돌린 뒤 메뉴로 넘어간다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_warmup(
    mut commands: Commands,
    warmup: Option<ResMut<Warmup>>,
    progress: Option<Res<PipelineProgress>>,
    library: Option<Res<MaterialLibrary>>,
    medal: Option<Res<MedalMesh>>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut hidden: Query<(Entity, &mut Visibility), Without<Node>>,
    meshes: Query<Entity, (With<Mesh3d>, Without<NoFrustumCulling>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(mut warmup) = warmup else {
        return;
    };
    // 장면(재질 목록)이 아직 만들어지지 않았다.
    let (Some(library), Some(medal)) = (library, medal) else {
        return;
    };

    if warmup.started.is_none() {
        // 숨은 부품을 드러내고, 시야 밖 부품도 그리도록 시야 절두체 검사를 끈다.
        for (entity, mut visibility) in &mut hidden {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;
                warmup.revealed.push(entity);
            }
        }
        for entity in &meshes {
            commands.entity(entity).insert(NoFrustumCulling);
            warmup.unculled.push(entity);
        }
        // 메달은 지급·투입 때 처음 생기므로 카메라 앞에 하나 놓아 둔다.
        let at = camera
            .iter()
            .next()
            .map(|camera| camera.translation() + camera.forward() * 0.6)
            .unwrap_or(Vec3::new(0.0, 1.0, 0.5));
        let probe = commands
            .spawn((
                Name::new("WarmupMedal"),
                Mesh3d(medal.0.clone()),
                MeshMaterial3d(library.medal.clone()),
                Transform::from_translation(at),
            ))
            .id();
        warmup.probe = Some(probe);
        warmup.started = Some(Instant::now());
    }
    let started = warmup.started.unwrap_or_else(Instant::now);
    warmup.frames += 1;

    let report = progress.as_ref().and_then(|progress| progress.read());
    if judge(&mut warmup, report, started.elapsed()) == Verdict::Continue {
        return;
    }

    for entity in warmup.revealed.drain(..) {
        if let Ok((_, mut visibility)) = hidden.get_mut(entity) {
            *visibility = Visibility::Hidden;
        }
    }
    for entity in warmup.unculled.drain(..) {
        if let Ok(mut entity) = commands.get_entity(entity) {
            entity.remove::<NoFrustumCulling>();
        }
    }
    if let Some(probe) = warmup.probe.take()
        && let Ok(mut probe) = commands.get_entity(probe)
    {
        probe.despawn();
    }
    info!(
        "렌더 예열 완료: {:.0} ms, {}프레임, 파이프라인 {}",
        started.elapsed().as_secs_f64() * 1000.0,
        warmup.frames,
        report.map_or_else(|| "보고 없음".to_owned(), |(total, _)| total.to_string())
    );
    commands.remove_resource::<Warmup>();
    next.set(AppState::MainMenu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_round_trips_and_starts_unreported() {
        let progress = PipelineProgress::new();
        assert_eq!(progress.read(), None);
        progress.store(67, 3);
        assert_eq!(progress.read(), Some((67, 3)));
    }

    #[test]
    fn finishes_without_render_world_after_min_frames() {
        let mut warmup = Warmup::default();
        for frame in 1..MIN_FRAMES {
            warmup.frames = frame;
            assert_eq!(judge(&mut warmup, None, Duration::ZERO), Verdict::Continue);
        }
        warmup.frames = MIN_FRAMES;
        assert_eq!(judge(&mut warmup, None, Duration::ZERO), Verdict::Done);
    }

    #[test]
    fn waits_for_scene_pipelines_to_appear_and_settle() {
        let mut warmup = Warmup::default();
        let step = |warmup: &mut Warmup, total, waiting| {
            warmup.frames += 1;
            judge(warmup, Some((total, waiting)), Duration::ZERO)
        };
        // 장면 파이프라인이 아직 대기열에 오르지 않은 동안은 끝나지 않는다.
        for _ in 0..10 {
            assert_eq!(step(&mut warmup, 54, 0), Verdict::Continue);
        }
        // 새 파이프라인이 생기고 대기 중인 것이 있으면 계속 기다린다.
        assert_eq!(step(&mut warmup, 67, 2), Verdict::Continue);
        for _ in 1..STABLE_FRAMES {
            assert_eq!(step(&mut warmup, 67, 0), Verdict::Continue);
        }
        assert_eq!(step(&mut warmup, 67, 0), Verdict::Done);
    }

    #[test]
    fn gives_up_after_timeout() {
        let mut warmup = Warmup::default();
        assert_eq!(judge(&mut warmup, Some((54, 5)), TIMEOUT), Verdict::Done);
    }
}
