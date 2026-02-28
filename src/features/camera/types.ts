export type CameraDevice = {
  path: string;
  name: string;
};

export type PolygonPoint = {
  x: number;
  y: number;
};

export type CaptureFrameResponse = {
  frameDataUrl: string;
  polygon: PolygonPoint[];
};

export type PostProcessMode = "none" | "sharpen";

export type ResolutionOption = {
  value: string;
  label: string;
  width: number;
  height: number;
};
