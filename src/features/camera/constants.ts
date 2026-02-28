import type { ResolutionOption } from "./types";

export const RESOLUTION_OPTIONS: ResolutionOption[] = [
  { value: "1280x720", label: "720p (1280x720)", width: 1280, height: 720 },
  { value: "1920x1080", label: "1080p (1920x1080)", width: 1920, height: 1080 },
  { value: "2560x1440", label: "2K (2560x1440)", width: 2560, height: 1440 },
  { value: "3840x2160", label: "4K (3840x2160)", width: 3840, height: 2160 },
];

export const DEFAULT_RESOLUTION = "2560x1440";
export const DEFAULT_FPS = 60;
export const DEFAULT_CAPTURE_ASPECT_RATIO = 16 / 9;
export const PREVIEW_MAX_WIDTH_PX = 756;
