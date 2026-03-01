use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CameraDevice {
    pub(crate) path: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCameraRequest {
    pub(crate) device: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) fps: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct CameraConfig {
    pub(crate) device: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) fps: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PolygonPoint {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureFrameResponse {
    pub(crate) frame_data_url: String,
    pub(crate) polygon: Vec<PolygonPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RectifySnapshotRequest {
    pub(crate) frame_data_url: String,
    pub(crate) polygon: Vec<PolygonPoint>,
    #[serde(default = "default_post_process_mode")]
    pub(crate) post_process: String,
    #[serde(default = "default_denoise_strength")]
    pub(crate) denoise_strength: String,
}

fn default_post_process_mode() -> String {
    "sharpen".to_string()
}

fn default_denoise_strength() -> String {
    "low".to_string()
}
