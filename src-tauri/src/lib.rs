mod vision;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::{fs, sync::Mutex};
use tauri::State;

// Tauri command layer: handles camera session lifecycle and delegates image processing to vision.rs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CameraDevice {
    path: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartCameraRequest {
    device: String,
    width: u32,
    height: u32,
    fps: u32,
}

#[derive(Debug, Clone)]
struct CameraConfig {
    device: String,
    width: u32,
    height: u32,
    fps: u32,
}

#[cfg(target_os = "linux")]
struct LinuxCameraSession {
    camera: rscam::Camera,
    last_frame_jpeg: Vec<u8>,
    last_polygon: Option<Vec<PolygonPoint>>,
}

struct CameraState {
    #[cfg(target_os = "linux")]
    current: Mutex<Option<LinuxCameraSession>>,
    #[cfg(not(target_os = "linux"))]
    current: Mutex<Option<CameraConfig>>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PolygonPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureFrameResponse {
    frame_data_url: String,
    polygon: Vec<PolygonPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RectifySnapshotRequest {
    frame_data_url: String,
    polygon: Vec<PolygonPoint>,
    #[serde(default = "default_post_process_mode")]
    post_process: String,
    #[serde(default = "default_denoise_strength")]
    denoise_strength: String,
}

fn default_post_process_mode() -> String {
    "sharpen".to_string()
}

fn default_denoise_strength() -> String {
    "low".to_string()
}

#[tauri::command]
fn list_cameras() -> Result<Vec<CameraDevice>, String> {
    #[cfg(target_os = "linux")]
    {
        let mut devices = Vec::new();
        let entries = fs::read_dir("/dev").map_err(|e| format!("读取 /dev 失败: {e}"))?;

        for entry in entries.flatten() {
            let node = entry.file_name().to_string_lossy().to_string();
            if !node.starts_with("video") {
                continue;
            }

            let name_path = format!("/sys/class/video4linux/{node}/name");
            let readable_name = fs::read_to_string(name_path)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| node.clone());

            devices.push(CameraDevice {
                path: format!("/dev/{node}"),
                name: readable_name,
            });
        }

        devices.sort_by(|a, b| a.path.cmp(&b.path));
        return Ok(devices);
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("当前 demo 仅实现了 Linux 摄像头采集。".into())
    }
}

#[tauri::command]
fn set_camera(state: State<CameraState>, request: StartCameraRequest) -> Result<(), String> {
    if !request.device.starts_with("/dev/video") {
        return Err("请选择 /dev/video* 设备。".into());
    }

    let config = CameraConfig {
        device: request.device,
        width: request.width.clamp(160, 3840),
        height: request.height.clamp(120, 2160),
        fps: request.fps.clamp(1, 60),
    };

    #[cfg(target_os = "linux")]
    {
        {
            let mut guard = state
                .current
                .lock()
                .map_err(|_| "摄像头状态锁失败。".to_string())?;
            *guard = None;
        }

        let session = start_camera_session_linux(&config)?;
        let mut guard = state
            .current
            .lock()
            .map_err(|_| "摄像头状态锁失败。".to_string())?;
        *guard = Some(session);
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut guard = state
            .current
            .lock()
            .map_err(|_| "摄像头状态锁失败。".to_string())?;
        *guard = Some(config);
        Ok(())
    }
}

#[tauri::command]
fn stop_camera(state: State<CameraState>) -> Result<(), String> {
    let mut guard = state
        .current
        .lock()
        .map_err(|_| "摄像头状态锁失败。".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn capture_frame(state: State<CameraState>) -> Result<CaptureFrameResponse, String> {
    #[cfg(target_os = "linux")]
    {
        let (frame_bytes, polygon) = {
            let mut guard = state
                .current
                .lock()
                .map_err(|_| "摄像头状态锁失败。".to_string())?;

            let session = guard
                .as_mut()
                .ok_or_else(|| "请先开启摄像头。".to_string())?;
            let frame = session
                .camera
                .capture()
                .map_err(|e| format!("读取画面失败: {e}"))?;
            let frame_bytes = frame.to_vec();

            if !vision::looks_like_jpeg(&frame_bytes) {
                return Err("当前仅支持 MJPG 摄像头输出，请换支持 MJPG 的分辨率/设备。".into());
            }

            session.last_frame_jpeg = frame_bytes.clone();
            let polygon = session.last_polygon.clone().unwrap_or_default();
            (frame_bytes, polygon)
        };

        let encoded = STANDARD.encode(frame_bytes);
        return Ok(CaptureFrameResponse {
            frame_data_url: format!("data:image/jpeg;base64,{encoded}"),
            polygon,
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("当前 demo 仅实现了 Linux 摄像头采集。".into())
    }
}

#[tauri::command]
fn update_polygon_detection(state: State<CameraState>) -> Result<Vec<PolygonPoint>, String> {
    #[cfg(target_os = "linux")]
    {
        let frame = {
            let guard = state
                .current
                .lock()
                .map_err(|_| "摄像头状态锁失败。".to_string())?;
            let session = guard
                .as_ref()
                .ok_or_else(|| "请先开启摄像头。".to_string())?;
            if session.last_frame_jpeg.is_empty() {
                return Ok(session.last_polygon.clone().unwrap_or_default());
            }
            session.last_frame_jpeg.clone()
        };

        let polygon = vision::detect_polygon_on_jpeg_linux(&frame).unwrap_or_default();

        let mut guard = state
            .current
            .lock()
            .map_err(|_| "摄像头状态锁失败。".to_string())?;
        if let Some(session) = guard.as_mut() {
            session.last_polygon = Some(polygon.clone());
        }

        return Ok(polygon);
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("当前 demo 仅实现了 Linux 摄像头采集。".into())
    }
}

#[tauri::command]
fn rectify_snapshot(request: RectifySnapshotRequest) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        return vision::rectify_snapshot_linux(
            &request.frame_data_url,
            &request.polygon,
            &request.post_process,
            &request.denoise_strength,
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("当前 demo 仅实现了 Linux 摄像头采集。".into())
    }
}

#[cfg(target_os = "linux")]
fn start_camera_session_linux(config: &CameraConfig) -> Result<LinuxCameraSession, String> {
    use rscam::{Config, Error, IntervalInfo};

    let mut camera = rscam::new(&config.device)
        .map_err(|e| format!("打开摄像头 {} 失败: {e}", config.device))?;

    let resolution = (config.width, config.height);
    let target_interval = (1, config.fps.max(1));
    let mut intervals = vec![target_interval];

    if let Ok(info) = camera.intervals(b"MJPG", resolution) {
        match info {
            IntervalInfo::Discretes(mut values) => {
                values.sort_by(|a, b| {
                    let da = fps_distance(*a, target_interval);
                    let db = fps_distance(*b, target_interval);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                });
                for interval in values {
                    push_interval_unique(&mut intervals, interval);
                }
            }
            IntervalInfo::Stepwise { min, max, .. } => {
                push_interval_unique(&mut intervals, min);
                push_interval_unique(&mut intervals, max);
            }
        }
    }

    for fallback in [
        (1, 60),
        (1, 50),
        (1, 40),
        (1, 30),
        (1, 25),
        (1, 20),
        (1, 15),
        (1, 10),
        (1, 5),
        (1, 1),
    ] {
        push_interval_unique(&mut intervals, fallback);
    }

    let mut last_error = String::new();
    for interval in intervals {
        let stream_config = Config {
            interval,
            resolution,
            format: b"MJPG",
            nbuffers: 8,
            ..Default::default()
        };

        match camera.start(&stream_config) {
            Ok(()) => {
                tune_camera_controls_linux(&camera);
                return Ok(LinuxCameraSession {
                    camera,
                    last_frame_jpeg: Vec::new(),
                    last_polygon: None,
                });
            }
            Err(Error::BadInterval) => {
                last_error = format!(
                    "Invalid or unsupported frame interval (尝试 {}fps 失败)",
                    interval_to_fps(interval)
                );
            }
            Err(err) => {
                last_error = err.to_string();
                break;
            }
        }
    }

    Err(format!(
        "启动采集失败（{} {}x{}@{}fps, MJPG）: {}",
        config.device, config.width, config.height, config.fps, last_error
    ))
}

#[cfg(target_os = "linux")]
fn tune_camera_controls_linux(camera: &rscam::Camera) {
    use rscam::{consts, Control};

    for control in camera.controls().flatten() {
        if control.flags & (consts::FLAG_DISABLED | consts::FLAG_READ_ONLY | consts::FLAG_INACTIVE)
            != 0
        {
            continue;
        }

        let name = control.name.to_ascii_lowercase();
        if is_quality_control_name(&name) {
            apply_control_target(camera, &control, true);
            continue;
        }

        if is_noise_reduction_control_name(&name) {
            apply_control_target(camera, &control, false);
        }
    }

    fn apply_control_target(camera: &rscam::Camera, control: &Control, quality_mode: bool) {
        use rscam::CtrlData;

        match &control.data {
            CtrlData::Boolean { .. } => {
                let _ = camera.set_control(control.id, &true);
            }
            CtrlData::Integer {
                minimum,
                maximum,
                step,
                ..
            } => {
                let target = if quality_mode {
                    *maximum
                } else {
                    *minimum + (((*maximum - *minimum) as f32 * 0.85) as i32)
                };
                let quantized = quantize_int_to_step(target, *minimum, *maximum, *step);
                let _ = camera.set_control(control.id, &quantized);
            }
            CtrlData::Integer64 {
                minimum,
                maximum,
                step,
                ..
            } => {
                let target = if quality_mode {
                    *maximum
                } else {
                    *minimum + (((*maximum - *minimum) as f64 * 0.85) as i64)
                };
                let quantized = quantize_i64_to_step(target, *minimum, *maximum, *step);
                let _ = camera.set_control(control.id, &quantized);
            }
            CtrlData::Menu { items, .. } => {
                if let Some(index) = pick_menu_index(items, quality_mode) {
                    let _ = camera.set_control(control.id, &index);
                }
            }
            CtrlData::IntegerMenu { items, .. } => {
                let target = if quality_mode {
                    items.iter().max_by_key(|it| it.value)
                } else {
                    items.iter().max_by_key(|it| it.value)
                };
                if let Some(item) = target {
                    let _ = camera.set_control(control.id, &item.index);
                }
            }
            _ => {}
        }
    }

    fn pick_menu_index(items: &[rscam::CtrlMenuItem], quality_mode: bool) -> Option<u32> {
        let mut best: Option<(u32, i32)> = None;
        for item in items {
            let text = item.name.to_ascii_lowercase();
            let score = if quality_mode {
                menu_quality_score(&text)
            } else {
                menu_noise_score(&text)
            };

            match best {
                None => best = Some((item.index, score)),
                Some((_, cur)) if score > cur => best = Some((item.index, score)),
                _ => {}
            }
        }
        best.map(|(idx, _)| idx)
            .or_else(|| items.last().map(|i| i.index))
    }

    fn menu_quality_score(text: &str) -> i32 {
        let mut score = 0;
        if text.contains("high") || text.contains("best") || text.contains("fine") {
            score += 40;
        }
        if text.contains("max") {
            score += 50;
        }
        if text.contains("medium") {
            score += 10;
        }
        if text.contains("low") {
            score -= 20;
        }
        score
    }

    fn menu_noise_score(text: &str) -> i32 {
        let mut score = 0;
        if text.contains("high") || text.contains("max") {
            score += 50;
        }
        if text.contains("medium") || text.contains("auto") || text.contains("on") {
            score += 25;
        }
        if text.contains("off") || text.contains("disable") || text.contains("low") {
            score -= 30;
        }
        score
    }

    fn quantize_int_to_step(value: i32, min: i32, max: i32, step: i32) -> i32 {
        let mut v = value.clamp(min, max);
        let s = step.abs().max(1);
        let offset = (v - min) / s;
        v = min + offset * s;
        v.clamp(min, max)
    }

    fn quantize_i64_to_step(value: i64, min: i64, max: i64, step: i64) -> i64 {
        let mut v = value.clamp(min, max);
        let s = step.abs().max(1);
        let offset = (v - min) / s;
        v = min + offset * s;
        v.clamp(min, max)
    }

    fn is_quality_control_name(name: &str) -> bool {
        (name.contains("jpeg") || name.contains("compression") || name.contains("quality"))
            && name.contains("quality")
    }

    fn is_noise_reduction_control_name(name: &str) -> bool {
        name.contains("noise reduction")
            || name.contains("denoise")
            || name.contains("noise_reduction")
            || name.contains("3d noise")
            || name.contains("2d noise")
    }
}

#[cfg(target_os = "linux")]
fn push_interval_unique(intervals: &mut Vec<(u32, u32)>, interval: (u32, u32)) {
    if interval.0 == 0 || interval.1 == 0 {
        return;
    }
    if !intervals.contains(&interval) {
        intervals.push(interval);
    }
}

#[cfg(target_os = "linux")]
fn interval_to_fps(interval: (u32, u32)) -> u32 {
    if interval.0 == 0 {
        return 0;
    }
    interval.1 / interval.0
}

#[cfg(target_os = "linux")]
fn fps_distance(a: (u32, u32), b: (u32, u32)) -> f64 {
    let afps = if a.0 == 0 {
        0.0
    } else {
        a.1 as f64 / a.0 as f64
    };
    let bfps = if b.0 == 0 {
        0.0
    } else {
        b.1 as f64 / b.0 as f64
    };
    (afps - bfps).abs()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CameraState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_cameras,
            set_camera,
            capture_frame,
            update_polygon_detection,
            rectify_snapshot,
            stop_camera
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
