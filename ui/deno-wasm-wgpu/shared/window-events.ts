import z from "npm:zod/v4";

import TK from "npm:terminal-kit";

export const WindowEvent = z.union([
  z.tuple([z.literal("resized"), z.object({ width: z.number(), height: z.number() })]),
  z.tuple([
    z.literal("cursor_moved"),
    z.object({ position: z.object({ x: z.number(), y: z.number() }) }),
  ]),
  z.tuple([
    z.literal("mouse_input"),
    z.object({
      state: z.enum(["pressed", "released"]),
      button: z.enum(["left"]),
    }),
  ]),
  z.tuple([z.literal("keyboard_input"), z.object({ textWithAllModifiers: z.string() })]),
  z.literal("redraw_requested"),
]);
export type WindowEvent = z.infer<typeof WindowEvent>;

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
