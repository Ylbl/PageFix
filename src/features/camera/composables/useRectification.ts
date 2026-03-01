import { type Ref } from "vue";
import { rectifySnapshot as apiRectifySnapshot, updatePolygonDetection } from "../api";
import { DEFAULT_CAPTURE_ASPECT_RATIO } from "../constants";
import type { PolygonPoint, PostProcessMode } from "../types";
import { clamp, clonePolygon, defaultSnapshotPolygon, normalizeError } from "../utils";

export function useRectification(
  frameUrl: Ref<string>,
  framePolygon: Ref<PolygonPoint[]>,
  capturedFrameUrl: Ref<string>,
  capturedAspectRatio: Ref<number>,
  errorMessage: Ref<string>,
  isRunning: Ref<boolean>,
  isPaused: Ref<boolean>,
  isRectifying: Ref<boolean>,
  postProcessMode: Ref<PostProcessMode>,
  livePanZoom: { resetTransform: () => void },
  capturedPanZoom: { resetTransform: () => void },
  captureOnce: () => Promise<void>,
  clearTimer: () => void,
  startPreviewLoops: (fps: number) => void,
  fps: Ref<number>,
) {
  async function detectPolygonForSnapshot(): Promise<PolygonPoint[]> {
    try {
      const polygon = await updatePolygonDetection();
      if (Array.isArray(polygon) && polygon.length >= 4) {
        return clonePolygon(polygon);
      }
    } catch {
      // Fall back to default rectangle.
    }
    return defaultSnapshotPolygon();
  }

  function pausePreview() {
    clearTimer();
    isPaused.value = true;
  }

  async function resumePreview() {
    if (!isRunning.value || !isPaused.value) {
      return;
    }
    isPaused.value = false;
    framePolygon.value = [];
    errorMessage.value = "";
    livePanZoom.resetTransform();
    await captureOnce();
    startPreviewLoops(clamp(fps.value, 1, 60));
  }

  async function captureSnapshot() {
    errorMessage.value = "";
    if (!isRunning.value && !frameUrl.value) {
      errorMessage.value = "请先开启预览后再截屏。";
      return;
    }
    if (isRunning.value) {
      await captureOnce();
    }
    if (frameUrl.value) {
      capturedFrameUrl.value = "";
      capturedAspectRatio.value = DEFAULT_CAPTURE_ASPECT_RATIO;
      livePanZoom.resetTransform();
      capturedPanZoom.resetTransform();
      framePolygon.value = await detectPolygonForSnapshot();
      if (isRunning.value) {
        pausePreview();
      }
    }
  }

  async function toggleSnapshotResume() {
    if (isPaused.value) {
      await resumePreview();
      return;
    }
    await captureSnapshot();
  }

  async function rectifySnapshot() {
    errorMessage.value = "";
    if (!frameUrl.value || framePolygon.value.length < 4) {
      errorMessage.value = "请先截屏并保证至少有 4 个顶点。";
      return;
    }
    isRectifying.value = true;
    try {
      const rectified = await apiRectifySnapshot({
        frameDataUrl: frameUrl.value,
        polygon: framePolygon.value,
        postProcess: postProcessMode.value,
        denoiseStrength: "low",
      });
      capturedFrameUrl.value = rectified;
      capturedPanZoom.resetTransform();
    } catch (error) {
      errorMessage.value = normalizeError(error);
    } finally {
      isRectifying.value = false;
    }
  }

  return {
    toggleSnapshotResume,
    rectifySnapshot,
  };
}
