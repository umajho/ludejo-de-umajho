// import { createCanvas } from "jsr:@gfx/canvas@0.5.6"; // program aborts after resizing, once after ctx is used.
// import { createCanvas } from "jsr:@josefabio/deno-canvas"; // cannot resize.
// import { Canvas, CanvasRenderingContext2D, createCanvas } from "npm:canvas"; // doesn't support `rem`.

import { Canvas, SKRSContext2D, createCanvas, ImageData } from "npm:@napi-rs/canvas";

export { Canvas, type SKRSContext2D as CanvasRenderingContext2D, createCanvas, ImageData };

export function clearAndDrawCurrentTime(canvas: Canvas, font: string) {
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
}
