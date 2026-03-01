<script setup lang="ts">
import type { CameraDevice, PostProcessMode, ResolutionOption } from "../types";

defineProps<{
  devices: CameraDevice[];
  isLoadingDevices: boolean;
  isRunning: boolean;
  isPaused: boolean;
  isRectifying: boolean;
  frameUrl: string;
  framePolygonLength: number;
  resolutionOptions: ResolutionOption[];
}>();

const selectedDevice = defineModel<string>("selectedDevice");
const selectedResolution = defineModel<string>("selectedResolution");
const fps = defineModel<number>("fps");
const postProcessMode = defineModel<PostProcessMode>("postProcessMode");

const emit = defineEmits<{
  refreshDevices: [];
  togglePreview: [];
  toggleSnapshotResume: [];
  rectifySnapshot: [];
}>();
</script>

<template>
  <div class="controls">
    <button :disabled="isLoadingDevices || isRunning" @click="emit('refreshDevices')">
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
      <option v-for="item in resolutionOptions" :key="item.value" :value="item.value">
        {{ item.label }}
      </option>
    </select>
    <input v-model.number="fps" :disabled="isRunning" type="number" min="1" max="60" placeholder="FPS" />
    <button :disabled="!isRunning && !devices.length" @click="emit('togglePreview')">
      {{ isRunning ? "关闭预览" : "开启预览" }}
    </button>
    <button
      :disabled="isPaused ? !isRunning : !isRunning || !frameUrl"
      class="secondary"
      @click="emit('toggleSnapshotResume')"
    >
      {{ isPaused ? "继续" : "截屏" }}
    </button>
    <button :disabled="!isPaused || !frameUrl || framePolygonLength < 4 || isRectifying" @click="emit('rectifySnapshot')">
      {{ isRectifying ? "矫正中..." : "矫正" }}
    </button>
    <select v-model="postProcessMode" class="post-process-select" :disabled="isRectifying">
      <option value="none">无</option>
      <option value="sharpen">锐化</option>
    </select>
  </div>
</template>

<style scoped>
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

@media (max-width: 640px) {
  select,
  input {
    width: 100%;
  }

  .post-process-select {
    width: auto;
  }
}
</style>
