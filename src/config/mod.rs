mod loader;

#[allow(unused_imports)]
pub use loader::{
    config_dir_path, config_file_path, load_or_create_config, persist_last_selection,
    remove_preset, save_config, upsert_preset, Config, Preset,
};
