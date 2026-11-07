use orchestra_config::*;
use std::path::Path;

#[test]
fn test_all_example_configs_are_valid() {
    let examples = [
        "configs/examples/dev.yaml",
        "configs/examples/standard.yaml",
        "configs/examples/ha.yaml",
        "configs/examples/orchestra.toml",
    ];

    for rel_path in &examples {
        let full_path = Path::new("../../").join(rel_path);
        // Also check direct path from crate or root
        let path = if full_path.exists() {
            full_path
        } else {
            Path::new(rel_path).to_path_buf()
        };

        if path.exists() {
            let config = load_from_file(&path)
                .unwrap_or_else(|e| panic!("Failed to load {}: {:?}", path.display(), e));
            validate_config(&config)
                .unwrap_or_else(|e| panic!("Failed to validate {}: {:?}", path.display(), e));
        }
    }
}

#[test]
fn test_profiles_and_serialization() {
    for profile in [Profile::Dev, Profile::Standard, Profile::Ha] {
        let cfg = create_profile_config("test-cluster", profile);
        assert!(validate_config(&cfg).is_ok());

        // YAML roundtrip
        let yaml = to_yaml(&cfg).unwrap();
        let from_yaml = parse_yaml(&yaml).unwrap();
        assert_eq!(cfg, from_yaml);

        // TOML roundtrip
        let toml = to_toml(&cfg).unwrap();
        let from_toml = parse_toml(&toml).unwrap();
        assert_eq!(cfg, from_toml);

        // JSON roundtrip
        let json = to_json(&cfg).unwrap();
        let from_json: OrchestraConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, from_json);
    }
}

#[test]
fn test_validation_rejects_even_control_planes_for_ha() {
    let mut cfg = create_profile_config("ha-test", Profile::Dev);
    cfg.spec.nodes.control_plane.count = 2; // Even number!
    let err = validate_config(&cfg).unwrap_err();
    assert!(err.to_string().contains("odd number"));
}

#[test]
fn test_validation_rejects_zero_control_planes() {
    let mut cfg = create_profile_config("zero-cp", Profile::Dev);
    cfg.spec.nodes.control_plane.count = 0;
    let err = validate_config(&cfg).unwrap_err();
    assert!(err.to_string().contains("at least 1"));
}

#[test]
fn test_validation_rejects_port_collision() {
    let mut cfg = create_profile_config("port-collision", Profile::Dev);
    cfg.spec
        .nodes
        .control_plane
        .port_mappings
        .push(PortMapping {
            container_port: 80,
            host_port: 6443, // Collides with api_server_port!
            protocol: "tcp".into(),
        });

    let err = validate_config(&cfg).unwrap_err();
    assert!(err.to_string().contains("Host port collision"));
}

#[test]
fn test_layered_config_loading_hierarchy() {
    let overrides = ConfigOverrides {
        name: Some("cli-cluster".into()),
        worker_count: Some(10),
        ebpf_enabled: Some(true),
        ..Default::default()
    };

    let cfg = load_config(None, Some(Profile::Standard), Some(overrides)).unwrap();
    assert_eq!(cfg.metadata.name, "cli-cluster");
    assert_eq!(cfg.spec.nodes.workers.count, 10);
    assert_eq!(
        cfg.spec.network.cni,
        orchestra_core::network::CniPlugin::Cilium
    );
}
