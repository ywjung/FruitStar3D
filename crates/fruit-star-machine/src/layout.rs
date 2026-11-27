//! 게임기 부품의 배치 (미터 단위).
//!
//! 원점은 게임기 아래 바닥의 중심이다. +X는 플레이어가 볼 때 오른쪽, +Y는 위,
//! +Z는 플레이어 쪽이다. 렌더링·물리·음향 계층이 같은 값을 공유한다.

use bevy::math::Vec3;

/// 받침대 높이 (= 캐비닛 바닥 높이).
pub const PEDESTAL_HEIGHT: f32 = 0.42;
pub const PEDESTAL_SIZE: Vec3 = Vec3::new(0.60, PEDESTAL_HEIGHT, 0.46);

/// 캐비닛 본체.
pub const CABINET_WIDTH: f32 = 0.52;
pub const CABINET_HEIGHT: f32 = 0.84;
pub const CABINET_DEPTH: f32 = 0.36;
pub const CABINET_BOTTOM: f32 = PEDESTAL_HEIGHT;
pub const CABINET_TOP: f32 = CABINET_BOTTOM + CABINET_HEIGHT;
/// 캐비닛 앞면의 Z 좌표.
pub const FRONT_Z: f32 = CABINET_DEPTH / 2.0;
/// 외장 판 두께.
pub const WALL: f32 = 0.014;

/// 릴.
pub const REEL_RADIUS: f32 = 0.135;
pub const REEL_WIDTH: f32 = 0.100;
/// 릴 사이 간격.
pub const REEL_GAP: f32 = 0.010;
/// 릴 중심 높이.
pub const REEL_CENTER_Y: f32 = 0.980;
/// 릴 축의 Z 좌표. 릴 앞면이 유리에서 조금 들어가 보이도록 한다.
pub const REEL_AXIS_Z: f32 = FRONT_Z - 0.030 - REEL_RADIUS;

/// 릴 `index`(0=왼쪽)의 중심.
pub fn reel_center(index: usize) -> Vec3 {
    let pitch = REEL_WIDTH + REEL_GAP;
    Vec3::new((index as f32 - 1.0) * pitch, REEL_CENTER_Y, REEL_AXIS_Z)
}

/// 표시창 (릴이 보이는 구멍).
pub const WINDOW_WIDTH: f32 = 3.0 * REEL_WIDTH + 2.0 * REEL_GAP + 0.010;
pub const WINDOW_HEIGHT: f32 = 0.115;

/// 릴 패널 (표시창과 표시기가 있는 검은 유리 패널).
pub const REEL_PANEL_BOTTOM: f32 = 0.846;
pub const REEL_PANEL_TOP: f32 = 1.098;

/// 상단 패널.
pub const TOP_PANEL_BOTTOM: f32 = 1.106;
pub const TOP_PANEL_TOP: f32 = 1.250;
pub const MARQUEE_WIDTH: f32 = 0.352;
/// 간판 텍스처의 가로세로 비 (2048×640).
pub const MARQUEE_ASPECT: f32 = 3.2;
pub const SPEAKER_RADIUS: f32 = 0.024;
pub const SPEAKER_X: f32 = 0.208;

/// 하단 장식 패널.
pub const BELLY_BOTTOM: f32 = 0.548;
pub const BELLY_TOP: f32 = 0.750;
pub const BELLY_WIDTH: f32 = 0.460;

/// 조작부 선반.
pub const DECK_BOTTOM: f32 = 0.762;
pub const DECK_TOP: f32 = 0.836;
/// 선반이 앞으로 튀어나온 길이.
pub const DECK_DEPTH: f32 = 0.090;
pub const DECK_FRONT_Z: f32 = FRONT_Z + DECK_DEPTH;

/// 7세그먼트 표시기 줄의 중심 높이.
pub const DISPLAY_Y: f32 = 0.878;
/// 표시기별 중심 X (CREDIT, COUNT, PAYOUT 순으로 왼쪽부터).
pub const DISPLAY_CREDIT_X: f32 = -0.140;
pub const DISPLAY_COUNT_X: f32 = 0.0;
pub const DISPLAY_PAYOUT_X: f32 = 0.150;
pub const DIGIT_HEIGHT: f32 = 0.024;
pub const DIGIT_WIDTH: f32 = 0.013;
pub const DIGIT_PITCH: f32 = 0.019;

/// STAR CHANCE 램프.
pub const CHANCE_LAMP_POS: Vec3 = Vec3::new(-0.201, 0.992, FRONT_Z - 0.004);
pub const CHANCE_LAMP_RADIUS: f32 = 0.028;

/// 정지 버튼 `index`의 중심 (선반 앞면).
pub fn stop_button_pos(index: usize) -> Vec3 {
    Vec3::new(reel_center(index).x, 0.799, DECK_FRONT_Z)
}
pub const STOP_BUTTON_RADIUS: f32 = 0.022;

/// START 레버의 회전축 위치 (선반 앞면 왼쪽).
pub const LEVER_PIVOT: Vec3 = Vec3::new(-0.208, 0.800, DECK_FRONT_Z);
pub const LEVER_LENGTH: f32 = 0.062;
pub const LEVER_KNOB_RADIUS: f32 = 0.021;

/// MAX BET 버튼 (선반 윗면 왼쪽).
pub const BET_BUTTON_POS: Vec3 = Vec3::new(-0.120, DECK_TOP, FRONT_Z + 0.048);
pub const BET_BUTTON_SIZE: Vec3 = Vec3::new(0.066, 0.010, 0.036);

/// 1 BET 버튼 (선반 윗면, MAX BET 오른쪽).
pub const BET_ONE_BUTTON_POS: Vec3 = Vec3::new(-0.045, DECK_TOP, FRONT_Z + 0.048);
pub const BET_ONE_BUTTON_SIZE: Vec3 = Vec3::new(0.036, 0.010, 0.036);

/// 상태 표시등 (표시창 오른쪽에 세로로 나란히).
pub const STATUS_LAMP_X: f32 = 0.2025;
pub const STATUS_LAMP_SIZE: Vec3 = Vec3::new(0.046, 0.0125, 0.0);
pub const STATUS_LAMP_TOP_Y: f32 = 1.012;
pub const STATUS_LAMP_PITCH: f32 = 0.019;

/// 메달 투입구 (선반 윗면 오른쪽).
pub const MEDAL_SLOT_POS: Vec3 = Vec3::new(0.175, DECK_TOP, FRONT_Z + 0.046);

/// 메달 받침.
pub const TRAY_FLOOR_Y: f32 = CABINET_BOTTOM + 0.012;
pub const TRAY_HALF_WIDTH: f32 = 0.205;
pub const TRAY_FRONT_Z: f32 = FRONT_Z + 0.125;
pub const TRAY_LIP_HEIGHT: f32 = 0.046;
/// 메달 배출구 (메달이 나오는 위치).
pub const CHUTE_POS: Vec3 = Vec3::new(0.0, CABINET_BOTTOM + 0.078, FRONT_Z + 0.006);

/// 메달.
pub const MEDAL_RADIUS: f32 = 0.0125;
pub const MEDAL_THICKNESS: f32 = 0.0018;

/// 외곽 LED.
pub const LED_RADIUS: f32 = 0.0075;
pub const LED_SIDE_COUNT: u8 = 16;
pub const LED_TOP_COUNT: u8 = 11;
pub const LED_SIDE_X: f32 = CABINET_WIDTH / 2.0 - 0.012;
pub const CROWN_HEIGHT: f32 = 0.044;

/// 데이터 카운터 (게임기 위의 집계 표시기).
pub const DATA_COUNTER_SIZE: Vec3 = Vec3::new(0.380, 0.072, 0.060);
pub const DATA_COUNTER_CENTER: Vec3 = Vec3::new(
    0.0,
    CABINET_TOP + CROWN_HEIGHT + 0.006 + 0.036,
    FRONT_Z - 0.050,
);
/// 톱 램프 반지름.
pub const TOP_LAMP_RADIUS: f32 = 0.026;

/// 릴 패널의 배당표 (표시창 위).
pub const PAYTABLE_SIZE: Vec3 = Vec3::new(0.430, 0.05375, 0.0);

/// 좌우 LED 띠의 `index`번째 알의 높이.
pub fn led_side_y(index: u8) -> f32 {
    let bottom = BELLY_BOTTOM + 0.012;
    let top = TOP_PANEL_TOP - 0.012;
    bottom + (top - bottom) * f32::from(index) / f32::from(LED_SIDE_COUNT - 1)
}

/// 상단 LED 띠의 `index`번째 알의 X 좌표.
pub fn led_top_x(index: u8) -> f32 {
    let half = CABINET_WIDTH / 2.0 - 0.045;
    -half + 2.0 * half * f32::from(index) / f32::from(LED_TOP_COUNT - 1)
}
