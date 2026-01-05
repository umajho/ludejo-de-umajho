## TODOs

- [ ] FIXME: camera movement jittering.
  - see: <https://github.com/sotrh/learn-wgpu/issues/294>.
  - This has been mitigated on the `wasm-weblike-manual` build since the render
    process is moved to a web worker, although I think there is still some room
    for improvement.
  - The same mitigation could also happen on the native build after I move the
    rendering to a separate thread.
- [ ] FIXME: on Zen (Firefox 146), resizing usually freezes the whole browser
      for a while. When the browser recovers, the rendering stops working.

## Coding conventions

Function parameter ordering:

- label and name
- context parameters (e.g., device, queue)
- other parameters
