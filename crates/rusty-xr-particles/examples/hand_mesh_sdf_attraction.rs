use rusty_xr_particles::{
    build_mesh_sdf_particle_attraction_scenario, build_sdf_from_mesh_bounds,
    build_synthetic_hand_mesh, CameraPose, MeshSdfParticleAttractionScenarioConfig,
    MeshSdfSignMode, MeshToSdfConfig, ParticleSphereSpawnConfig, SdfParticleAttractionConfig,
    SyntheticHandMeshConfig, TriangleMeshSnapshot, TriangleMeshSurface, Vec3,
};

fn main() {
    let camera = CameraPose::new(
        Vec3::new(0.0, 1.55, 0.0),
        Vec3::new(0.0, -0.05, -1.0),
        Vec3::UP,
    );
    let scenario_config = MeshSdfParticleAttractionScenarioConfig {
        spawn: ParticleSphereSpawnConfig {
            count: 2048,
            distance_meters: 1.05,
            radius_meters: 0.24,
            particle_radius_meters: 0.006,
            vertical_offset_meters: -0.05,
            yaw_only: true,
        },
        mesh_sdf: MeshToSdfConfig {
            voxel_size_meters: 0.035,
            padding_meters: 0.18,
            max_voxels: 512 * 512,
            sign_mode: MeshSdfSignMode::TriangleNormal,
            ..MeshToSdfConfig::default()
        },
        attraction: SdfParticleAttractionConfig {
            strength: 5.0,
            attraction_distance_meters: 1.5,
            max_speed_meters_per_second: 1.4,
            max_extrapolation_meters: 0.08,
            ..SdfParticleAttractionConfig::default()
        },
        ..MeshSdfParticleAttractionScenarioConfig::default()
    };

    let initial_hand = moving_hand_mesh(1, Vec3::new(0.0, 1.35, -0.72));
    let scenario =
        build_mesh_sdf_particle_attraction_scenario(1, camera, &initial_hand, scenario_config)
            .expect("synthetic hand SDF scenario should build");
    let mut particles = scenario.particles;
    let mut sdf = scenario.sdf;
    let attraction_config = scenario.attraction_config;
    let particle_spawn_center = scenario.particle_spawn_center;

    for frame in 0..120 {
        if frame % 20 == 0 {
            let hand_offset = ((frame as f32) * 0.05).sin() * 0.04;
            let hand_mesh = moving_hand_mesh(frame as u64 + 2, Vec3::new(hand_offset, 1.35, -0.72));
            let mesh_bounds = hand_mesh
                .bounds()
                .expect("synthetic hand should have bounds");
            let sdf_bounds = mesh_bounds
                .include_sphere(particle_spawn_center, scenario_config.spawn.radius_meters);
            sdf = build_sdf_from_mesh_bounds(
                frame as u64 + 2,
                &hand_mesh,
                scenario_config.mesh_sdf,
                sdf_bounds,
            )
            .expect("updated synthetic hand SDF should build");
        }

        let stats = rusty_xr_particles::step_particles_toward_sdf(
            &mut particles,
            &sdf,
            1.0 / 72.0,
            attraction_config,
        );

        if frame % 30 == 0 {
            let payload = particles.render_payload(
                frame as u64,
                rusty_xr_particles::RenderCoordinateSpace::World,
            );
            println!(
                "frame={frame:03} particles={} sdf_samples={} affected={} max_speed={:.3} sdf_voxels={}",
                payload.points.len(),
                stats.sampled_count,
                stats.affected_count,
                stats.max_speed_observed,
                sdf.voxel_count(),
            );
        }
    }
}

fn moving_hand_mesh(version: u64, center: Vec3) -> TriangleMeshSnapshot {
    let mut surface = build_synthetic_hand_mesh(SyntheticHandMeshConfig::default());
    translate_surface(&mut surface, center);
    surface
        .to_triangle_mesh_snapshot(version)
        .expect("synthetic hand mesh indices should fit u32")
}

fn translate_surface(surface: &mut TriangleMeshSurface, offset: Vec3) {
    for vertex in &mut surface.vertices {
        *vertex += offset;
    }
}
