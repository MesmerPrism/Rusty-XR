use rusty_xr_contracts::{
    DepthMetricRange, DepthPayloadDescriptor, DepthSampleIdentityPolicy, DepthViewDescriptor,
    DepthWorldSpaceContract, DepthWorldSpaceRenderPath, Eye, FieldOfView, ImageSize, Pose, Quat,
    Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fov = FieldOfView::new(-0.72, 0.72, 0.72, -0.72);
    let left_depth_view = DepthViewDescriptor::new(
        Eye::Left,
        Pose::new(Vec3::new(-0.032, 1.55, 0.0), Quat::IDENTITY),
        fov,
    );
    let right_depth_view = DepthViewDescriptor::new(
        Eye::Right,
        Pose::new(Vec3::new(0.032, 1.55, 0.0), Quat::IDENTITY),
        fov,
    );

    let contract = DepthWorldSpaceContract::environment_depth(
        "synthetic-scene-particle-map-contract",
        DepthWorldSpaceRenderPath::SceneParticleMap,
        DepthPayloadDescriptor::new(ImageSize::new(320, 320), 320 * 320 * 2),
        DepthMetricRange::new(0.1, 100.0),
        left_depth_view,
        right_depth_view,
    )
    .with_runtime_capture_time_ns(123_456_789)
    .with_depth_texture_transform("rotate0+flipY")
    .with_reference_space("LOCAL")
    .with_projection_y_convention("vulkan-positive-viewport-y-flipped-in-shader")
    .with_render_target_size(ImageSize::new(1832, 1920))
    .with_sample_identity_policy(DepthSampleIdentityPolicy::ReferenceSpaceCell)
    .with_passthrough_visible(true);

    assert!(contract.is_valid());
    println!("{}", serde_json::to_string_pretty(&contract)?);
    Ok(())
}
