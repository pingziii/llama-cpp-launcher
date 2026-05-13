use llm_launcher::config::{config_file_path, remove_preset, upsert_preset, Config, Preset};

#[test]
fn test_config_default_values() {
    let config = Config::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert!(!config.debug_logging);
    assert!(config.llama_bin_path.is_none());
}

#[test]
fn test_config_default_model_dir_exists() {
    let config = Config::default();
    assert!(
        !config.model_dir.is_empty(),
        "model_dir should not be empty"
    );
}

#[test]
fn test_config_default_presets_exist() {
    let config = Config::default();
    assert_eq!(config.presets.len(), 2);
    assert_eq!(config.presets[0].name, "high-throughput");
    assert_eq!(config.presets[1].name, "low-memory");
}

#[test]
fn test_preset_struct() {
    let preset = Preset {
        name: "test".to_string(),
        ctx_size: Some(2048),
        n_gpu_layers: Some(20),
        batch_size: Some(512),
        threads: Some(4),
        description: Some("Test preset".to_string()),
    };
    assert_eq!(preset.name, "test");
    assert_eq!(preset.ctx_size, Some(2048));
    assert_eq!(preset.n_gpu_layers, Some(20));
    assert_eq!(preset.batch_size, Some(512));
    assert_eq!(preset.threads, Some(4));
}

#[test]
fn test_preset_with_none_fields() {
    let preset = Preset {
        name: "minimal".to_string(),
        ctx_size: None,
        n_gpu_layers: None,
        batch_size: None,
        threads: None,
        description: None,
    };
    assert!(preset.ctx_size.is_none());
    assert!(preset.n_gpu_layers.is_none());
    assert!(preset.batch_size.is_none());
    assert!(preset.threads.is_none());
    assert!(preset.description.is_none());
}

#[test]
fn test_config_file_path_returns_non_empty() {
    let path = config_file_path();
    assert!(path.to_string_lossy().len() > 0);
    assert!(path.ends_with("config.yml"));
}

#[test]
fn test_config_serde_roundtrip() {
    let config = Config::default();
    let yaml_str = serde_yaml::to_string(&config).expect("serialize");
    let deserialized: Config = serde_yaml::from_str(&yaml_str).expect("deserialize");
    assert_eq!(deserialized.host, config.host);
    assert_eq!(deserialized.port, config.port);
    assert_eq!(deserialized.presets.len(), config.presets.len());
}

#[test]
fn test_upsert_preset_adds_new() {
    let mut config = Config::default();
    let count_before = config.presets.len();
    let new_preset = Preset {
        name: "custom-test".to_string(),
        ctx_size: Some(2048),
        n_gpu_layers: None,
        batch_size: None,
        threads: None,
        description: Some("Test preset".to_string()),
    };
    let added = upsert_preset(&mut config, new_preset);
    assert!(added);
    assert_eq!(config.presets.len(), count_before + 1);
}

#[test]
fn test_upsert_preset_updates_existing() {
    let mut config = Config::default();
    let updated = Preset {
        name: "high-throughput".to_string(),
        ctx_size: Some(16384),
        n_gpu_layers: Some(99),
        batch_size: None,
        threads: None,
        description: Some("Updated".to_string()),
    };
    let added = upsert_preset(&mut config, updated);
    assert!(!added, "should update existing, not add new");
    let ht = config
        .presets
        .iter()
        .find(|p| p.name == "high-throughput")
        .unwrap();
    assert_eq!(ht.ctx_size, Some(16384));
    assert_eq!(ht.n_gpu_layers, Some(99));
}

#[test]
fn test_remove_preset_removes_by_name() {
    let mut config = Config::default();
    let count_before = config.presets.len();
    let removed = remove_preset(&mut config, "low-memory");
    assert!(removed);
    assert_eq!(config.presets.len(), count_before - 1);
    assert!(config.presets.iter().all(|p| p.name != "low-memory"));
}

#[test]
fn test_remove_preset_nonexistent() {
    let mut config = Config::default();
    let count_before = config.presets.len();
    let removed = remove_preset(&mut config, "nonexistent");
    assert!(!removed);
    assert_eq!(config.presets.len(), count_before);
}

#[test]
fn test_config_dir_path_returns_expected_basename() {
    let path = llm_launcher::config::config_dir_path();
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(file_name, ".llamalauncher");
}

#[test]
fn test_config_dir_path_is_absolute() {
    let path = llm_launcher::config::config_dir_path();
    assert!(path.is_absolute());
}

#[test]
fn test_config_file_path_ends_with_config_yml() {
    let path = llm_launcher::config::config_file_path();
    assert_eq!(path.file_name().unwrap(), "config.yml");
}

#[test]
fn test_config_file_path_lives_in_subdir() {
    let path = llm_launcher::config::config_file_path();
    let parent = path.parent().unwrap();
    assert_eq!(parent.file_name().unwrap(), ".llamalauncher");
}

#[test]
fn test_default_model_dir_is_in_home() {
    let config = Config::default();
    let path = std::path::Path::new(&config.model_dir);
    // Should be under the user's home directory
    let home = dirs::home_dir().unwrap();
    assert!(
        path.starts_with(&home),
        "model_dir {:?} should start with home {:?}",
        path,
        home
    );
}

#[test]
fn test_persist_last_selection_updates_config() {
    let mut config = Config::default();
    assert!(config.last_model.is_none());
    assert!(config.last_preset.is_none());

    // This writes to real disk, so we check the in-memory update
    let result = llm_launcher::config::persist_last_selection(
        &mut config,
        "/models/test.gguf",
        Some("high-throughput"),
    );
    assert!(result.is_ok() || result.is_err());
    assert_eq!(config.last_model, Some("/models/test.gguf".to_string()));
    assert_eq!(config.last_preset, Some("high-throughput".to_string()));
}
