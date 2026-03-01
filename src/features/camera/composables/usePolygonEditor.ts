import { type Ref } from "vue";
import { clamp01, clonePolygon } from "../utils";

export function usePolygonEditor(
  framePolygon: Ref<import("../types").PolygonPoint[]>,
  previewSvgRef: Ref<SVGSVGElement | null>,
  isPaused: Ref<boolean>,
) {
  let draggingVertexIndex: number | null = null;
  let draggingPointerId: number | null = null;

  function getPreviewPointFromClient(
    clientX: number,
    clientY: number,
  ): import("../types").PolygonPoint | null {
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

  return {
    startDragVertex,
    insertVertexOnSegment,
    removeVertex,
    onWindowPointerMove,
    onWindowPointerUp,
  };
}
