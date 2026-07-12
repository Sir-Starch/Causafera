use ontopolis_core::deterministic::DeterministicConfig;

#[test]
fn deterministic_config_serde() {
    let config = DeterministicConfig::new(42);
    let json = serde_json::to_string(&config).unwrap();
    let restored: DeterministicConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.world_seed, restored.world_seed);
    assert_eq!(config.strict_mode, restored.strict_mode);
}
