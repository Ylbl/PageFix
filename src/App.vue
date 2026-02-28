<script setup lang="ts">
import { useCameraScanner } from "./features/camera/composables/useCameraScanner";

const {
  RESOLUTION_OPTIONS,
  previewMaxWidthPx,
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
    <div class="controls">
      <button :disabled="isLoadingDevices || isRunning" @click="refreshDevices">
        {{ isLoadingDevices ? "扫描中..." : "刷新设备" }}
      </button>

      <select v-model="selectedDevice" :disabled="isRunning || !devices.length">
        <option value="" disabled>请选择摄像头</option>
        <option v-for="item in devices" :key="item.path" :value="item.path">
          {{ item.name }} ({{ item.path }})
        </option>
      </select>
    </div>

    <div class="controls">
      <select v-model="selectedResolution" :disabled="isRunning">
        <option v-for="item in RESOLUTION_OPTIONS" :key="item.value" :value="item.value">
          {{ item.label }}
        </option>
      </select>
      <input v-model.number="fps" :disabled="isRunning" type="number" min="1" max="60" placeholder="FPS" />
      <button :disabled="!isRunning && !devices.length" @click="togglePreview">
        {{ isRunning ? "关闭预览" : "开启预览" }}
      </button>
      <button
        :disabled="isPaused ? !isRunning : !isRunning || !frameUrl"
        class="secondary"
        @click="toggleSnapshotResume"
      >
        {{ isPaused ? "继续" : "截屏" }}
      </button>
      <button :disabled="!isPaused || !frameUrl || framePolygon.length < 4 || isRectifying" @click="rectifySnapshot">
        {{ isRectifying ? "矫正中..." : "矫正" }}
      </button>
      <select v-model="postProcessMode" class="post-process-select" :disabled="isRectifying">
        <option value="none">无</option>
        <option value="sharpen">锐化</option>
      </select>
    </div>

    <p v-if="errorMessage" class="error-message">{{ errorMessage }}</p>
    <p v-if="isRunning && isPaused" class="status-message">预览已暂停，请在上方窗口拖拽顶点后点击“矫正”。</p>

    <section
      ref="livePreviewRef"
      :class="['preview', { 'live-pannable': isPaused && frameUrl && canPanLive(), 'live-dragging': isLiveDragging }]"
      @wheel.prevent="onLiveWheel"
      @pointerdown="onLivePointerDown"
      @pointermove="onLivePointerMove"
      @pointerup="onLivePointerUp"
      @pointercancel="onLivePointerUp"
    >
      <img
        v-if="frameUrl"
        class="live-preview-image"
        :src="frameUrl"
        alt="camera frame"
        :style="{ transform: `translate(${livePanX}px, ${livePanY}px) scale(${liveZoom})` }"
      />
      <svg
        v-if="frameUrl"
        ref="previewSvgRef"
        :class="['polygon-overlay', 'live-preview-overlay', { 'editable-overlay': isPaused }]"
        :style="{ transform: `translate(${livePanX}px, ${livePanY}px) scale(${liveZoom})` }"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
      >
        <polygon
          v-if="isPaused && framePolygon.length >= 3"
          class="editable-polygon-fill"
          :points="polygonPointsForSvg(framePolygon)"
        />
        <template v-for="segment in isPaused ? polygonSegments(framePolygon) : []" :key="`seg-${segment.index}`">
          <line
            class="segment-hit"
            :x1="segment.from.x * 100"
            :y1="segment.from.y * 100"
            :x2="segment.to.x * 100"
            :y2="segment.to.y * 100"
            @pointerdown="insertVertexOnSegment(segment.index, $event)"
          />
          <line
            class="segment-line"
            :x1="segment.from.x * 100"
            :y1="segment.from.y * 100"
            :x2="segment.to.x * 100"
            :y2="segment.to.y * 100"
          />
        </template>
        <circle
          v-for="(point, index) in isPaused ? framePolygon : []"
          :key="`point-${index}`"
          class="vertex-handle"
          :cx="point.x * 100"
          :cy="point.y * 100"
          r="1.6"
          @pointerdown="startDragVertex(index, $event)"
          @contextmenu.prevent="removeVertex(index, $event)"
        />
      </svg>
      <div v-else class="placeholder">等待 Rust 采集并标注画面...</div>
    </section>

    <section
      ref="capturePreviewRef"
      :class="[
        'preview',
        'capture-preview',
        { pannable: capturedFrameUrl && canPanCaptured(), dragging: isCapturedDragging },
      ]"
      :style="{ aspectRatio: String(capturedAspectRatio) }"
      @wheel.prevent="onCapturedWheel"
      @pointerdown="onCapturedPointerDown"
      @pointermove="onCapturedPointerMove"
      @pointerup="onCapturedPointerUp"
      @pointercancel="onCapturedPointerUp"
    >
      <img
        v-if="capturedFrameUrl"
        :src="capturedFrameUrl"
        alt="captured frame"
        :style="{ transform: `translate(${capturedPanX}px, ${capturedPanY}px) scale(${capturedZoom})` }"
        @load="onCapturedImageLoad"
      />
      <div v-else class="placeholder">截屏后此窗口为空，点击“矫正”后显示结果。</div>
    </section>

    <div class="capture-actions">
      <button class="secondary" :disabled="!capturedFrameUrl" @click="rotateCapturedLeft">左旋转90°</button>
      <button class="secondary" :disabled="!capturedFrameUrl" @click="rotateCapturedRight">右旋转90°</button>
    </div>
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

.controls {
  display: flex;
  gap: 12px;
  margin-bottom: 14px;
  flex-wrap: wrap;
}

button,
select,
input {
  border-radius: 10px;
  border: 1px solid #cbd5e1;
  padding: 10px 12px;
  font-size: 15px;
}

button {
  border-color: #2563eb;
  background: #2563eb;
  color: #ffffff;
  cursor: pointer;
}

button.secondary {
  border-color: #9ca3af;
  background: #ffffff;
  color: #111827;
}

button:disabled,
select:disabled,
input:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

select {
  min-width: 260px;
}

.post-process-select {
  min-width: 86px;
  width: auto;
  padding: 8px 10px;
  font-size: 14px;
}

input {
  width: 120px;
  background: #ffffff;
}

.error-message {
  margin: 0 0 14px;
  color: #b91c1c;
}

.status-message {
  margin: 0 0 14px;
  color: #14532d;
}

.preview {
  position: relative;
  width: min(100%, var(--preview-width));
  aspect-ratio: 16 / 9;
  margin: 0 auto;
  border-radius: 14px;
  overflow: hidden;
  background: #0f172a;
}

.capture-preview {
  margin-top: 14px;
  background: #ffffff;
}

.capture-actions {
  width: min(100%, var(--preview-width));
  margin: 10px auto 0;
  display: flex;
  gap: 10px;
  justify-content: center;
}

.capture-preview img {
  object-fit: contain;
  transform-origin: center center;
  transition: transform 60ms linear;
  will-change: transform;
}

.live-preview-image,
.live-preview-overlay {
  transform-origin: center center;
  transition: transform 60ms linear;
  will-change: transform;
}

.preview.live-pannable {
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.preview.live-dragging {
  cursor: grabbing;
}

.preview.live-dragging .live-preview-image,
.preview.live-dragging .live-preview-overlay {
  transition: none;
}

.capture-preview.pannable {
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.capture-preview.dragging {
  cursor: grabbing;
}

.capture-preview.dragging img {
  transition: none;
}

.polygon-overlay {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.polygon-overlay polygon {
  fill: rgba(34, 197, 94, 0.08);
  stroke: #22c55e;
  stroke-width: 0.9;
  vector-effect: non-scaling-stroke;
}

.editable-overlay {
  pointer-events: auto;
  touch-action: none;
}

.editable-polygon-fill {
  fill: rgba(34, 197, 94, 0.12);
  stroke: none;
}

.segment-line {
  stroke: #22c55e;
  stroke-width: 1.2;
  stroke-linecap: round;
  vector-effect: non-scaling-stroke;
}

.segment-hit {
  stroke: transparent;
  stroke-width: 10;
  stroke-linecap: round;
  cursor: copy;
  pointer-events: stroke;
  vector-effect: non-scaling-stroke;
}

.vertex-handle {
  fill: #10b981;
  stroke: #ffffff;
  stroke-width: 0.5;
  cursor: grab;
  pointer-events: all;
  vector-effect: non-scaling-stroke;
}

.vertex-handle:active {
  cursor: grabbing;
}

img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.placeholder {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: #cbd5e1;
  font-size: 15px;
}

@media (max-width: 640px) {
  .camera-page {
    padding: 16px 14px 24px;
  }

  select,
  input {
    width: 100%;
  }

  .post-process-select {
    width: auto;
  }
}
</style>

<style>
html,
body,
#app {
  margin: 0;
  min-height: 100%;
}

body {
  background: #f3f4f6;
}
</style>
