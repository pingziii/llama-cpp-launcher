mod model_metadata;
mod optimizer;

pub use model_metadata::{read_gguf_metadata, ModelMetadata};
pub use optimizer::{
    calculate_optimal_params, calculate_optimal_with_preset, calculate_params,
    calculate_params_with_preset, estimate_model_memory, LaunchParams,
};
