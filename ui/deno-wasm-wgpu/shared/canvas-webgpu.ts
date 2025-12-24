import Path from "node:path";

import * as Canvas2d from "./canvas-2d.ts";

const shader = await Deno.readTextFile(
  Path.join(import.meta.dirname!, "./canvas-2d-renderer.wgsl"),
);

/**
 * @author ChatGPT
 */
export class Canvas2dRenderer {
  #device: GPUDevice;

  #pipeline: GPURenderPipeline;
  #sampler: GPUSampler;

  #dynamic: {
    texture: GPUTexture;
    bindGroup: GPUBindGroup;
  } | null = null;

  constructor(device: GPUDevice) {
    this.#device = device;

    const format = navigator.gpu.getPreferredCanvasFormat();

    const shaderModule = device.createShaderModule({
      code: shader,
    });

    this.#pipeline = device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module: shaderModule,
        entryPoint: "vs",
      },
      fragment: {
        module: shaderModule,
        entryPoint: "fs",
        targets: [{ format }],
      },
      primitive: {
        topology: "triangle-list",
      },
    });

    this.#sampler = device.createSampler({
      magFilter: "nearest",
      minFilter: "nearest",
    });
  }

  #updateDynamic(opts: { width: number; height: number }) {
    if (
      this.#dynamic &&
      opts.width === this.#dynamic.texture.width &&
      opts.height === this.#dynamic.texture.height
    ) {
      return this.#dynamic;
    } else if (this.#dynamic) {
      this.#dynamic.texture.destroy();
    }

    const texture = this.#device.createTexture({
      size: [opts.width, opts.height],
      format: "rgba8unorm",
      usage:
        GPUTextureUsage.TEXTURE_BINDING |
        GPUTextureUsage.COPY_DST |
        GPUTextureUsage.RENDER_ATTACHMENT,
    });

    const bindGroup = this.#device.createBindGroup({
      layout: this.#pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: this.#sampler },
        { binding: 1, resource: texture.createView() },
      ],
    });

    return (this.#dynamic = { texture, bindGroup });
  }

  render(context: GPUCanvasContext, canvas2d: Canvas2d.Canvas) {
    const width = canvas2d.width;
    const height = canvas2d.height;

    const { texture, bindGroup } = this.#updateDynamic({ width, height });

    this.#device.queue.writeTexture(
      { texture: texture },
      canvas2d.getContext("2d").getImageData(0, 0, width, height).data,
      { bytesPerRow: width * 4 },
      { width, height },
    );

    const encoder = this.#device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: context.getCurrentTexture().createView(),
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });

    pass.setPipeline(this.#pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.draw(6);
    pass.end();

    this.#device.queue.submit([encoder.finish()]);
  }
}
