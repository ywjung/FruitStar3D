//! 기기 상태(열 상태·배터리) 측정과 자동 실행 중 화면 자동 잠금 끄기.
//!
//! 실제 값은 iOS에서만 읽는다 (`NSProcessInfo.thermalState`, `UIDevice.batteryLevel`).
//! 다른 플랫폼에서는 아무것도 읽지 않으므로 요약도 나오지 않는다.
//! iOS 호출은 objc2 크레이트의 안전한(safe) 메서드만 쓰며, 메인 스레드에서만 부른다.

/// 기기 열 상태. iOS `NSProcessInfoThermalState`의 네 단계와 같다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thermal {
    Nominal,
    Fair,
    Serious,
    Critical,
}

impl Thermal {
    const ALL: [Self; 4] = [Self::Nominal, Self::Fair, Self::Serious, Self::Critical];

    /// 운영체제가 준 원시 값. 모르는 값은 가장 가까운 단계로 본다.
    #[cfg_attr(not(any(target_os = "ios", test)), allow(dead_code))]
    pub fn from_raw(raw: isize) -> Self {
        match raw {
            i if i <= 0 => Self::Nominal,
            1 => Self::Fair,
            2 => Self::Serious,
            _ => Self::Critical,
        }
    }

    /// Apple이 작업을 줄이라고 권하는 단계("높음" 이상)인지.
    pub fn is_hot(self) -> bool {
        matches!(self, Self::Serious | Self::Critical)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Nominal => "정상",
            Self::Fair => "약간 높음",
            Self::Serious => "높음",
            Self::Critical => "위험",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Nominal => 0,
            Self::Fair => 1,
            Self::Serious => 2,
            Self::Critical => 3,
        }
    }
}

/// 한 번 읽은 기기 상태.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceSample {
    pub thermal: Thermal,
    /// 배터리 잔량 (0.0 ~ 1.0). 알 수 없으면 `None`.
    pub battery: Option<f32>,
    /// 전원에 연결되어 충전 중이거나 완충 상태인지. 알 수 없으면 `None`.
    pub plugged: Option<bool>,
}

/// 측정 구간 동안 열 상태별 머문 시간과 배터리 변화를 모은다.
#[derive(Debug, Clone, Default)]
pub struct DeviceTally {
    seconds: [f32; 4],
    first_battery: Option<f32>,
    last_battery: Option<f32>,
    peak: Option<Thermal>,
    plugged: Option<bool>,
}

impl DeviceTally {
    /// `sample`이 직전 `seconds`초 동안의 상태라고 보고 더한다.
    pub fn record(&mut self, sample: DeviceSample, seconds: f32) {
        if let Some(slot) = self.seconds.get_mut(sample.thermal.index()) {
            *slot += seconds.max(0.0);
        }
        self.peak = Some(match self.peak {
            Some(peak) if peak.index() >= sample.thermal.index() => peak,
            _ => sample.thermal,
        });
        if let Some(level) = sample.battery {
            self.first_battery.get_or_insert(level);
            self.last_battery = Some(level);
        }
        // 한 번이라도 전원에 연결돼 있었으면 연결로 본다 (배터리 감소를 그대로 읽으면 안 되므로).
        if let Some(plugged) = sample.plugged {
            self.plugged = Some(self.plugged.unwrap_or(false) || plugged);
        }
    }

    /// 한 줄 요약. 기록이 없으면 `None`.
    pub fn summary(&self) -> Option<String> {
        let peak = self.peak?;
        let spans: Vec<String> = Thermal::ALL
            .iter()
            .filter_map(|&state| {
                let secs = self.seconds.get(state.index()).copied().unwrap_or(0.0);
                (secs > 0.0).then(|| format!("{} {:.0}초", state.label(), secs))
            })
            .collect();
        let battery = match (self.first_battery, self.last_battery) {
            (Some(first), Some(last)) => {
                format!("배터리 {:.0}% → {:.0}%", first * 100.0, last * 100.0)
            }
            _ => "배터리 알 수 없음".to_owned(),
        };
        let power = match self.plugged {
            Some(true) => " (전원 연결)",
            Some(false) => " (배터리 사용)",
            None => "",
        };
        Some(format!(
            "기기 상태: 열 {} (최고 {}), {}{}",
            spans.join(", "),
            peak.label(),
            battery,
            power
        ))
    }
}

/// 측정 결과 한 줄을 남긴다. iOS의 로그는 시스템 로그로만 가므로, `devicectl --console`로
/// 받을 수 있게 표준 오류에도 쓴다.
pub fn report(line: &str) {
    bevy::log::info!("{line}");
    #[cfg(target_os = "ios")]
    eprintln!("{line}");
}

/// 지금 기기 열 상태. iOS에서만 읽으며 다른 플랫폼에서는 `None`.
/// `NSProcessInfo`는 스레드에 안전하므로 어느 스레드에서 불러도 된다.
#[cfg(target_os = "ios")]
pub fn thermal() -> Option<Thermal> {
    Some(Thermal::from_raw(
        objc2_foundation::NSProcessInfo::processInfo()
            .thermalState()
            .0,
    ))
}

#[cfg(not(target_os = "ios"))]
pub fn thermal() -> Option<Thermal> {
    None
}

/// 지금 기기 상태를 읽는다. iOS 메인 스레드가 아니면 `None`.
#[cfg(target_os = "ios")]
pub fn read() -> Option<DeviceSample> {
    let mtm = objc2::MainThreadMarker::new()?;
    let thermal = thermal()?;
    let device = objc2_ui_kit::UIDevice::currentDevice(mtm);
    device.setBatteryMonitoringEnabled(true);
    let level = device.batteryLevel();
    // 잔량을 모르면 iOS는 -1을 준다.
    let battery = (0.0..=1.0).contains(&level).then_some(level);
    let state = device.batteryState();
    let plugged = (state != objc2_ui_kit::UIDeviceBatteryState::Unknown)
        .then(|| state != objc2_ui_kit::UIDeviceBatteryState::Unplugged);
    Some(DeviceSample {
        thermal,
        battery,
        plugged,
    })
}

#[cfg(not(target_os = "ios"))]
pub fn read() -> Option<DeviceSample> {
    None
}

/// 화면 자동 잠금을 끄거나 켠다. iOS 메인 스레드에서만 동작한다.
#[cfg(target_os = "ios")]
pub fn keep_screen_on(on: bool) {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        objc2_ui_kit::UIApplication::sharedApplication(mtm).setIdleTimerDisabled(on);
    }
}

#[cfg(not(target_os = "ios"))]
pub fn keep_screen_on(_on: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(thermal: Thermal, battery: Option<f32>) -> DeviceSample {
        DeviceSample {
            thermal,
            battery,
            plugged: None,
        }
    }

    #[test]
    fn raw_thermal_values_map_to_four_levels() {
        assert_eq!(Thermal::from_raw(0), Thermal::Nominal);
        assert_eq!(Thermal::from_raw(1), Thermal::Fair);
        assert_eq!(Thermal::from_raw(2), Thermal::Serious);
        assert_eq!(Thermal::from_raw(3), Thermal::Critical);
        assert_eq!(Thermal::from_raw(-1), Thermal::Nominal);
        assert_eq!(Thermal::from_raw(9), Thermal::Critical);
    }

    #[test]
    fn only_serious_and_critical_count_as_hot() {
        assert!(!Thermal::Nominal.is_hot());
        assert!(!Thermal::Fair.is_hot());
        assert!(Thermal::Serious.is_hot());
        assert!(Thermal::Critical.is_hot());
    }

    #[test]
    fn empty_tally_has_no_summary() {
        assert_eq!(DeviceTally::default().summary(), None);
    }

    #[test]
    fn summary_lists_time_per_state_peak_and_battery() {
        let mut tally = DeviceTally::default();
        tally.record(sample(Thermal::Nominal, Some(0.86)), 1.0);
        tally.record(sample(Thermal::Nominal, Some(0.85)), 119.0);
        tally.record(sample(Thermal::Fair, Some(0.82)), 60.0);
        tally.record(sample(Thermal::Nominal, Some(0.81)), 0.0);
        assert_eq!(
            tally.summary().as_deref(),
            Some("기기 상태: 열 정상 120초, 약간 높음 60초 (최고 약간 높음), 배터리 86% → 81%")
        );
    }

    #[test]
    fn any_plugged_sample_marks_the_run_as_on_power() {
        let mut tally = DeviceTally::default();
        let mut on_battery = sample(Thermal::Nominal, Some(0.50));
        on_battery.plugged = Some(false);
        tally.record(on_battery, 10.0);
        assert!(
            tally
                .summary()
                .is_some_and(|s| s.ends_with("(배터리 사용)"))
        );
        let mut charging = sample(Thermal::Nominal, Some(0.51));
        charging.plugged = Some(true);
        tally.record(charging, 10.0);
        tally.record(on_battery, 10.0);
        assert!(tally.summary().is_some_and(|s| s.ends_with("(전원 연결)")));
    }

    #[test]
    fn unknown_battery_and_negative_time_are_handled() {
        let mut tally = DeviceTally::default();
        tally.record(sample(Thermal::Serious, None), -3.0);
        tally.record(sample(Thermal::Fair, None), 5.0);
        assert_eq!(
            tally.summary().as_deref(),
            Some("기기 상태: 열 약간 높음 5초 (최고 높음), 배터리 알 수 없음")
        );
    }
}
