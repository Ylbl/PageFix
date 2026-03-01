<script setup lang="ts">
import type { PolygonPoint } from "../types";
import { polygonPointsForSvg, polygonSegments } from "../utils";

defineProps<{
  polygon: PolygonPoint[];
  isPaused: boolean;
  livePanX: number;
  livePanY: number;
  liveZoom: number;
  onStartDragVertex: (index: number, event: PointerEvent) => void;
  onInsertVertexOnSegment: (segmentIndex: number, event: PointerEvent) => void;
  onRemoveVertex: (index: number, event: MouseEvent) => void;
}>();

const previewSvgRef = defineModel<SVGSVGElement | null>("svgRef");
</script>

<template>
  <svg
    ref="previewSvgRef"
    :class="['polygon-overlay', 'live-preview-overlay', { 'editable-overlay': isPaused }]"
    :style="{ transform: `translate(${livePanX}px, ${livePanY}px) scale(${liveZoom})` }"
    viewBox="0 0 100 100"
    preserveAspectRatio="none"
  >
    <polygon
      v-if="isPaused && polygon.length >= 3"
      class="editable-polygon-fill"
      :points="polygonPointsForSvg(polygon)"
    />
    <template v-for="segment in isPaused ? polygonSegments(polygon) : []" :key="`seg-${segment.index}`">
      <line
        class="segment-hit"
        :x1="segment.from.x * 100"
        :y1="segment.from.y * 100"
        :x2="segment.to.x * 100"
        :y2="segment.to.y * 100"
        @pointerdown="onInsertVertexOnSegment(segment.index, $event)"
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
      v-for="(point, index) in isPaused ? polygon : []"
      :key="`point-${index}`"
      class="vertex-handle"
      :cx="point.x * 100"
      :cy="point.y * 100"
      r="1.6"
      @pointerdown="onStartDragVertex(index, $event)"
      @contextmenu.prevent="onRemoveVertex(index, $event)"
    />
  </svg>
</template>

<style scoped>
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
</style>
