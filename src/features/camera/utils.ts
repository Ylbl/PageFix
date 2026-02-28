import type { PolygonPoint } from "./types";

export function clamp(value: number, min: number, max: number) {
  const n = Number.isFinite(value) ? Math.floor(value) : min;
  return Math.max(min, Math.min(max, n));
}

export function clampFloat(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.max(min, Math.min(max, value));
}

export function clamp01(value: number) {
  return Math.max(0, Math.min(1, value));
}

export function clonePolygon(points: PolygonPoint[]) {
  return points.map((point) => ({
    x: Number.isFinite(point.x) ? point.x : 0,
    y: Number.isFinite(point.y) ? point.y : 0,
  }));
}

export function polygonSegments(points: PolygonPoint[]) {
  if (points.length < 2) {
    return [];
  }

  return points.map((from, index) => ({
    index,
    from,
    to: points[(index + 1) % points.length],
  }));
}

export function polygonPointsForSvg(points: PolygonPoint[]) {
  return points.map((p) => `${(p.x * 100).toFixed(3)},${(p.y * 100).toFixed(3)}`).join(" ");
}

export function normalizeError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "调用失败，请重试。";
}

export function defaultSnapshotPolygon(): PolygonPoint[] {
  return [
    { x: 0.08, y: 0.08 },
    { x: 0.92, y: 0.08 },
    { x: 0.92, y: 0.92 },
    { x: 0.08, y: 0.92 },
  ];
}
