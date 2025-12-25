import z from "npm:zod/v4";

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
