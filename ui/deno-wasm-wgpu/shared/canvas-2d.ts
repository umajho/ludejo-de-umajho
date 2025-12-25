// import { createCanvas } from "jsr:@gfx/canvas@0.5.6"; // program aborts after resizing, once after ctx is used.
// import { createCanvas } from "jsr:@josefabio/deno-canvas"; // cannot resize.
// import { Canvas, CanvasRenderingContext2D, createCanvas } from "npm:canvas"; // doesn't support `rem`.
import { Canvas, SKRSContext2D, createCanvas, ImageData } from "npm:@napi-rs/canvas";

export { Canvas, type SKRSContext2D as CanvasRenderingContext2D, createCanvas, ImageData };

export function clearAndDrawCurrentTime(
  canvas: Canvas,
  font: string,
  opts?: { redSpot?: { x: number; y: number } | null },
) {
  const now = new Date();
  const hh = String(now.getHours()).padStart(2, "0");
  const mm = String(now.getMinutes()).padStart(2, "0");
  const ss = String(now.getSeconds()).padStart(2, "0");
  const subseconds = String(now.getMilliseconds()).padStart(3, "0");

  const ctx = canvas.getContext("2d");

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  ctx.fillStyle = "white";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.font = font;
  ctx.fillText(`${hh}:${mm}:${ss}.${subseconds}`, canvas.width / 2, canvas.height / 2);

  if (opts?.redSpot) {
    ctx.fillStyle = "red";
    ctx.beginPath();
    ctx.arc(opts.redSpot.x, opts.redSpot.y, 5, 0, Math.PI * 2);
    ctx.fill();
  }
}

export class Resizer {
  #canvas: Canvas = createCanvas(1, 1);
  #context: SKRSContext2D = this.#canvas.getContext("2d");

  constructor() {
    this.#context.imageSmoothingEnabled = true;
    this.#context.imageSmoothingQuality = "high";
  }

  resize(srcCanvas: Canvas, width: number, height: number): ImageData {
    if (width !== this.#canvas.width || height !== this.#canvas.height) {
      this.#canvas.width = width;
      this.#canvas.height = height;
    }

    this.#context.clearRect(0, 0, this.#canvas.width, this.#canvas.height);
    this.#context.drawImage(
      srcCanvas,
      // oxlint-disable-next-line no-useless-spread
      ...[0, 0, srcCanvas.width, srcCanvas.height],
      // oxlint-disable-next-line no-useless-spread
      ...[0, 0, width, height],
    );

    return this.#context.getImageData(0, 0, width, height);
  }

  #tmpCanvas: Canvas | null = null;

  resizeFromImageData(srcImageData: ImageData, width: number, height: number): ImageData {
    if (!this.#tmpCanvas) {
      this.#tmpCanvas = createCanvas(srcImageData.width, srcImageData.height);
    } else if (
      this.#tmpCanvas.width !== srcImageData.width ||
      this.#tmpCanvas.height !== srcImageData.height
    ) {
      this.#tmpCanvas.width = srcImageData.width;
      this.#tmpCanvas.height = srcImageData.height;
    }

    this.#tmpCanvas.getContext("2d").putImageData(srcImageData, 0, 0);

    return this.resize(this.#tmpCanvas, width, height);
  }
}
