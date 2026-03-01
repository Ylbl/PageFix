<script setup lang="ts">
import type { PolygonPoint } from "../types";
import PolygonOverlay from "./PolygonOverlay.vue";

defineProps<{
  frameUrl: string;
  framePolygon: PolygonPoint[];
  isPaused: boolean;
  livePanX: number;
  livePanY: number;
  liveZoom: number;
  isLiveDragging: boolean;
  canPanLive: () => boolean;
  onStartDragVertex: (index: number, event: PointerEvent) => void;
  onInsertVertexOnSegment: (segmentIndex: number, event: PointerEvent) => void;
  onRemoveVertex: (index: number, event: MouseEvent) => void;
}>();

const emit = defineEmits<{
  wheel: [event: WheelEvent];
  pointerdown: [event: PointerEvent];
  pointermove: [event: PointerEvent];
  pointerup: [event: PointerEvent];
}>();

const livePreviewRef = defineModel<HTMLElement | null>("containerRef");
const previewSvgRef = defineModel<SVGSVGElement | null>("svgRef");
</script>

<template>
  <section
    ref="livePreviewRef"
    :class="['preview', { 'live-pannable': isPaused && frameUrl && canPanLive(), 'live-dragging': isLiveDragging }]"
    @wheel.prevent="emit('wheel', $event)"
    @pointerdown="emit('pointerdown', $event)"
    @pointermove="emit('pointermove', $event)"
    @pointerup="emit('pointerup', $event)"
    @pointercancel="emit('pointerup', $event)"
  >
    <img
      v-if="frameUrl"
      class="live-preview-image"
      :src="frameUrl"
      alt="camera frame"
      :style="{ transform: `translate(${livePanX}px, ${livePanY}px) scale(${liveZoom})` }"
    />
    <PolygonOverlay
      v-if="frameUrl"
      v-model:svg-ref="previewSvgRef"
      :polygon="framePolygon"
      :is-paused="isPaused"
      :live-pan-x="livePanX"
      :live-pan-y="livePanY"
      :live-zoom="liveZoom"
      :on-start-drag-vertex="onStartDragVertex"
      :on-insert-vertex-on-segment="onInsertVertexOnSegment"
      :on-remove-vertex="onRemoveVertex"
    />
    <div v-else class="placeholder">等待 Rust 采集并标注画面...</div>
  </section>
</template>

<style scoped>
.preview {
  position: relative;
  width: min(100%, var(--preview-width));
  aspect-ratio: 16 / 9;
  margin: 0 auto;
  border-radius: 14px;
  overflow: hidden;
  background: #0f172a;
}

.live-preview-image,
:deep(.live-preview-overlay) {
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
.preview.live-dragging :deep(.live-preview-overlay) {
  transition: none;
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
</style>
