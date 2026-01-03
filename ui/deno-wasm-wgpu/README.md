## TODOs

### `[f]wasm-wgpu-to-terminal.ts`

Goal: With Deno, do WebGPU rendering in WASM, and retrieve the render results as
pixel data, so that they can be drawn in a terminal.

Failed attempts:

- getting the surface textures used in WASM on the JS side, and copying their
  data to buffers: `GPUCanvasContext.prototype.getCurrentTexture` returns new
  textures, instead of the ones used in WASM.
- getting pixel data from SDL2: `SDL_GetRendererOutputSize` reports zero width
  and height, which means `SDL_RenderReadPixels` won't work. It seems that this
  is because WebGPU takes over the work of SDL2's renderer.

Real solutions:

- Drawing without a window, and copying the data out of the destination texture.
  See: [Wgpu without a window].
- Waiting for Deno to support `OffscreenCanvas`. See:
  https://github.com/denoland/deno/issues/5701

I decide to just wait.

NOTE: the unfinished work is in the branch
`feature/deno-wasm-wgpu-x/wasm-wgpu-to-terminal`.

[Wgpu without a window]: https://sotrh.github.io/learn-wgpu/showcase/windowless/
