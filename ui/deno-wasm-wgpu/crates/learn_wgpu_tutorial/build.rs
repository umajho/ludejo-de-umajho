fn main() {
    let wesl = wesl::Wesl::new("src/drawing/shaders");

    wesl.build_artifact(
        &"package::render::model_demo".parse().unwrap(),
        "render_model_demo",
    );
    wesl.build_artifact(
        &"package::render::depth_debug".parse().unwrap(),
        "render_depth_debug",
    );
    wesl.build_artifact(
        &"package::render::model_light_source_indicator"
            .parse()
            .unwrap(),
        "render_model_light_source_indicator",
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
