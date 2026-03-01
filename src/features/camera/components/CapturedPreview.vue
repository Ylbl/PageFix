<script setup lang="ts">
defineProps<{
  capturedFrameUrl: string;
  capturedAspectRatio: number;
  capturedPanX: number;
  capturedPanY: number;
  capturedZoom: number;
  isCapturedDragging: boolean;
  canPanCaptured: () => boolean;
}>();

const emit = defineEmits<{
  wheel: [event: WheelEvent];
  pointerdown: [event: PointerEvent];
  pointermove: [event: PointerEvent];
  pointerup: [event: PointerEvent];
  imageLoad: [event: Event];
  rotateCapturedLeft: [];
  rotateCapturedRight: [];
}>();

const capturePreviewRef = defineModel<HTMLElement | null>("containerRef");
</script>

<template>
  <section
    ref="capturePreviewRef"
    :class="[
      'preview',
      'capture-preview',
      { pannable: capturedFrameUrl && canPanCaptured(), dragging: isCapturedDragging },
    ]"
    :style="{ aspectRatio: String(capturedAspectRatio) }"
    @wheel.prevent="emit('wheel', $event)"
    @pointerdown="emit('pointerdown', $event)"
    @pointermove="emit('pointermove', $event)"
    @pointerup="emit('pointerup', $event)"
    @pointercancel="emit('pointerup', $event)"
  >
    <img
      v-if="capturedFrameUrl"
      :src="capturedFrameUrl"
      alt="captured frame"
      :style="{ transform: `translate(${capturedPanX}px, ${capturedPanY}px) scale(${capturedZoom})` }"
      @load="emit('imageLoad', $event)"
    />
    <div v-else class="placeholder">截屏后此窗口为空，点击"矫正"后显示结果。</div>
  </section>

  <div class="capture-actions">
    <button class="secondary" :disabled="!capturedFrameUrl" @click="emit('rotateCapturedLeft')">左旋转90°</button>
    <button class="secondary" :disabled="!capturedFrameUrl" @click="emit('rotateCapturedRight')">右旋转90°</button>
  </div>
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

img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

button.secondary {
  border-radius: 10px;
  border: 1px solid #9ca3af;
  padding: 10px 12px;
  font-size: 15px;
  background: #ffffff;
  color: #111827;
  cursor: pointer;
}

button.secondary:disabled {
  opacity: 0.55;
  cursor: not-allowed;
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
