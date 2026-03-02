<script setup lang="ts">
import type { DetectionWeights } from "../types";

const weights = defineModel<DetectionWeights>("weights", { required: true });

const emit = defineEmits<{
  update: [];
}>();
</script>

<template>
  <div class="weights-panel">
    <h3>检测算法权重调试</h3>
    <div class="weight-control">
      <label>
        <span>Canny 边缘检测: {{ weights.canny.toFixed(2) }}</span>
        <input v-model.number="weights.canny" type="range" min="0" max="2" step="0.05" @input="emit('update')" />
      </label>
    </div>
    <div class="weight-control">
      <label>
        <span>HSV 亮度检测: {{ weights.hsv.toFixed(2) }}</span>
        <input v-model.number="weights.hsv" type="range" min="0" max="2" step="0.05" @input="emit('update')" />
      </label>
    </div>
  </div>
</template>

<style scoped>
.weights-panel {
  margin-bottom: 14px;
  padding: 12px;
  border: 1px solid #cbd5e1;
  border-radius: 10px;
  background: #f8fafc;
}

h3 {
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: #334155;
}

.weight-control {
  margin-bottom: 10px;
}

.weight-control:last-child {
  margin-bottom: 0;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span {
  font-size: 13px;
  color: #475569;
}

input[type="range"] {
  width: 100%;
  height: 6px;
  border-radius: 3px;
  background: #cbd5e1;
  outline: none;
  -webkit-appearance: none;
}

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #2563eb;
  cursor: pointer;
}

input[type="range"]::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #2563eb;
  cursor: pointer;
  border: none;
}
</style>
