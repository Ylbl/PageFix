import { ref, type Ref } from "vue";
import { clampFloat } from "../utils";

type UsePanZoomOptions = {
  containerRef: Ref<HTMLElement | null>;
  minZoom: number;
  maxZoom: number;
  zoomStep: number;
  canOperate: () => boolean;
};

export function usePanZoom(options: UsePanZoomOptions) {
  const zoom = ref(1);
  const panX = ref(0);
  const panY = ref(0);
  const isDragging = ref(false);

  let dragPointerId: number | null = null;
  let dragStartClientX = 0;
  let dragStartClientY = 0;
  let dragStartPanX = 0;
  let dragStartPanY = 0;

  function getPanLimits(zoomLevel = zoom.value) {
    const rect = options.containerRef.value?.getBoundingClientRect();
    if (!rect || rect.width <= 0 || rect.height <= 0 || zoomLevel <= 1) {
      return { maxX: 0, maxY: 0 };
    }

    const contentWidth = rect.width * zoomLevel;
    const contentHeight = rect.height * zoomLevel;

    return {
      maxX: Math.max((contentWidth - rect.width) / 2, 0),
      maxY: Math.max((contentHeight - rect.height) / 2, 0),
    };
  }

  function canPan(zoomLevel = zoom.value) {
    const limits = getPanLimits(zoomLevel);
    return limits.maxX > 0.5 || limits.maxY > 0.5;
  }

  function clampPan(nextX: number, nextY: number, zoomLevel = zoom.value) {
    const { maxX, maxY } = getPanLimits(zoomLevel);
    if (maxX <= 0 && maxY <= 0) {
      return { x: 0, y: 0 };
    }

    return {
      x: clampFloat(nextX, -maxX, maxX),
      y: clampFloat(nextY, -maxY, maxY),
    };
  }

  function resetTransform() {
    zoom.value = 1;
    panX.value = 0;
    panY.value = 0;
    isDragging.value = false;
    dragPointerId = null;
  }

  function onWheel(event: WheelEvent) {
    if (!options.canOperate()) {
      return;
    }
    event.preventDefault();

    const nextZoom = clampFloat(
      zoom.value + (event.deltaY < 0 ? options.zoomStep : -options.zoomStep),
      options.minZoom,
      options.maxZoom,
    );
    const clamped = clampPan(panX.value, panY.value, nextZoom);
    zoom.value = nextZoom;
    panX.value = clamped.x;
    panY.value = clamped.y;
  }

  function onPointerDown(event: PointerEvent) {
    if (!options.canOperate() || !canPan() || event.button !== 0) {
      return;
    }

    const container = event.currentTarget as HTMLElement | null;
    if (!container) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    dragPointerId = event.pointerId;
    dragStartClientX = event.clientX;
    dragStartClientY = event.clientY;
    dragStartPanX = panX.value;
    dragStartPanY = panY.value;
    isDragging.value = true;
    container.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (!isDragging.value || dragPointerId !== event.pointerId) {
      return;
    }

    const deltaX = event.clientX - dragStartClientX;
    const deltaY = event.clientY - dragStartClientY;
    const clamped = clampPan(dragStartPanX + deltaX, dragStartPanY + deltaY);
    panX.value = clamped.x;
    panY.value = clamped.y;
  }

  function onPointerUp(event: PointerEvent) {
    if (dragPointerId !== event.pointerId) {
      return;
    }

    const container = event.currentTarget as HTMLElement | null;
    if (container?.hasPointerCapture(event.pointerId)) {
      container.releasePointerCapture(event.pointerId);
    }

    isDragging.value = false;
    dragPointerId = null;
  }

  return {
    zoom,
    panX,
    panY,
    isDragging,
    canPan,
    clampPan,
    resetTransform,
    onWheel,
    onPointerDown,
    onPointerMove,
    onPointerUp,
  };
}
