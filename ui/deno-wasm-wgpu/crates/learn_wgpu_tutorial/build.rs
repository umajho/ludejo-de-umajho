fn main() {
    let wesl = wesl::Wesl::new("src/shaders");

    wesl.build_artifact(&"package::render::main".parse().unwrap(), "render_main");
    wesl.build_artifact(&"package::render::depth".parse().unwrap(), "render_depth");
    wesl.build_artifact(&"package::render::light".parse().unwrap(), "render_light");
    wesl.build_artifact(&"package::render::sky".parse().unwrap(), "render_sky");
    wesl.build_artifact(
        &"package::render::hdr_tonemapping".parse().unwrap(),
        "render_hdr_tonemapping",
    );

    wesl.build_artifact(
        &"package::compute::equirectangular".parse().unwrap(),
        "compute_equirectangular",
    );
}
