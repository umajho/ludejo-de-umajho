import "@std/dotenv/load";

import { match, P } from "npm:ts-pattern";

import TK from "npm:terminal-kit";

import { WindowBuilder, Window, EventType } from "jsr:@divy/sdl2@0.15";

import { setProcessTitle } from "./shared/init.ts";

import * as Canvas2d from "./shared/canvas-2d.ts";
import * as CanvasGpu from "./shared/canvas-webgpu.ts";
import { WindowEvent } from "./shared/window-events.ts";
import * as WinImplTerm from "./shared/win-impl-term.ts";
import * as Sdl2 from "./shared/sdl.ts";

import init, * as w from "./crates/learn_wgpu_tutorial/dist-web-manual/learn_wgpu_tutorial.js";

setProcessTitle("deno-wasm-wgpu-c");

const SCALE = 16;
const CHARACTER_ASPECT_RATIO = 0.5;

class App {
  #terminalResizer = new Canvas2d.Resizer();
  #terminalSize: { width: number; height: number } | null = null;

  #device!: GPUDevice;
  #window!: Window;
  #surface!: Deno.UnsafeWindowSurface;
  #context!: GPUCanvasContext;

  #cursorPosition: { x: number; y: number } | null = null;

  async start() {
    await init();

    {
      this.#window = new WindowBuilder("Hello, Deno!", 1, 1)
        // .skipTaskbar() // no effect, at least on macOS.
        // .hidden() // seems unnecessary if `present` is not called?
        .build();

      const adapter = (await navigator.gpu.requestAdapter())!;
      this.#device = await adapter.requestDevice();

      this.#surface = this.#window.windowSurface(400, 400);
      this.#context = this.#surface.getContext("webgpu");

      this.#context.configure({
        device: this.#device,
        format: navigator.gpu.getPreferredCanvasFormat(),
        alphaMode: "opaque",
      });
    }

    Object.defineProperties(this.#surface, {
      width: {
        get: () => {
          const size = Sdl2.raw.SDL_GetWindowSize(this.#window);
          return size.width;
        },
        set: (newWidth: number) => {
          const size = Sdl2.raw.SDL_GetWindowSize(this.#window);
          Sdl2.raw.SDL_SetWindowSize(this.#window, newWidth, size.height);
        },
      },
      height: {
        get: () => {
          const size = Sdl2.raw.SDL_GetWindowSize(this.#window);
          return size.height;
        },
        set: (newHeight: number) => {
          const size = Sdl2.raw.SDL_GetWindowSize(this.#window);
          Sdl2.raw.SDL_SetWindowSize(this.#window, size.width, newHeight);
        },
      },
      xRequestRedraw: {
        value: () => {
          // requests here is ignored here since redrawing is handled in
          // `render`.
        },
      },
    });

    const size = Sdl2.raw.SDL_GetWindowSize(this.#window);
    await w.run_web_weblike_manual(this.#surface);
    w.handle_resized(size.width, size.height);
    // w.handle_redraw_requested();

    WinImplTerm.initialize();
    WinImplTerm.listenToWindowEvents(this.#windowEvent.bind(this));
  }

  #windowEvent(ev: WindowEvent) {
    match(ev)
      .with(["resized", P.select()], (data) => {
        const appWidth = data.width * SCALE * CHARACTER_ASPECT_RATIO;
        const appHeight = data.height * SCALE;

        this.#terminalSize = { width: data.width, height: data.height };

        Sdl2.raw.SDL_SetWindowSize(this.#window, appWidth, appHeight);
        w.handle_resized(appWidth, appHeight);
      })
      .with(["cursor_moved", P.select()], (data) => {
        this.#cursorPosition = data.position;

        // TODO!!!
      })
      .with(["mouse_input", P.select()], (data) => {
        if (data.button === "left") {
          // this.#isMousePressing = data.state === "pressed";
        }

        // TODO!!!
      })
      .with(["keyboard_input", P.select()], (data) => {
        if (data.textWithAllModifiers === "\x03") {
          TK.terminal.processExit(0);
        }

        // TODO!!!
      })
      .with("redraw_requested", () => this.render())
      .otherwise(() => {});
  }

  #isRendering = false;
  async render() {
    if (this.#isRendering) return;
    this.#isRendering = true;
    try {
      await this._render();
    } finally {
      this.#isRendering = false;
    }
  }
  async _render() {
    if (!this.#terminalSize) return;

    // const currentTexture = this.#context.getCurrentTexture();

    w.handle_redraw_requested();

    for await (const event of this.#window.events()) {
      if (event.type === EventType.Draw) {
        this.#surface.present();
        // console.log(Sdl2.raw.SDL_GetRendererOutputSize(this.#window));
        break;
      }
    }

    const windowSize = Sdl2.raw.SDL_GetWindowSize(this.#window);

    // const imageData = await CanvasGpu.textureToImageData(currentTexture, {
    //   device: this.#device,
    //   width: windowSize.width,
    //   height: windowSize.height,
    // });

    // console.log(imageData.data.filter((v) => v !== 0).length);

    // WinImplTerm.redraw(
    //   this.#terminalResizer.resizeFromImageData(
    //     imageData,
    //     this.#terminalSize.width,
    //     this.#terminalSize.height,
    //   ),
    // );
  }
}

async function main() {
  const app = new App();

  await app.start();
}

if (import.meta.main) {
  await main();
}
