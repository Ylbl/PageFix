// Device entry returned by the Rust `list_cameras` command.
export type CameraDevice = {
  path: string;
  name: string;
};

// Polygon vertex in normalized coordinates: [0, 1] x [0, 1].
export type PolygonPoint = {
  x: number;
  y: number;
};

// Single frame payload returned by Rust capture command.
export type CaptureFrameResponse = {
  frameDataUrl: string;
  polygon: PolygonPoint[];
};

// Post-processing choices executed in Rust after perspective correction.
export type PostProcessMode = "none" | "sharpen";

// Detection pipeline weights for tuning.
export type DetectionWeights = {
  canny: number;
  hsv: number;
};

// Select-option model for capture resolutions.
export type ResolutionOption = {
  value: string;
  label: string;
  width: number;
  height: number;
};
