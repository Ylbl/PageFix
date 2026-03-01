use crate::models::RectifySnapshotRequest;

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
