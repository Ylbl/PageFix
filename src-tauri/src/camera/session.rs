use crate::models::CameraConfig;
use crate::state::LinuxCameraSession;
use super::controls::tune_camera_controls_linux;
use super::interval::{fps_distance, interval_to_fps, push_interval_unique};
use rscam::{Config, Error, IntervalInfo};

pub(crate) fn start_camera_session_linux(config: &CameraConfig) -> Result<LinuxCameraSession, String> {
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
