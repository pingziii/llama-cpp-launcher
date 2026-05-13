use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to read configuration file at {path}: {details}")]
    ConfigRead { path: PathBuf, details: String },

    #[error("Failed to write configuration file at {path}: {details}")]
    ConfigWrite { path: PathBuf, details: String },

    #[error("Hardware detection failed: {0}")]
    Hardware(String),

    #[error("Model directory '{path}' not found or not accessible: {details}")]
    ModelDirNotFound { path: PathBuf, details: String },

    #[error("No GGUF model files found in '{path}'")]
    NoModelsFound { path: PathBuf },

    #[error("llama.cpp binary not found: searched PATH and configured path {path}")]
    BinaryNotFound { path: String },

    #[error(
        "Insufficient memory: estimated {estimated_mb:.0} MB needed, {available_mb:.0} MB available"
    )]
    InsufficientMemory {
        estimated_mb: f64,
        available_mb: f64,
    },

    #[error("Failed to start llama.cpp process: {0}")]
    ProcessStart(String),

    #[error("llama.cpp process exited unexpectedly: {0}")]
    ProcessCrashed(String),

    #[error("Server health check failed (HTTP {status}): {details}")]
    ServerHealth { status: u16, details: String },

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Port {0} is already in use")]
    PortInUse(u16),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML config error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Returns a user-facing suggestion for how to fix the error.
    #[allow(dead_code)]
    pub fn suggestion(&self) -> &str {
        match self {
            AppError::Config(_) => "Check your configuration file for errors.",
            AppError::ConfigRead { .. } => "Ensure the config file exists and is readable.",
            AppError::ConfigWrite { .. } => "Ensure the config directory is writable.",
            AppError::Hardware(_) => "The tool can still run in CPU-only mode.",
            AppError::ModelDirNotFound { .. } => {
                "Set 'model_dir' in config to a directory containing .gguf files."
            }
            AppError::NoModelsFound { .. } => "Place .gguf model files in your model directory.",
            AppError::BinaryNotFound { .. } => {
                "Install llama.cpp or set 'llama_bin_path' in config."
            }
            AppError::InsufficientMemory { .. } => {
                "Try a smaller model, reduce context size, or use a 'low-memory' preset."
            }
            AppError::ProcessStart(_) => "Check that the llama.cpp binary is compatible.",
            AppError::ProcessCrashed(_) => "Check the logs. The model may be incompatible.",
            AppError::ServerHealth { .. } => {
                "Server is not ready yet. Check llama.cpp output or press Enter to retry."
            }
            AppError::Http(_) => "A network error occurred with the local server.",
            AppError::PortInUse(_) => "Change 'port' in config or stop the existing process.",
            AppError::Io(_) => "A filesystem operation failed. Check permissions.",
            AppError::YamlError(_) => {
                "Fix syntax errors in the config file, or check that the config can be serialized."
            }
            AppError::Cancelled => "Operation cancelled by user.",
            AppError::Other(_) => "An unexpected error occurred.",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
