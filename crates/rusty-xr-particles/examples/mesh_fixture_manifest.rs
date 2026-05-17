use rusty_xr_particles::mesh_fixture_manifest_examples;

#[cfg(feature = "serde")]
fn main() {
    let manifests = mesh_fixture_manifest_examples();
    println!(
        "{}",
        serde_json::to_string_pretty(&manifests).expect("fixture manifests should serialize")
    );
}

#[cfg(not(feature = "serde"))]
fn main() {
    for manifest in mesh_fixture_manifest_examples() {
        println!(
            "mesh fixture manifest: id={} kind={:?} vertices={} indices={} samples={} topology_hash={:016x}",
            manifest.fixture_id,
            manifest.fixture_kind,
            manifest.vertex_count,
            manifest.index_count,
            manifest.coordinate_sample_count,
            manifest.topology_hash
        );
    }
}
