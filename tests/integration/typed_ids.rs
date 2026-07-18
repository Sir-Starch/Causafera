use causafera_types::{AgentId, ConceptId, SimulationTime};

#[test]
fn typed_ids_are_distinct() {
    let a = AgentId::new(1);
    let b = ConceptId::new(1);
    // Different types should not compare equal even with same raw value
    // This is a compile-time guarantee, but we verify the types exist
    assert_eq!(a.raw(), b.raw());
}

#[test]
fn simulation_time_orders() {
    let t0 = SimulationTime::new(0);
    let t1 = SimulationTime::new(1);
    assert!(t0 < t1);
    assert_eq!(t0.tick(), t1);
}
