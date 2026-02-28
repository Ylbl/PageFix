import { invoke } from "@tauri-apps/api/core";
import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  DEFAULT_CAPTURE_ASPECT_RATIO,
  DEFAULT_FPS,
  DEFAULT_RESOLUTION,
  PREVIEW_MAX_WIDTH_PX,
  RESOLUTION_OPTIONS,
} from "../constants";
import type { CameraDevice, CaptureFrameResponse, PolygonPoint, PostProcessMode } from "../types";
import {
  clamp,
  clamp01,
  clonePolygon,
  defaultSnapshotPolygon,
  normalizeError,
  polygonPointsForSvg,
  polygonSegments,
} from "../utils";
import { usePanZoom } from "./usePanZoom";

export function useCameraScanner() {
  // Central UI state for camera lifecycle, polygon editing and rectify workflow.
  const devices = ref<CameraDevice[]>([]);
  const selectedDevice = ref("");
  const selectedResolution = ref(DEFAULT_RESOLUTION);
  const fps = ref(DEFAULT_FPS);
  const postProcessMode = ref<PostProcessMode>("sharpen");

  const frameUrl = ref("");
  const framePolygon = ref<PolygonPoint[]>([]);
  const capturedFrameUrl = ref("");
  const capturedAspectRatio = ref(DEFAULT_CAPTURE_ASPECT_RATIO);

  const errorMessage = ref("");
  const isLoadingDevices = ref(false);
  const isRunning = ref(false);
  const isPaused = ref(false);
  const isRectifying = ref(false);

  const previewSvgRef = ref<SVGSVGElement | null>(null);
  const livePreviewRef = ref<HTMLElement | null>(null);
  const capturePreviewRef = ref<HTMLElement | null>(null);

  const livePanZoom = usePanZoom({
    containerRef: livePreviewRef,
    minZoom: 1,
    maxZoom: 8,
    zoomStep: 0.12,
    canOperate: () => isPaused.value && Boolean(frameUrl.value),
  });

  const capturedPanZoom = usePanZoom({
    containerRef: capturePreviewRef,
    minZoom: 1,
    maxZoom: 6,
    zoomStep: 0.12,
    canOperate: () => Boolean(capturedFrameUrl.value),
  });

  const liveZoom = livePanZoom.zoom;
  const livePanX = livePanZoom.panX;
  const livePanY = livePanZoom.panY;
  const isLiveDragging = livePanZoom.isDragging;

  const capturedZoom = capturedPanZoom.zoom;
  const capturedPanX = capturedPanZoom.panX;
  const capturedPanY = capturedPanZoom.panY;
  const isCapturedDragging = capturedPanZoom.isDragging;

  let captureTimer: number | null = null;
  let inFlight = false;

  // Polygon editor state.
  let draggingVertexIndex: number | null = null;
  let draggingPointerId: number | null = null;

  function getSelectedResolutionOption() {
    return (
      RESOLUTION_OPTIONS.find((item) => item.value === selectedResolution.value) ??
      RESOLUTION_OPTIONS[1] ??
      RESOLUTION_OPTIONS[0]
    );
  }

  function getPreviewPointFromClient(clientX: number, clientY: number): PolygonPoint | null {
    const svg = previewSvgRef.value;
    if (!svg) {
      return null;
    }

    const rect = svg.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return null;
    }

    return {
      x: clamp01((clientX - rect.left) / rect.width),
      y: clamp01((clientY - rect.top) / rect.height),
    };
  }

  function startDragVertex(index: number, event: PointerEvent) {
    if (!isPaused.value || event.button !== 0) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();

    draggingVertexIndex = index;
    draggingPointerId = event.pointerId;
    moveDraggedVertex(event);
  }

  function moveDraggedVertex(event: PointerEvent) {
    if (draggingVertexIndex === null) {
      return;
    }

    if (draggingPointerId !== null && event.pointerId !== draggingPointerId) {
      return;
    }

    const point = getPreviewPointFromClient(event.clientX, event.clientY);
    if (!point) {
      return;
    }

    const next = clonePolygon(framePolygon.value);
    if (!next[draggingVertexIndex]) {
      return;
    }

    next[draggingVertexIndex] = point;
    framePolygon.value = next;
  }

  function stopDragVertex(event?: PointerEvent) {
    if (draggingVertexIndex === null) {
      return;
    }

    if (event && draggingPointerId !== null && event.pointerId !== draggingPointerId) {
      return;
    }

    draggingVertexIndex = null;
    draggingPointerId = null;
  }

  function insertVertexOnSegment(segmentIndex: number, event: PointerEvent) {
    if (!isPaused.value || event.button !== 0) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();

    if (framePolygon.value.length < 2) {
      return;
    }

    const point = getPreviewPointFromClient(event.clientX, event.clientY);
    if (!point) {
      return;
    }

    const next = clonePolygon(framePolygon.value);
    const insertAt = Math.min(segmentIndex + 1, next.length);
    next.splice(insertAt, 0, point);
    framePolygon.value = next;

    draggingVertexIndex = insertAt;
    draggingPointerId = event.pointerId;
  }

  function removeVertex(index: number, event: MouseEvent) {
    if (!isPaused.value) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();

    const next = clonePolygon(framePolygon.value);
    if (!next[index]) {
      return;
    }

    next.splice(index, 1);
    framePolygon.value = next;

    if (draggingVertexIndex === null) {
      return;
    }

    if (draggingVertexIndex === index) {
      stopDragVertex();
      return;
    }

    if (draggingVertexIndex > index) {
      draggingVertexIndex -= 1;
    }
  }

  function onWindowPointerMove(event: PointerEvent) {
    moveDraggedVertex(event);
  }

  function onWindowPointerUp(event: PointerEvent) {
    stopDragVertex(event);
  }

  async function refreshDevices() {
    isLoadingDevices.value = true;
    errorMessage.value = "";

    try {
      const list = await invoke<CameraDevice[]>("list_cameras");
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

  async function captureOnce() {
    if (!isRunning.value || isPaused.value || inFlight) {
      return;
    }

    inFlight = true;
    try {
      const result = await invoke<CaptureFrameResponse>("capture_frame");
      frameUrl.value = result.frameDataUrl;
    } catch (error) {
      errorMessage.value = normalizeError(error);
      await stopCamera();
    } finally {
      inFlight = false;
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
    capturedAspectRatio.value = DEFAULT_CAPTURE_ASPECT_RATIO;
    framePolygon.value = [];
    livePanZoom.resetTransform();
    capturedPanZoom.resetTransform();

    try {
      await invoke("set_camera", { request });
      isRunning.value = true;
      await captureOnce();
      startPreviewLoops(request.fps);
    } catch (error) {
      errorMessage.value = normalizeError(error);
      await stopCamera();
    }
  }

  async function togglePreview() {
    if (isRunning.value) {
      await stopCamera();
      return;
    }
    await startCamera();
  }

  async function stopCamera() {
    clearTimer();
    isRunning.value = false;
    isPaused.value = false;
    inFlight = false;
    framePolygon.value = [];
    livePanZoom.resetTransform();

    try {
      await invoke("stop_camera");
    } catch {
      // Ignore stop errors.
    }
  }

  async function detectPolygonForSnapshot(): Promise<PolygonPoint[]> {
    try {
      const polygon = await invoke<PolygonPoint[]>("update_polygon_detection");
      if (Array.isArray(polygon) && polygon.length >= 4) {
        return clonePolygon(polygon);
      }
    } catch {
      // Fall back to default rectangle.
    }
    return defaultSnapshotPolygon();
  }

  async function captureSnapshot() {
    errorMessage.value = "";

    if (!isRunning.value && !frameUrl.value) {
      errorMessage.value = "请先开启预览后再截屏。";
      return;
    }

    if (isRunning.value && !inFlight) {
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
      const rectified = await invoke<string>("rectify_snapshot", {
        request: {
          frameDataUrl: frameUrl.value,
          polygon: framePolygon.value,
          postProcess: postProcessMode.value,
          denoiseStrength: "low",
        },
      });
      capturedFrameUrl.value = rectified;
      capturedPanZoom.resetTransform();
    } catch (error) {
      errorMessage.value = normalizeError(error);
    } finally {
      isRectifying.value = false;
    }
  }

  function getCapturedMimeType(dataUrl: string) {
    const match = /^data:([^;,]+)[;,]/i.exec(dataUrl);
    return match?.[1] ?? "image/png";
  }

  function loadDataUrlImage(dataUrl: string) {
    return new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error("旋转失败：无法读取截图数据。"));
      image.src = dataUrl;
    });
  }

  function onCapturedImageLoad(event: Event) {
    const image = event.target as HTMLImageElement | null;
    if (!image || image.naturalWidth <= 0 || image.naturalHeight <= 0) {
      return;
    }
    capturedAspectRatio.value = image.naturalWidth / image.naturalHeight;
  }

  async function rotateCapturedBy(degrees: number) {
    if (!capturedFrameUrl.value) {
      return;
    }

    const normalized = ((Math.round(degrees) % 360) + 360) % 360;
    if (![90, 180, 270].includes(normalized)) {
      return;
    }

    try {
      const current = capturedFrameUrl.value;
      const image = await loadDataUrlImage(current);
      const quarterTurn = normalized === 90 || normalized === 270;

      const canvas = document.createElement("canvas");
      canvas.width = quarterTurn ? image.naturalHeight : image.naturalWidth;
      canvas.height = quarterTurn ? image.naturalWidth : image.naturalHeight;

      const ctx = canvas.getContext("2d");
      if (!ctx) {
        throw new Error("旋转失败：无法创建绘图上下文。");
      }

      ctx.translate(canvas.width / 2, canvas.height / 2);
      ctx.rotate((degrees * Math.PI) / 180);
      ctx.drawImage(image, -image.naturalWidth / 2, -image.naturalHeight / 2);

      const mime = getCapturedMimeType(current);
      capturedFrameUrl.value =
        mime === "image/jpeg" ? canvas.toDataURL(mime, 0.98) : canvas.toDataURL(mime);
    } catch (error) {
      errorMessage.value = normalizeError(error);
    }

    const clamped = capturedPanZoom.clampPan(capturedPanX.value, capturedPanY.value, capturedZoom.value);
    capturedPanX.value = clamped.x;
    capturedPanY.value = clamped.y;
  }

  function rotateCapturedLeft() {
    void rotateCapturedBy(-90);
  }

  function rotateCapturedRight() {
    void rotateCapturedBy(90);
  }

  onMounted(() => {
    void refreshDevices();
    window.addEventListener("pointermove", onWindowPointerMove);
    window.addEventListener("pointerup", onWindowPointerUp);
    window.addEventListener("pointercancel", onWindowPointerUp);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("pointermove", onWindowPointerMove);
    window.removeEventListener("pointerup", onWindowPointerUp);
    window.removeEventListener("pointercancel", onWindowPointerUp);
    void stopCamera();
  });

  return {
    RESOLUTION_OPTIONS,
    previewMaxWidthPx: PREVIEW_MAX_WIDTH_PX,
    devices,
    selectedDevice,
    selectedResolution,
    fps,
    postProcessMode,
    frameUrl,
    framePolygon,
    capturedFrameUrl,
    capturedAspectRatio,
    errorMessage,
    isLoadingDevices,
    isRunning,
    isPaused,
    isRectifying,
    previewSvgRef,
    livePreviewRef,
    capturePreviewRef,
    liveZoom,
    livePanX,
    livePanY,
    isLiveDragging,
    capturedZoom,
    capturedPanX,
    capturedPanY,
    isCapturedDragging,
    refreshDevices,
    togglePreview,
    toggleSnapshotResume,
    rectifySnapshot,
    rotateCapturedLeft,
    rotateCapturedRight,
    polygonSegments,
    polygonPointsForSvg,
    startDragVertex,
    insertVertexOnSegment,
    removeVertex,
    canPanLive: () => livePanZoom.canPan(),
    canPanCaptured: () => capturedPanZoom.canPan(),
    onLiveWheel: livePanZoom.onWheel,
    onLivePointerDown: livePanZoom.onPointerDown,
    onLivePointerMove: livePanZoom.onPointerMove,
    onLivePointerUp: livePanZoom.onPointerUp,
    onCapturedWheel: capturedPanZoom.onWheel,
    onCapturedPointerDown: capturedPanZoom.onPointerDown,
    onCapturedPointerMove: capturedPanZoom.onPointerMove,
    onCapturedPointerUp: capturedPanZoom.onPointerUp,
    onCapturedImageLoad,
  };
}
