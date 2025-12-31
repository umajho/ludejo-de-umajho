fn main() {
    let wesl = wesl::Wesl::new("src/drawing/shaders");

    wesl.build_artifact(&"package::render::model".parse().unwrap(), "render_model");
    wesl.build_artifact(
        &"package::render::depth_debug".parse().unwrap(),
        "render_depth_debug",
    );
    wesl.build_artifact(
        &"package::render::light_source_indicator_model"
            .parse()
            .unwrap(),
        "render_light_source_indicator_model",
    );
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
