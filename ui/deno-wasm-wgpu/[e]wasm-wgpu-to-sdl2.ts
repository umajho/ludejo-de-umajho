import "@std/dotenv/load";

import { EventType, WindowBuilder } from "jsr:@divy/sdl2@0.15";

import { setProcessTitle } from "./shared/init.ts";

import { SDL_WINDOWEVENT_RESIZED, SDL_WINDOWEVENT_SIZE_CHANGED } from "./shared/sdl.ts";

import init, * as w from "./crates/learn_wgpu_tutorial/dist-web-manual/learn_wgpu_tutorial.js";

setProcessTitle("deno-wasm-wgpu-b");

async function main() {
  await init();

  const window = new WindowBuilder("Hello, Deno!", 640, 480)
    //.resizable()
    .build();

  const adapter = (await navigator.gpu.requestAdapter())!;
  const device = await adapter.requestDevice();

  const surface = window.windowSurface(640, 480);
  const context = surface.getContext("webgpu");

  context.configure({
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    alphaMode: "opaque",
  });

  Object.defineProperties(surface, {
    width: {
      get: () => 640,
      set: (_v: number) => {},
    },
    height: {
      get: () => 480,
      set: (_v: number) => {},
    },
    xRequestRedraw: {
      value: () => {},
    },
  });

  await w.run_web_weblike_manual(surface);
  w.handle_resized(640, 480);
  w.handle_redraw_requested();

  for await (const event of window.events()) {
    if (event.type === EventType.Quit) {
      break;
    } else if (event.type === EventType.Draw) {
      w.handle_redraw_requested();
      surface.present();
    } else if (
      event.type === EventType.WindowEvent &&
      (event.event === SDL_WINDOWEVENT_RESIZED || event.event === SDL_WINDOWEVENT_SIZE_CHANGED)
    ) {
      w.handle_resized(event.data1, event.data2);
    }
  }
}

if (import.meta.main) {
  await main();
}

Deno.exit(0);
