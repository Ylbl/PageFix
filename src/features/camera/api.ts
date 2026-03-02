import { invoke } from "@tauri-apps/api/core";
import type { CameraDevice, CaptureFrameResponse, DetectionWeights, PolygonPoint, PostProcessMode } from "./types";

export async function listCameras(): Promise<CameraDevice[]> {
  return invoke<CameraDevice[]>("list_cameras");
}

export async function setCamera(request: {
  device: string;
  width: number;
  height: number;
  fps: number;
}): Promise<void> {
  return invoke("set_camera", { request });
}

export async function stopCamera(): Promise<void> {
  return invoke("stop_camera");
}

export async function captureFrame(): Promise<CaptureFrameResponse> {
  return invoke<CaptureFrameResponse>("capture_frame");
}

export async function updateDetectionWeights(weights: DetectionWeights): Promise<void> {
  return invoke("update_detection_weights", { weights });
}

export async function rectifySnapshot(request: {
  frameDataUrl: string;
  polygon: PolygonPoint[];
  postProcess: PostProcessMode;
  denoiseStrength: string;
}): Promise<string> {
  return invoke<string>("rectify_snapshot", { request });
}
