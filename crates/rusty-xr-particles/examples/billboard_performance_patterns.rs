use rusty_xr_particles::{
    build_morphed_ring_atlas_rgba, particle_billboard_render_budget,
    write_particle_billboard_instances, ColorRgba, MorphedRingAtlasConfig,
    ParticleBillboardBuildConfig, ParticleBillboardSortCamera, ParticleRender, ParticleSceneBasis,
    ParticleTrailConfig, ParticleTrailEmitter, Vec3, DEFAULT_PARTICLE_DISC_SEGMENTS,
};

fn main() {
    let particles = sample_particles();
    let mut sort_indices = Vec::new();
    let mut instances = Vec::new();
    let stats = write_particle_billboard_instances(
        &particles,
        ParticleSceneBasis::default(),
        ParticleBillboardBuildConfig {
            sort_back_to_front: true,
            ..ParticleBillboardBuildConfig::default()
        },
        Some(ParticleBillboardSortCamera {
            position: Vec3::ZERO,
            forward: Vec3::FORWARD_NEG_Z,
        }),
        &mut sort_indices,
        &mut instances,
    );

    let atlas = build_morphed_ring_atlas_rgba(MorphedRingAtlasConfig {
        frame_resolution: 16,
        frame_count: 8,
        atlas_columns: 4,
        ..MorphedRingAtlasConfig::default()
    });

    let mut trail_source = particles[0];
    trail_source.frame01 = 0.25;
    let mut trails = ParticleTrailEmitter::new(ParticleTrailConfig {
        enabled: true,
        visuals_enabled: true,
        lifetime_seconds: 1.0,
        copies_per_second: 10.0,
        max_spawn_batches_per_frame: 1,
        copies_per_particle: 2,
        size_multiplier: 0.75,
    });
    trails.update(0.11, &[trail_source]);
    trail_source.frame01 = 0.75;
    trails.update(0.01, &[trail_source]);
    let frozen_trail_frame = trails
        .particles()
        .iter()
        .find(|particle| particle.color.a > 0.0)
        .map(|particle| particle.frame01)
        .unwrap_or(0.0);

    let budget = particle_billboard_render_budget(
        particles.len(),
        trails.last_active_count(),
        DEFAULT_PARTICLE_DISC_SEGMENTS,
    );

    println!("billboard modes:");
    println!("  center-project: project each center once, expand the disc in clip/NDC space");
    println!("  world-vertices: expand each fan vertex in scene space, then project it");
    println!(
        "instances source={} emitted={} skipped={} first_z={:.2}",
        stats.source_count, stats.emitted_count, stats.skipped_count, instances[0].position_size[2]
    );
    println!(
        "ring atlas {}x{} frames={} rgba_bytes={}",
        atlas.width,
        atlas.height,
        atlas.frame_count,
        atlas.rgba.len()
    );
    println!(
        "trail snapshot frame01={:.2} visible_instances={} total_indices={}",
        frozen_trail_frame, budget.visible_instances, budget.total_indices
    );
}

fn sample_particles() -> Vec<ParticleRender> {
    let mut near = ParticleRender::new(Vec3::new(0.0, 0.0, -1.0), 0.08, ColorRgba::WHITE);
    near.frame01 = 0.15;

    let mut far = ParticleRender::new(
        Vec3::new(0.1, 0.0, -2.0),
        0.10,
        ColorRgba::new(0.5, 0.8, 1.0, 0.75),
    );
    far.frame01 = 0.65;

    vec![near, far]
}
