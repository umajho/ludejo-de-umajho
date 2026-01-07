import Path from "node:path";

import * as Canvas2d from "./canvas-2d.ts";

const shader = await Deno.readTextFile(Path.join(import.meta.dirname!, "./copying.wgsl"));

/**
 * @author ChatGPT
 */
export class Canvas2dRenderer {
  #device: GPUDevice;

  #isOffscreen: boolean;

  #pipeline: GPURenderPipeline;
  #sampler: GPUSampler;

  #dynamic: {
    texture: GPUTexture;
    bindGroup: GPUBindGroup;
  } | null = null;

  constructor(device: GPUDevice, opts?: { isOffscreen?: boolean }) {
    this.#device = device;

    this.#isOffscreen = opts?.isOffscreen ?? false;

    const format = navigator.gpu.getPreferredCanvasFormat();

    const shaderModule = device.createShaderModule({
      code: shader,
    });

    this.#pipeline = device.createRenderPipeline({
      layout: "auto",
      vertex: {
        module: shaderModule,
        entryPoint: "vs_main",
      },
      fragment: {
        module: shaderModule,
        entryPoint: "fs_main",
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
        GPUTextureUsage.COPY_DST |
        GPUTextureUsage.COPY_SRC |
        (this.#isOffscreen
          ? 0
          : GPUTextureUsage.TEXTURE_BINDING) /* don't know why it is not `RENDER_ATTACHMENT` but this. */,
    });

    const bindGroup = this.#device.createBindGroup({
      layout: this.#pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: texture.createView() },
        { binding: 1, resource: this.#sampler },
      ],
    });

    return (this.#dynamic = { texture, bindGroup });
  }

  get texture() {
    return this.#dynamic?.texture ?? null;
  }

  render(context: GPUCanvasContext, src: Canvas2d.Canvas) {
    const width = src.width;
    const height = src.height;

    const { texture, bindGroup } = this.#updateDynamic({ width, height });

    this.#device.queue.writeTexture(
      { texture },
      src.getContext("2d").getImageData(0, 0, width, height).data,
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
    pass.draw(3);
    pass.end();

    this.#device.queue.submit([encoder.finish()]);
  }
}

export async function textureToImageData(
  texture: GPUTexture,
  opts: { device: GPUDevice; width: number; height: number },
): Promise<Canvas2d.ImageData> {
  const bytesPerPixel = 4;
  const unalignedBytesPerRow = opts.width * bytesPerPixel;
  const bytesPerRow = Math.ceil(unalignedBytesPerRow / 256) * 256;
  const bufferSize = bytesPerRow * opts.height;

  const buffer = opts.device.createBuffer({
    size: bufferSize,
    usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
  });

  const commandEncoder = opts.device.createCommandEncoder();
  commandEncoder.copyTextureToBuffer(
    { texture },
    { buffer, bytesPerRow },
    { width: opts.width, height: opts.height },
  );
  opts.device.queue.submit([commandEncoder.finish()]);

  await buffer.mapAsync(GPUMapMode.READ);

  const mapped = buffer.getMappedRange();
  const data = new Uint8Array(mapped);
  const pixels = new Uint8ClampedArray(opts.width * opts.height * bytesPerPixel);

  for (let y = 0; y < opts.height; y++) {
    const srcOffset = y * bytesPerRow;
    const dstOffset = y * unalignedBytesPerRow;
    pixels.set(data.subarray(srcOffset, srcOffset + unalignedBytesPerRow), dstOffset);
  }

  buffer.unmap();

  return new Canvas2d.ImageData(pixels, opts.width, opts.height);
}
