pub fn r_model_demo(device: &wgpu::Device) -> RenderShader {
    RenderShader(device.create_shader_module(include_wesl_desc!("render_model_demo")))
}

pub fn r_depth_debug(device: &wgpu::Device) -> RenderShader {
    RenderShader(device.create_shader_module(include_wesl_desc!("render_depth_debug")))
}

pub fn r_model_light_source_indicator(device: &wgpu::Device) -> RenderShader {
    RenderShader(
        device.create_shader_module(include_wesl_desc!("render_model_light_source_indicator")),
    )
}

pub fn r_sky(device: &wgpu::Device) -> RenderShader {
    RenderShader(device.create_shader_module(include_wesl_desc!("render_sky")))
}

pub fn r_hdr_tonemapping(device: &wgpu::Device) -> RenderShader {
    RenderShader(device.create_shader_module(include_wesl_desc!("render_hdr_tonemapping")))
}

pub fn c_equirectangular(device: &wgpu::Device) -> ComputeShaderComputeEquirectToCubemap {
    ComputeShaderComputeEquirectToCubemap(
        device.create_shader_module(include_wesl_desc!("compute_equirectangular")),
    )
}

pub struct RenderShader(wgpu::ShaderModule);

impl RenderShader {
    pub fn vertex_state<'a>(&'a self, opts: VertexStatePartial<'a>) -> wgpu::VertexState<'a> {
        wgpu::VertexState {
            module: &self.0,
            entry_point: Some("vs_main"),
            compilation_options: opts.compilation_options,
            buffers: opts.buffers,
        }
    }

    pub fn fragment_state<'a>(
        &'a self,
        opts: FragmentStatePartial<'a>,
    ) -> Option<wgpu::FragmentState<'a>> {
        Some(wgpu::FragmentState {
            module: &self.0,
            entry_point: Some("fs_main"),
            targets: opts.targets,
            compilation_options: opts.compilation_options,
        })
    }
}

pub struct VertexStatePartial<'a> {
    pub compilation_options: wgpu::PipelineCompilationOptions<'a>,
    pub buffers: &'a [wgpu::VertexBufferLayout<'a>],
}

pub struct FragmentStatePartial<'a> {
    pub targets: &'a [Option<wgpu::ColorTargetState>],
    pub compilation_options: wgpu::PipelineCompilationOptions<'a>,
}

pub struct ComputeShaderComputeEquirectToCubemap(wgpu::ShaderModule);

impl ComputeShaderComputeEquirectToCubemap {
    pub fn compute_pipeline_descriptor<'a>(
        &'a self,
        opts: ComputePipelineDescriptorPartial<'a>,
    ) -> wgpu::ComputePipelineDescriptor<'a> {
        wgpu::ComputePipelineDescriptor {
            label: opts.label,
            layout: opts.layout,
            module: &self.0,
            entry_point: Some("compute_equirect_to_cubemap"),
            compilation_options: opts.compilation_options,
            cache: opts.cache,
        }
    }
}

pub struct ComputePipelineDescriptorPartial<'a> {
    pub label: Option<&'a str>,
    pub layout: Option<&'a wgpu::PipelineLayout>,
    pub compilation_options: wgpu::PipelineCompilationOptions<'a>,
    pub cache: Option<&'a wgpu::PipelineCache>,
}

/// based on [`wgpu::include_wgsl`].
macro include_wesl_desc($($token:tt)*) {
    {
        wgpu::ShaderModuleDescriptor {
            label: Some($($token)*),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(wesl::include_wesl!($($token)*))),
        }
    }
}
