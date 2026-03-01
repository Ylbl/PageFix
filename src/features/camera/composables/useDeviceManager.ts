import { ref, type Ref } from "vue";
import { listCameras, setCamera, stopCamera } from "../api";
import { RESOLUTION_OPTIONS } from "../constants";
import type { CameraDevice } from "../types";
import { clamp, normalizeError } from "../utils";

export function useDeviceManager(
  errorMessage: Ref<string>,
  isRunning: Ref<boolean>,
  isPaused: Ref<boolean>,
  framePolygon: Ref<import("../types").PolygonPoint[]>,
  capturedFrameUrl: Ref<string>,
  capturedAspectRatio: Ref<number>,
  livePanZoom: { resetTransform: () => void },
  capturedPanZoom: { resetTransform: () => void },
  startPreviewLoops: (fps: number) => void,
  captureOnce: () => Promise<void>,
  clearTimer: () => void,
) {
  const devices = ref<CameraDevice[]>([]);
  const selectedDevice = ref("");
  const selectedResolution = ref("2560x1440");
  const fps = ref(60);
  const isLoadingDevices = ref(false);

  function getSelectedResolutionOption() {
    return (
      RESOLUTION_OPTIONS.find((item) => item.value === selectedResolution.value) ??
      RESOLUTION_OPTIONS[1] ??
      RESOLUTION_OPTIONS[0]
    );
  }

  async function refreshDevices() {
    isLoadingDevices.value = true;
    errorMessage.value = "";
    try {
      const list = await listCameras();
      devices.value = list;
      if (!list.length) {
        selectedDevice.value = "";
        errorMessage.value = "未找到摄像头设备，请检查 /dev/video*。";
        return;
      }
      if (!list.some((item) => item.path === selectedDevice.value)) {
        selectedDevice.value = list[0].path;
      }
    } catch (error) {
      errorMessage.value = normalizeError(error);
    } finally {
      isLoadingDevices.value = false;
    }
  }

  async function startCamera() {
    if (isRunning.value) {
      return;
    }
    if (!selectedDevice.value) {
      await refreshDevices();
    }
    if (!selectedDevice.value) {
      errorMessage.value = errorMessage.value || "请先选择可用摄像头。";
      return;
    }
    const resolution = getSelectedResolutionOption();
    const request = {
      device: selectedDevice.value,
      width: resolution.width,
      height: resolution.height,
      fps: clamp(fps.value, 1, 60),
    };
    fps.value = request.fps;
    isPaused.value = false;
    errorMessage.value = "";
    capturedFrameUrl.value = "";
    capturedAspectRatio.value = 16 / 9;
    framePolygon.value = [];
    livePanZoom.resetTransform();
    capturedPanZoom.resetTransform();
    try {
      await setCamera(request);
      isRunning.value = true;
      await captureOnce();
      startPreviewLoops(request.fps);
    } catch (error) {
      errorMessage.value = normalizeError(error);
      await doStopCamera();
    }
  }

  async function togglePreview() {
    if (isRunning.value) {
      await doStopCamera();
      return;
    }
    await startCamera();
  }

  async function doStopCamera() {
    clearTimer();
    isRunning.value = false;
    isPaused.value = false;
    framePolygon.value = [];
    livePanZoom.resetTransform();
    try {
      await stopCamera();
    } catch {
      // Ignore stop errors.
    }
  }

  return {
    devices,
    selectedDevice,
    selectedResolution,
    fps,
    isLoadingDevices,
    refreshDevices,
    startCamera,
    togglePreview,
    doStopCamera,
  };
}
