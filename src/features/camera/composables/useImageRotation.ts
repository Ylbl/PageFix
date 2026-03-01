import { type Ref } from "vue";
import { normalizeError } from "../utils";

export function useImageRotation(
  capturedFrameUrl: Ref<string>,
  capturedAspectRatio: Ref<number>,
  errorMessage: Ref<string>,
  capturedPanZoom: { clampPan: (x: number, y: number, z?: number) => { x: number; y: number } },
  capturedPanX: Ref<number>,
  capturedPanY: Ref<number>,
  capturedZoom: Ref<number>,
) {
  function getCapturedMimeType(dataUrl: string) {
    const match = /^data:([^;,]+)[;,]/i.exec(dataUrl);
    return match?.[1] ?? "image/png";
  }

  function loadDataUrlImage(dataUrl: string) {
    return new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error("旋转失败：无法读取截图数据。"));
      image.src = dataUrl;
    });
  }

  function onCapturedImageLoad(event: Event) {
    const image = event.target as HTMLImageElement | null;
    if (!image || image.naturalWidth <= 0 || image.naturalHeight <= 0) {
      return;
    }
    capturedAspectRatio.value = image.naturalWidth / image.naturalHeight;
  }

  async function rotateCapturedBy(degrees: number) {
    if (!capturedFrameUrl.value) {
      return;
    }
    const normalized = ((Math.round(degrees) % 360) + 360) % 360;
    if (![90, 180, 270].includes(normalized)) {
      return;
    }
    try {
      const current = capturedFrameUrl.value;
      const image = await loadDataUrlImage(current);
      const quarterTurn = normalized === 90 || normalized === 270;
      const canvas = document.createElement("canvas");
      canvas.width = quarterTurn ? image.naturalHeight : image.naturalWidth;
      canvas.height = quarterTurn ? image.naturalWidth : image.naturalHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        throw new Error("旋转失败：无法创建绘图上下文。");
      }
      ctx.translate(canvas.width / 2, canvas.height / 2);
      ctx.rotate((degrees * Math.PI) / 180);
      ctx.drawImage(image, -image.naturalWidth / 2, -image.naturalHeight / 2);
      const mime = getCapturedMimeType(current);
      capturedFrameUrl.value =
        mime === "image/jpeg" ? canvas.toDataURL(mime, 0.98) : canvas.toDataURL(mime);
    } catch (error) {
      errorMessage.value = normalizeError(error);
    }
    const clamped = capturedPanZoom.clampPan(capturedPanX.value, capturedPanY.value, capturedZoom.value);
    capturedPanX.value = clamped.x;
    capturedPanY.value = clamped.y;
  }

  function rotateCapturedLeft() {
    void rotateCapturedBy(-90);
  }

  function rotateCapturedRight() {
    void rotateCapturedBy(90);
  }

  return {
    onCapturedImageLoad,
    rotateCapturedLeft,
    rotateCapturedRight,
  };
}
