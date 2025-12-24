import { EventType, WindowBuilder } from "jsr:@divy/sdl2@0.15";

import * as Canvas2d from "./shared/canvas-2d.ts";
import { SDL_WINDOWEVENT_RESIZED, SDL_WINDOWEVENT_SIZE_CHANGED } from "./shared/sdl.ts";
import { Canvas2dRenderer } from "./shared/canvas-webgpu.ts";

async function main() {
  const canvas2d = Canvas2d.createCanvas(640, 480);

  const window = new WindowBuilder("Hello, Deno!", canvas2d.width, canvas2d.height)
    .resizable()
    .build();

  const adapter = (await navigator.gpu.requestAdapter())!;
  const device = await adapter.requestDevice();

  const surface = window.windowSurface(canvas2d.width, canvas2d.height);
  const context = surface.getContext("webgpu");

  context.configure({
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    alphaMode: "opaque",
  });

  const renderer = new Canvas2dRenderer(device);

  for await (const event of window.events()) {
    if (event.type === EventType.Quit) {
      break;
    } else if (event.type === EventType.Draw) {
      Canvas2d.clearAndDrawCurrentTime(canvas2d, "10rem sans-serif");
      renderer.render(context, canvas2d);
      surface.present();
    } else if (
      event.type === EventType.WindowEvent &&
      (event.event === SDL_WINDOWEVENT_RESIZED || event.event === SDL_WINDOWEVENT_SIZE_CHANGED)
    ) {
      canvas2d.width = event.data1;
      canvas2d.height = event.data2;
    }
  }
}

if (import.meta.main) {
  await main();
}

Deno.exit(0);
