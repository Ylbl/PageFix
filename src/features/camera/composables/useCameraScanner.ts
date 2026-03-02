import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  DEFAULT_CAPTURE_ASPECT_RATIO,
  PREVIEW_MAX_WIDTH_PX,
  RESOLUTION_OPTIONS,
} from "../constants";
import type { DetectionWeights, PolygonPoint, PostProcessMode } from "../types";
import { polygonPointsForSvg, polygonSegments } from "../utils";
import { updateDetectionWeights } from "../api";
import { useDeviceManager } from "./useDeviceManager";
import { useFrameCapture } from "./useFrameCapture";
import { useImageRotation } from "./useImageRotation";
import { usePanZoom } from "./usePanZoom";
import { usePolygonEditor } from "./usePolygonEditor";
import { useRectification } from "./useRectification";

export function useCameraScanner() {
  const frameUrl = ref("");
  const framePolygon = ref<PolygonPoint[]>([]);
  const capturedFrameUrl = ref("");
  const capturedAspectRatio = ref(DEFAULT_CAPTURE_ASPECT_RATIO);
  const errorMessage = ref("");
  const isRunning = ref(false);
  const isPaused = ref(false);
  const isRectifying = ref(false);
  const postProcessMode = ref<PostProcessMode>("sharpen");
  const detectionWeights = ref<DetectionWeights>({ canny: 0.5, hsv: 1.0 });
  const previewSvgRef = ref<SVGSVGElement | null>(null);
  const livePreviewRef = ref<HTMLElement | null>(null);
  const capturePreviewRef = ref<HTMLElement | null>(null);

  async function onWeightsUpdate() {
    try {
      await updateDetectionWeights(detectionWeights.value);
    } catch (error) {
      console.error("更新权重失败:", error);
    }
  }

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

  // Frame capture needs doStopCamera, but doStopCamera comes from device manager
  // which needs captureOnce + startPreviewLoops. Resolve with a late-bound wrapper.
  let doStopCameraFn: () => Promise<void> = async () => {};

  const { clearTimer, startPreviewLoops, captureOnce, resetInFlight } = useFrameCapture(
    frameUrl,
    framePolygon,
    isRunning,
    isPaused,
    errorMessage,
    () => doStopCameraFn(),
  );

  const {
    devices,
    selectedDevice,
    selectedResolution,
    fps,
    isLoadingDevices,
    refreshDevices,
    togglePreview,
    doStopCamera,
  } = useDeviceManager(
    errorMessage,
    isRunning,
    isPaused,
    framePolygon,
    capturedFrameUrl,
    capturedAspectRatio,
    livePanZoom,
    capturedPanZoom,
    startPreviewLoops,
    captureOnce,
    clearTimer,
  );

  doStopCameraFn = async () => {
    resetInFlight();
    await doStopCamera();
  };

  const { toggleSnapshotResume, rectifySnapshot } = useRectification(
    frameUrl,
    framePolygon,
    capturedFrameUrl,
    capturedAspectRatio,
    errorMessage,
    isRunning,
    isPaused,
    isRectifying,
    postProcessMode,
    livePanZoom,
    capturedPanZoom,
    captureOnce,
    clearTimer,
    startPreviewLoops,
    fps,
  );

  const {
    startDragVertex,
    insertVertexOnSegment,
    removeVertex,
    onWindowPointerMove,
    onWindowPointerUp,
  } = usePolygonEditor(framePolygon, previewSvgRef, isPaused);

  const { onCapturedImageLoad, rotateCapturedLeft, rotateCapturedRight } = useImageRotation(
    capturedFrameUrl,
    capturedAspectRatio,
    errorMessage,
    capturedPanZoom,
    capturedPanZoom.panX,
    capturedPanZoom.panY,
    capturedPanZoom.zoom,
  );

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
    void doStopCamera();
  });

  return {
    RESOLUTION_OPTIONS,
    previewMaxWidthPx: PREVIEW_MAX_WIDTH_PX,
    devices,
    selectedDevice,
    selectedResolution,
    fps,
    postProcessMode,
    detectionWeights,
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
    liveZoom: livePanZoom.zoom,
    livePanX: livePanZoom.panX,
    livePanY: livePanZoom.panY,
    isLiveDragging: livePanZoom.isDragging,
    capturedZoom: capturedPanZoom.zoom,
    capturedPanX: capturedPanZoom.panX,
    capturedPanY: capturedPanZoom.panY,
    isCapturedDragging: capturedPanZoom.isDragging,
    refreshDevices,
    togglePreview,
    toggleSnapshotResume,
    rectifySnapshot,
    rotateCapturedLeft,
    rotateCapturedRight,
    onWeightsUpdate,
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
