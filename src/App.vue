<script setup lang="ts">
import { useCameraScanner } from "./features/camera/composables/useCameraScanner";
import CameraControls from "./features/camera/components/CameraControls.vue";
import DetectionWeights from "./features/camera/components/DetectionWeights.vue";
import LivePreview from "./features/camera/components/LivePreview.vue";
import CapturedPreview from "./features/camera/components/CapturedPreview.vue";

const {
  RESOLUTION_OPTIONS,
  previewMaxWidthPx,
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
  onWeightsUpdate,
  startDragVertex,
  insertVertexOnSegment,
  removeVertex,
  canPanLive,
  canPanCaptured,
  onLiveWheel,
  onLivePointerDown,
  onLivePointerMove,
  onLivePointerUp,
  onCapturedWheel,
  onCapturedPointerDown,
  onCapturedPointerMove,
  onCapturedPointerUp,
  onCapturedImageLoad,
} = useCameraScanner();
</script>

<template>
  <main class="camera-page" :style="{ '--preview-width': `${previewMaxWidthPx}px` }">
    <CameraControls
      v-model:selected-device="selectedDevice"
      v-model:selected-resolution="selectedResolution"
      v-model:fps="fps"
      v-model:post-process-mode="postProcessMode"
      :devices="devices"
      :is-loading-devices="isLoadingDevices"
      :is-running="isRunning"
      :is-paused="isPaused"
      :is-rectifying="isRectifying"
      :frame-url="frameUrl"
      :frame-polygon-length="framePolygon.length"
      :resolution-options="RESOLUTION_OPTIONS"
      @refresh-devices="refreshDevices"
      @toggle-preview="togglePreview"
      @toggle-snapshot-resume="toggleSnapshotResume"
      @rectify-snapshot="rectifySnapshot"
    />

    <DetectionWeights v-model:weights="detectionWeights" @update="onWeightsUpdate" />

    <p v-if="errorMessage" class="error-message">{{ errorMessage }}</p>
    <p v-if="isRunning && isPaused" class="status-message">预览已暂停，请在上方窗口拖拽顶点后点击"矫正"。</p>

    <LivePreview
      v-model:container-ref="livePreviewRef"
      v-model:svg-ref="previewSvgRef"
      :frame-url="frameUrl"
      :frame-polygon="framePolygon"
      :is-paused="isPaused"
      :live-pan-x="livePanX"
      :live-pan-y="livePanY"
      :live-zoom="liveZoom"
      :is-live-dragging="isLiveDragging"
      :can-pan-live="canPanLive"
      :on-start-drag-vertex="startDragVertex"
      :on-insert-vertex-on-segment="insertVertexOnSegment"
      :on-remove-vertex="removeVertex"
      @wheel="onLiveWheel"
      @pointerdown="onLivePointerDown"
      @pointermove="onLivePointerMove"
      @pointerup="onLivePointerUp"
    />

    <CapturedPreview
      v-model:container-ref="capturePreviewRef"
      :captured-frame-url="capturedFrameUrl"
      :captured-aspect-ratio="capturedAspectRatio"
      :captured-pan-x="capturedPanX"
      :captured-pan-y="capturedPanY"
      :captured-zoom="capturedZoom"
      :is-captured-dragging="isCapturedDragging"
      :can-pan-captured="canPanCaptured"
      @wheel="onCapturedWheel"
      @pointerdown="onCapturedPointerDown"
      @pointermove="onCapturedPointerMove"
      @pointerup="onCapturedPointerUp"
      @image-load="onCapturedImageLoad"
      @rotate-captured-left="rotateCapturedLeft"
      @rotate-captured-right="rotateCapturedRight"
    />
  </main>
</template>

<style scoped>
.camera-page {
  max-width: 960px;
  margin: 0 auto;
  padding: 24px 20px 32px;
  color: #1f2937;
  --preview-width: 540px;
}

.error-message {
  margin: 0 0 14px;
  color: #b91c1c;
}

.status-message {
  margin: 0 0 14px;
  color: #14532d;
}

@media (max-width: 640px) {
  .camera-page {
    padding: 16px 14px 24px;
  }
}
</style>
