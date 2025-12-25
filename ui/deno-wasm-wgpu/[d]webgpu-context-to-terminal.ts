import "@std/dotenv/load";

import { match, P } from "npm:ts-pattern";

import TK from "npm:terminal-kit";

import { WindowBuilder, Window } from "jsr:@divy/sdl2@0.15";

import { setProcessTitle } from "./shared/init.ts";

import * as Canvas2d from "./shared/canvas-2d.ts";
import * as CanvasGpu from "./shared/canvas-webgpu.ts";
import { WindowEvent } from "./shared/window-events.ts";
import * as WinImplTerm from "./shared/win-impl-term.ts";
import * as Sdl2 from "./shared/sdl.ts";

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
  #renderer!: CanvasGpu.Canvas2dRenderer;
  #textHelperCanvas!: Canvas2d.Canvas;

  #cursorPosition: { x: number; y: number } | null = null;

  #isMousePressing = false;
  #redSpot: { x: number; y: number } | null = null;

  async start() {
    {
      this.#window = new WindowBuilder("Hello, Deno!", 1, 1)
        // .skipTaskbar() // no effect, at least on macOS.
        .hidden() // seems unnecessary?
        .build();

      const adapter = (await navigator.gpu.requestAdapter())!;
      this.#device = await adapter.requestDevice();

      this.#surface = this.#window.windowSurface(1, 1);
      this.#context = this.#surface.getContext("webgpu");

      this.#context.configure({
        device: this.#device,
        format: navigator.gpu.getPreferredCanvasFormat(),
        alphaMode: "opaque",
      });

      this.#renderer = new CanvasGpu.Canvas2dRenderer(this.#device);

      this.#textHelperCanvas = Canvas2d.createCanvas(1, 1);
    }

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
      })
      .with(["cursor_moved", P.select()], (data) => {
        this.#cursorPosition = data.position;

        this.#updateRedSpot();
      })
      .with(["mouse_input", P.select()], (data) => {
        if (data.button === "left") {
          this.#isMousePressing = data.state === "pressed";
        }

        this.#updateRedSpot();
      })
      .with(["keyboard_input", P.select()], (data) => {
        if (data.textWithAllModifiers === "\x03") {
          TK.terminal.processExit(0);
        }
      })
      .with("redraw_requested", () => this.render())
      .otherwise(() => {});
  }

  #updateRedSpot() {
    if (this.#cursorPosition && this.#isMousePressing) {
      this.#redSpot = { ...this.#cursorPosition };
    } else {
      this.#redSpot = null;
    }
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

    const windowSize = Sdl2.raw.SDL_GetWindowSize(this.#window);
    if (
      windowSize.width !== this.#textHelperCanvas.width ||
      windowSize.height !== this.#textHelperCanvas.height
    ) {
      this.#textHelperCanvas.width = windowSize.width;
      this.#textHelperCanvas.height = windowSize.height;
    }

    Canvas2d.clearAndDrawCurrentTime(this.#textHelperCanvas, "10rem monospace", {
      redSpot: this.#redSpot
        ? {
            x: (this.#redSpot.x + 0.5) * SCALE * CHARACTER_ASPECT_RATIO,
            y: (this.#redSpot.y + 0.5) * SCALE,
          }
        : null,
    });

    this.#renderer.render(this.#context, this.#textHelperCanvas);

    const imageData = await CanvasGpu.textureToImageData(this.#renderer.texture!, {
      device: this.#device,
      width: windowSize.width,
      height: windowSize.height,
    });

    WinImplTerm.redraw(
      this.#terminalResizer.resizeFromImageData(
        imageData,
        this.#terminalSize.width,
        this.#terminalSize.height,
      ),
    );
  }
}

async function main() {
  const app = new App();

  await app.start();
}

if (import.meta.main) {
  await main();
}
