use causafera_core::Phase;
use causafera_runtime::{CURRENT_DIGEST_SCHEMA_VERSION, Runtime, RuntimeConfig};

#[test]
fn legacy_ids_stable() {
    // Given: the production scheduler with the bounded zero-energy thermal carrier enabled.
    let runtime = Runtime::new(RuntimeConfig::new(1_021))
        .expect("production runtime with thermal systems must bootstrap");

    // When: its authoritative recipe is exported before the first tick.
    let snapshot = runtime
        .export_snapshot()
        .expect("initial runtime state must export");

    // Then: legacy registrations remain untouched and thermal systems are appended as 9 and 10.
    assert_eq!(CURRENT_DIGEST_SCHEMA_VERSION.raw(), 6);
    assert_eq!(snapshot.recipe.system_registrations.len(), 11);
    assert_eq!(
        snapshot
            .recipe
            .system_registrations
            .iter()
            .map(|registration| registration.registration_order)
            .collect::<Vec<_>>(),
        (0_u16..=10).collect::<Vec<_>>()
    );
    assert_eq!(
        snapshot.recipe.system_registrations[9].phase,
        Phase::Physics
    );
    assert_eq!(
        snapshot.recipe.system_registrations[10].phase,
        Phase::Physics
    );
}
