# ab3de: A bland 3D explorer

A toy 3D toolkit I'm building to learn about various topics.

## TODOs

- [ ] FIXME: camera movement jittering.
  - see: <https://github.com/sotrh/learn-wgpu/issues/294>.
  - This has been mitigated on the `wasm-weblike-manual` build since the render
    process is moved to a web worker, although I think there is still some room
    for improvement.
  - For the native app, rendering has also been moved to a separate thread, but
    I don't see much improvement.
- [ ] FIXME: on Zen (Firefox 146), resizing usually freezes the whole browser
      for a while. When the browser recovers, the rendering stops working.

## Current Goals

- [ ] Add egui intergration to learn egui.
  - Frontend (ab3de_ui) and backend (ab3de_engine) should be separated, so that
    the engine can be used without a UI.
    - For example, to make the engine available for remotion or FrameScript.
    - The architecture should allow UI to be attachable. For example, we can run
      an engine in a SharedWorker for remotion, and it should be possible to
      open another window, where another camera view with UI attached is used to
      view the same engine's scene.
- [ ] Support proper file management. Don't embed resources in the binary.
- [ ] Improve `.pmx` support.
  - [ ] Handle toon and sphere textures.
  - [ ] Handle light properly.
  - [ ] Add shadow support.
  - [ ] Implement a `.pmx` parser (with `nom`?) to replace the unmaintained
        `mmd` crate.
  - [ ] Add UI to inspect `.pmx` file contents.
  - [ ] Support physics. (`rapier3d` or `wgrapier3d`? Should be deterministic.)
- [ ] remotion/FrameScript integration.

## Coding conventions

Function parameter ordering:

- label and name
- context parameters (e.g., device, queue)
- other parameters

## Credits

- [learn-wgpu]: This project's codebase is literally derived from this tutorial
  by Benjamin Hansen and other contributors. (That's the reason why a copy of
  its license is included in [LICENSE.md].)

[learn-wgpu]: https://github.com/sotrh/learn-wgpu
[LICENSE.md]: ./LICENSE.md
