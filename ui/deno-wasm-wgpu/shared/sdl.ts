import { Window } from "@divy/sdl2";

export const SDL_WINDOWEVENT_RESIZED = 0x05;
export const SDL_WINDOWEVENT_SIZE_CHANGED = 0x06;

/**
 * implement functionalities not imported from `deno_sdl2`.
 */
//! MIT License
//!
//! Copyright (c) 2021-2024 Divy Srivastava
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
// oxlint-disable
export const raw = (() => {
  let DENO_SDL2_PATH: string | undefined;
  try {
    DENO_SDL2_PATH = Deno.env.get("DENO_SDL2_PATH");
  } catch (_) {
    // ignore, this can only fail if permission is not given
  }

  const OS_PREFIX = Deno.build.os === "windows" ? "" : "lib";
  const OS_SUFFIX =
    Deno.build.os === "windows" ? ".dll" : Deno.build.os === "darwin" ? ".dylib" : ".so";

  function getLibraryPath(lib: string): string {
    lib = `${OS_PREFIX}${lib}${OS_SUFFIX}`;
    if (DENO_SDL2_PATH) {
      return `${DENO_SDL2_PATH}/${lib}`;
    } else {
      return lib;
    }
  }

  const sdl2 = Deno.dlopen(getLibraryPath("SDL2"), {
    SDL_GetWindowSize: {
      parameters: ["pointer", "pointer", "pointer"],
      result: "i32",
    },
    SDL_SetWindowSize: {
      parameters: ["pointer", "i32", "i32"],
      result: "i32",
    },
  });

  function getWindowRawPointer(window: Window): Deno.PointerValue {
    return window["raw"];
  }

  return {
    SDL_GetWindowSize: (window: Window) => {
      const wBuf = new Uint32Array(1);
      const hBuf = new Uint32Array(1);
      const wPtr = Deno.UnsafePointer.of(wBuf);
      const hPtr = Deno.UnsafePointer.of(hBuf);
      sdl2.symbols.SDL_GetWindowSize(getWindowRawPointer(window), wPtr, hPtr);
      return { width: wBuf[0], height: hBuf[0] };
    },
    SDL_SetWindowSize: (window: Window, width: number, height: number) => {
      return sdl2.symbols.SDL_SetWindowSize(getWindowRawPointer(window), width, height);
    },
  };
})();
