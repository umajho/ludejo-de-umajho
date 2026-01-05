import init, { Runner } from "./dist-web-manual/ab3de_engine.js";

await init();
console.log("Initialized WASM.");

let offscreenCanvas;
let runner;

let hasRequestedRedraw = false;

onmessage = async (ev) => {
  switch (ev.data[0]) {
    case "start": {
      const [_, offscreenCanvas_] = ev.data;
      offscreenCanvas = offscreenCanvas_;
      Object.defineProperty(offscreenCanvas, "xRequestRedraw", {
        value: () => {
          hasRequestedRedraw = true;
        },
      });
      runner = new Runner(offscreenCanvas);
      await runner.start();
      postMessage("started");
      break;
    }
    case "command": {
      const [_, name, ...args] = ev.data;
      if (name === "handle_resized") {
        offscreenCanvas.width = args[0];
        offscreenCanvas.height = args[1];
      }
      runner[name](...args);
      break;
    }
  }
};

const tickFrame = () => {
  if (hasRequestedRedraw) {
    hasRequestedRedraw = false;
    runner.handle_redraw_requested();
  }
  requestAnimationFrame(tickFrame);
};
tickFrame();

postMessage("ready");
