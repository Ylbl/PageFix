use std::sync::Mutex;

#[cfg(target_os = "linux")]
use crate::models::PolygonPoint;

#[cfg(target_os = "linux")]
pub(crate) struct LinuxCameraSession {
    pub(crate) camera: rscam::Camera,
    pub(crate) last_frame_jpeg: Vec<u8>,
    pub(crate) last_polygon: Option<Vec<PolygonPoint>>,
}

pub(crate) struct CameraState {
    #[cfg(target_os = "linux")]
    pub(crate) current: Mutex<Option<LinuxCameraSession>>,
    #[cfg(not(target_os = "linux"))]
    pub(crate) current: Mutex<Option<crate::models::CameraConfig>>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }
}
