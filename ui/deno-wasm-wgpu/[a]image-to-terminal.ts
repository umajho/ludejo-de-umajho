import { match, P } from "npm:ts-pattern";

import ansiEscapes from "npm:ansi-escapes";
import ansiStyles from "npm:ansi-styles";
import TK from "npm:terminal-kit";

import * as Canvas2d from "./shared/canvas-2d.ts";

import { listenToWindowEvents, WindowEvent } from "./shared/window-events.ts";

const SCALE = 16;
const CHARACTER_ASPECT_RATIO = 0.5;

const TEXT_ENCODER = new TextEncoder();

class App {
  #appCanvas: Canvas2d.Canvas;
  // #appCtx: Canvas2d.CanvasRenderingContext2D;

  #terminalCanvas: Canvas2d.Canvas;
  #terminalCtx: Canvas2d.CanvasRenderingContext2D;

  #cursorPosition: { x: number; y: number } | null = null;

  #isMousePressing = false;
  #redSpot: { x: number; y: number } | null = null;

  constructor() {
    Deno.stdout.writeSync(
      TEXT_ENCODER.encode(
        [
          ansiEscapes.enterAlternativeScreen,
          ansiEscapes.clearTerminal,
          ansiEscapes.cursorHide,
        ].join(""),
      ),
    );

    this.#appCanvas = Canvas2d.createCanvas(1, 1);
    // this.#appCtx = this.#appCanvas.getContext("2d");

    this.#terminalCanvas = Canvas2d.createCanvas(1, 1);
    this.#terminalCtx = this.#terminalCanvas.getContext("2d");
    this.#terminalCtx.imageSmoothingEnabled = true;
    this.#terminalCtx.imageSmoothingQuality = "high";
  }

  start() {
    listenToWindowEvents(this.#windowEvent.bind(this));
  }

  #windowEvent(ev: WindowEvent) {
    match(ev)
      .with(["resized", P.select()], (data) => {
        const appWidth = data.width * SCALE * CHARACTER_ASPECT_RATIO;
        const appHeight = data.height * SCALE;

        this.#appCanvas.width = appWidth;
        this.#appCanvas.height = appHeight;
        this.#terminalCanvas.width = data.width;
        this.#terminalCanvas.height = data.height;
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

  render() {
    Canvas2d.clearAndDrawCurrentTime(this.#appCanvas, "10rem monospace");

    this.#terminalCtx.clearRect(0, 0, this.#terminalCanvas.width, this.#terminalCanvas.height);
    this.#terminalCtx.drawImage(
      this.#appCanvas,
      // oxlint-disable-next-line no-useless-spread
      ...[0, 0, this.#appCanvas.width, this.#appCanvas.height],
      // oxlint-disable-next-line no-useless-spread
      ...[0, 0, this.#terminalCanvas.width, this.#terminalCanvas.height],
    );

    let img = "";
    const imgData = this.#terminalCtx.getImageData(
      0,
      0,
      this.#terminalCanvas.width,
      this.#terminalCanvas.height,
    ).data;
    for (let y = 0; y < this.#terminalCanvas.height; y++) {
      for (let x = 0; x < this.#terminalCanvas.width; x++) {
        const offset = (y * this.#terminalCanvas.width + x) * 4;
        const p = imgData.subarray(offset, offset + 3);
        const char =
          x === this.#redSpot?.x && y === this.#redSpot?.y
            ? ansiStyles.color.red.open + "⬤" + ansiStyles.color.close
            : " ";
        img += ansiStyles.bgColor.ansi256(ansiStyles.rgbToAnsi256(p[0], p[1], p[2])) + char;
      }
    }

    Deno.stdout.writeSync(TEXT_ENCODER.encode(ansiEscapes.cursorTo(0, 0) + img));
  }
}

function main() {
  const app = new App();

  app.start();
}

if (import.meta.main) {
  main();
}
