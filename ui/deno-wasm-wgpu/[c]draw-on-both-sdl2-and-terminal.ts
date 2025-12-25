import "@std/dotenv/load";

import { match, P } from "npm:ts-pattern";

import TK from "npm:terminal-kit";

import { EventType, WindowBuilder, Window } from "jsr:@divy/sdl2@0.15";

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
  #appCanvas: Canvas2d.Canvas;
  // #appCtx: Canvas2d.CanvasRenderingContext2D;

  #terminalResizer = new Canvas2d.Resizer();
  #terminalSize: { width: number; height: number } | null = null;

  #window!: Window;
  #surface!: Deno.UnsafeWindowSurface;
  #context!: GPUCanvasContext;
  #renderer!: CanvasGpu.Canvas2dRenderer;

  #cursorPosition: { x: number; y: number } | null = null;

  #isMousePressing = false;
  #redSpot: { x: number; y: number } | null = null;

  constructor() {
    this.#appCanvas = Canvas2d.createCanvas(1, 1);
    // this.#appCtx = this.#appCanvas.getContext("2d");
  }

  async start() {
    {
      this.#window = new WindowBuilder(
        "Hello, Deno!",
        this.#appCanvas.width,
        this.#appCanvas.height,
      )
        .alwaysOnTop()
        .build();

      const adapter = (await navigator.gpu.requestAdapter())!;
      const device = await adapter.requestDevice();

      this.#surface = this.#window.windowSurface(400, 400);
      this.#context = this.#surface.getContext("webgpu");

      this.#context.configure({
        device,
        format: navigator.gpu.getPreferredCanvasFormat(),
        alphaMode: "opaque",
      });

      this.#renderer = new CanvasGpu.Canvas2dRenderer(device);
    }

    WinImplTerm.initialize();
    WinImplTerm.listenToWindowEvents(this.#windowEvent.bind(this));
  }

  #windowEvent(ev: WindowEvent) {
    match(ev)
      .with(["resized", P.select()], (data) => {
        const appWidth = data.width * SCALE * CHARACTER_ASPECT_RATIO;
        const appHeight = data.height * SCALE;

        this.#appCanvas.width = appWidth;
        this.#appCanvas.height = appHeight;
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

    Canvas2d.clearAndDrawCurrentTime(this.#appCanvas, "10rem monospace", {
      redSpot: this.#redSpot
        ? {
            x: (this.#redSpot.x + 0.5) * SCALE * CHARACTER_ASPECT_RATIO,
            y: (this.#redSpot.y + 0.5) * SCALE,
          }
        : null,
    });

    for await (const event of this.#window.events()) {
      if (event.type === EventType.Draw) {
        this.#renderer.render(this.#context, this.#appCanvas);
        this.#surface.present();
        break;
      }
    }

    WinImplTerm.redraw(
      this.#terminalResizer.resize(
        this.#appCanvas,
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
