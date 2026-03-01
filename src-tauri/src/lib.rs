mod models;
mod state;
#[cfg(target_os = "linux")]
mod camera;
#[cfg(target_os = "linux")]
mod vision;
mod commands;

use state::CameraState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CameraState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::camera::list_cameras,
            commands::camera::set_camera,
            commands::camera::capture_frame,
            commands::camera::stop_camera,
            commands::vision::update_polygon_detection,
            commands::vision::rectify_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
