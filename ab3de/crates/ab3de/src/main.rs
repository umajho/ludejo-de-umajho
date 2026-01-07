#![feature(decl_macro)]

use clap::Parser;

mod ui;
mod uiless;
mod utils;

/// A Bland 3D Engine.
#[derive(clap::Parser)]
struct Args {
    /// Run without a UI.
    #[arg(long, action)]
    uiless: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    if args.uiless {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let request_adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None, // might be overriden.
            force_fallback_adapter: false,
        };
        let device_descriptor = wgpu::DeviceDescriptor {
            label: None, // will be set later.
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::defaults(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        };

        Ok((uiless::run(instance, request_adapter_options, device_descriptor))?)
    } else {
        Ok(ui::run()?)
    }
}
