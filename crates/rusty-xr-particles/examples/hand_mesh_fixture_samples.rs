use rusty_xr_particles::{
    build_fixture_hand_mesh, ColorRgba, FixtureHandMeshConfig, HandMeshSnapshot, Handedness,
    LiveHandMeshParticleSampler, MeshSurfaceCrossNeighborConfig, MeshSurfaceSampleConfig,
    RenderCoordinateSpace, TriangleMeshSurface,
};

fn main() {
    let sample_config = MeshSurfaceSampleConfig {
        point_count: 128,
        first_tier_neighbor_count: 6,
        second_tier_neighbor_count: 12,
        seed: 42,
    };
    let mesh = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
    let snapshot = hand_snapshot_from_mesh(0, &mesh, Handedness::Left);
    let mut sampler = LiveHandMeshParticleSampler::new(sample_config).with_render_style(
        RenderCoordinateSpace::World,
        0.006,
        ColorRgba::new(0.1, 0.85, 1.0, 1.0),
    );
    let first_update = sampler.update_from_snapshot(&snapshot);
    let payload = sampler.render_payload(0);

    let mut other_mesh = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
    for vertex in &mut other_mesh.vertices {
        vertex.x += 0.14;
    }
    let other_snapshot = hand_snapshot_from_mesh(0, &other_mesh, Handedness::Right);
    let mut other_sampler = LiveHandMeshParticleSampler::new(MeshSurfaceSampleConfig {
        seed: 43,
        ..sample_config
    });
    other_sampler.update_from_snapshot(&other_snapshot);
    let cross = sampler.samples().cross_neighborhood_with(
        other_sampler.samples(),
        MeshSurfaceCrossNeighborConfig {
            neighbors_per_point: 2,
            max_distance_meters: 0.18,
        },
    );

    let mut deformed_mesh = mesh.clone();
    for vertex in &mut deformed_mesh.vertices {
        vertex.z += 0.01;
    }
    let deformed_snapshot = hand_snapshot_from_mesh(1, &deformed_mesh, Handedness::Left);
    let live_update = sampler.update_from_snapshot(&deformed_snapshot);

    println!(
        "fixture hand mesh: vertices={} triangles={} samples={} first_neighbors={} second_neighbors={} cross_neighbors={} first_update={:?} live_update={:?} payload_points={}",
        mesh.vertex_count(),
        mesh.triangle_count(),
        sampler.samples().point_count(),
        sampler
            .samples()
            .first_tier_neighbors
            .first()
            .map_or(0, Vec::len),
        sampler
            .samples()
            .second_tier_neighbors
            .first()
            .map_or(0, Vec::len),
        cross.a_to_b_neighbors.first().map_or(0, Vec::len),
        first_update.status,
        live_update.status,
        payload.points.len()
    );
}

fn hand_snapshot_from_mesh(
    version: u64,
    mesh: &TriangleMeshSurface,
    handedness: Handedness,
) -> HandMeshSnapshot {
    let indices = mesh
        .triangles
        .iter()
        .map(|triangle| {
            [
                u32::try_from(triangle[0]).expect("fixture mesh index fits u32"),
                u32::try_from(triangle[1]).expect("fixture mesh index fits u32"),
                u32::try_from(triangle[2]).expect("fixture mesh index fits u32"),
            ]
        })
        .collect();
    HandMeshSnapshot::new(version, mesh.vertices.clone(), indices).with_handedness(handedness)
}
