use rusty_xr_particles::{
    build_fixture_hand_mesh, ColorRgba, DynamicMeshCollider, DynamicMeshColliderConfig,
    DynamicMeshColliderDiagnosticConfig, FixtureHandMeshConfig, HandMeshSnapshot, Handedness,
    TriangleMeshSurface, Vec3,
};

fn main() {
    let mut mesh = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
    translate_surface(&mut mesh, Vec3::new(0.0, 1.35, -0.70));

    let snapshot = hand_snapshot_from_mesh(1, &mesh, Handedness::Left);
    let mut collider = DynamicMeshCollider::new(DynamicMeshColliderConfig {
        surface_inflation_meters: 0.004,
        contact_padding_meters: 0.002,
        prefer_convex: true,
        diagnostic: DynamicMeshColliderDiagnosticConfig {
            enabled: true,
            shell_inflation_meters: 0.002,
            color: ColorRgba::new(0.1, 0.9, 1.0, 0.42),
        },
        ..DynamicMeshColliderConfig::default()
    });

    let first = collider.update_from_hand_mesh_snapshot(&snapshot);
    let query_point = mesh.vertices[0] + Vec3::new(0.0, 0.0, 0.025);
    let closest = collider
        .closest_point(query_point)
        .expect("fixture hand collider should answer closest-point queries");
    let overlaps_probe = collider.overlaps_sphere(query_point, 0.03);

    let mut deformed_mesh = mesh.clone();
    for vertex in &mut deformed_mesh.vertices {
        vertex.y += 0.015;
        vertex.z += (vertex.x * 12.0).sin() * 0.004;
    }
    let deformed_snapshot = hand_snapshot_from_mesh(2, &deformed_mesh, Handedness::Left);
    let second = collider.update_from_hand_mesh_snapshot(&deformed_snapshot);
    let shell = collider.diagnostic_shell();

    println!(
        "hand mesh dynamic collider: first={:?} second={:?} vertices={} triangles={} convex_eligible={} shell_vertices={} shell_triangles={} overlaps_probe={} closest_distance={:.4} closest_triangle={}",
        first.status,
        second.status,
        second.vertex_count,
        second.triangle_count,
        second.convex_eligible,
        shell.map_or(0, |shell| shell.surface.vertex_count()),
        shell.map_or(0, |shell| shell.surface.triangle_count()),
        overlaps_probe,
        closest.distance_meters,
        closest.triangle_index
    );
}

fn translate_surface(surface: &mut TriangleMeshSurface, offset: Vec3) {
    for vertex in &mut surface.vertices {
        *vertex += offset;
    }
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
