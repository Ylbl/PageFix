import { type Ref } from "vue";
import { captureFrame } from "../api";
import type { PolygonPoint } from "../types";
import { normalizeError } from "../utils";

export function useFrameCapture(
  frameUrl: Ref<string>,
  framePolygon: Ref<PolygonPoint[]>,
  isRunning: Ref<boolean>,
  isPaused: Ref<boolean>,
  errorMessage: Ref<string>,
  doStopCamera: () => Promise<void>,
) {
  let captureTimer: number | null = null;
  let inFlight = false;

  function clearTimer() {
    if (captureTimer !== null) {
      window.clearInterval(captureTimer);
      captureTimer = null;
    }
  }

  function startPreviewLoops(targetFps: number) {
    clearTimer();
    const intervalMs = Math.max(1, Math.floor(1000 / targetFps));
    captureTimer = window.setInterval(() => {
      void captureOnce();
    }, intervalMs);
  }

  async function captureOnce() {
    if (!isRunning.value || isPaused.value || inFlight) {
      return;
    }
    inFlight = true;
    try {
      const result = await captureFrame();
      frameUrl.value = result.frameDataUrl;
      // Update live polygon overlay from real-time detection.
      if (!isPaused.value && result.polygon && result.polygon.length >= 4) {
        framePolygon.value = result.polygon;
      } else if (!isPaused.value) {
        framePolygon.value = [];
      }
    } catch (error) {
      errorMessage.value = normalizeError(error);
      await doStopCamera();
    } finally {
      inFlight = false;
    }
  }

  function resetInFlight() {
    inFlight = false;
  }

  return {
    clearTimer,
    startPreviewLoops,
    captureOnce,
    resetInFlight,
  };
}
