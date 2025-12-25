import TK from "npm:terminal-kit";
import ansiEscapes from "npm:ansi-escapes";
import ansiStyles from "npm:ansi-styles";

import * as Canvas2d from "./canvas-2d.ts";
import { WindowEvent } from "./window-events.ts";

const TEXT_ENCODER = new TextEncoder();

export function initialize() {
  Deno.stdout.writeSync(
    TEXT_ENCODER.encode(
      [ansiEscapes.enterAlternativeScreen, ansiEscapes.clearTerminal, ansiEscapes.cursorHide].join(
        "",
      ),
    ),
  );
}

export function listenToWindowEvents(rx: (ev: WindowEvent) => void) {
  let oldConsoleSize: { columns: number; rows: number } | null = null;
  setInterval(() => {
    const consoleSize = Deno.consoleSize();
    if (
      oldConsoleSize?.columns !== consoleSize.columns ||
      oldConsoleSize?.rows !== consoleSize.rows
    ) {
      rx(["resized", { width: consoleSize.columns, height: consoleSize.rows }]);
      oldConsoleSize = consoleSize;
    }

    rx("redraw_requested");
  }, 1000 / 60);

  TK.terminal.on("key", (name: string, _matches: string[], _data: unknown) => {
    if (name === "CTRL_C") {
      rx([
        "keyboard_input",
        {
          get textWithAllModifiers() {
            return "\x03";
          },
        },
      ]);
    }
  });

  TK.terminal.grabInput({ mouse: "motion" });
  let lastMousePosition: { x: number; y: number } | null = null;
  TK.terminal.on("mouse", (name: string, data: { x: number; y: number }) => {
    if (lastMousePosition?.x !== data.x || lastMousePosition?.y !== data.y) {
      rx(["cursor_moved", { position: { x: data.x - 1, y: data.y - 1 } }]);
      lastMousePosition = { x: data.x, y: data.y };
    }
    if (name === "MOUSE_LEFT_BUTTON_PRESSED") {
      rx(["mouse_input", { button: "left", state: "pressed" }]);
    } else if (name === "MOUSE_LEFT_BUTTON_RELEASED") {
      rx(["mouse_input", { button: "left", state: "released" }]);
    }
  });
}

export function redraw(imgData: Canvas2d.ImageData | ImageData) {
  let img = "";

  for (let y = 0; y < imgData.height; y++) {
    for (let x = 0; x < imgData.width; x++) {
      const offset = (y * imgData.width + x) * 4;
      const p = imgData.data.subarray(offset, offset + 3);
      img += ansiStyles.bgColor.ansi256(ansiStyles.rgbToAnsi256(p[0], p[1], p[2])) + " ";
    }
  }

  Deno.stdout.writeSync(TEXT_ENCODER.encode(ansiEscapes.cursorTo(0, 0) + img));
}
