use crate::models::{PolygonPoint, RectifySnapshotRequest};
use crate::state::CameraState;
use tauri::State;

#[tauri::command]
pub(crate) fn update_polygon_detection(
    state: State<CameraState>,
) -> Result<Vec<PolygonPoint>, String> {
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
        let polygon = crate::vision::detect_polygon_on_jpeg_linux(&frame).unwrap_or_default();
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
pub(crate) fn rectify_snapshot(request: RectifySnapshotRequest) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        return crate::vision::rectify_snapshot_linux(
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
