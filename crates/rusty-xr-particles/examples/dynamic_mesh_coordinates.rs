use rusty_xr_particles::{
    ColorRgba, LiveMeshSurfaceSampler, MeshSurfaceCrossNeighborConfig, MeshSurfaceSampleConfig,
    RenderCoordinateSpace, TriangleMeshSurface, Vec3,
};

fn main() {
    let config = MeshSurfaceSampleConfig {
        point_count: 48,
        first_tier_neighbor_count: 4,
        second_tier_neighbor_count: 8,
        seed: 2026,
    };
    let mesh = sample_grid_mesh(4, 0.05);
    let mut sampler = LiveMeshSurfaceSampler::new(config);
    let first = sampler.update_from_mesh(&mesh);

    let mut deformed = mesh.clone();
    for vertex in &mut deformed.vertices {
        vertex.z += (vertex.x * 18.0).sin() * 0.006;
    }
    let second = sampler.update_from_mesh(&deformed);
    let payload = sampler.samples().render_payload(
        2,
        RenderCoordinateSpace::World,
        0.006,
        ColorRgba::new(0.15, 0.8, 1.0, 1.0),
    );

    let mut shifted = deformed.clone();
    for vertex in &mut shifted.vertices {
        vertex.x += 0.16;
    }
    let mut other_sampler = LiveMeshSurfaceSampler::new(MeshSurfaceSampleConfig {
        seed: 2027,
        ..config
    });
    other_sampler.update_from_mesh(&shifted);
    let cross = sampler.samples().cross_neighborhood_with(
        other_sampler.samples(),
        MeshSurfaceCrossNeighborConfig {
            neighbors_per_point: 2,
            max_distance_meters: 0.18,
        },
    );

    println!(
        "dynamic mesh coordinates: vertices={} triangles={} samples={} first_neighbors={} second_neighbors={} cross_neighbors={} first_update={:?} second_update={:?} payload_points={}",
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
        first.status,
        second.status,
        payload.points.len()
    );
}

fn sample_grid_mesh(cells: usize, spacing: f32) -> TriangleMeshSurface {
    let cells = cells.max(1);
    let side = cells + 1;
    let half_width = cells as f32 * spacing * 0.5;
    let mut vertices = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            vertices.push(Vec3::new(
                x as f32 * spacing - half_width,
                y as f32 * spacing - half_width,
                0.0,
            ));
        }
    }

    let mut triangles = Vec::with_capacity(cells * cells * 2);
    for y in 0..cells {
        for x in 0..cells {
            let a = y * side + x;
            let b = a + 1;
            let c = a + side;
            let d = c + 1;
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }

    TriangleMeshSurface::new(vertices, triangles)
}
