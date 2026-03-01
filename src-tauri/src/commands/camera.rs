use crate::models::{CameraConfig, CameraDevice, CaptureFrameResponse, StartCameraRequest};
use crate::state::CameraState;
use tauri::State;

#[cfg(target_os = "linux")]
use base64::{engine::general_purpose::STANDARD, Engine as _};
#[cfg(target_os = "linux")]
use crate::camera::start_camera_session_linux;
#[cfg(target_os = "linux")]
use std::fs;

#[tauri::command]
pub(crate) fn list_cameras() -> Result<Vec<CameraDevice>, String> {
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
pub(crate) fn set_camera(
    state: State<CameraState>,
    request: StartCameraRequest,
) -> Result<(), String> {
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
pub(crate) fn stop_camera(state: State<CameraState>) -> Result<(), String> {
    let mut guard = state
        .current
        .lock()
        .map_err(|_| "摄像头状态锁失败。".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub(crate) fn capture_frame(state: State<CameraState>) -> Result<CaptureFrameResponse, String> {
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
            if !crate::vision::looks_like_jpeg(&frame_bytes) {
                return Err(
                    "当前仅支持 MJPG 摄像头输出，请换支持 MJPG 的分辨率/设备。".into(),
                );
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
