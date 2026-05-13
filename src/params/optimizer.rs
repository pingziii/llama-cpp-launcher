use crate::config::Preset;
use crate::hardware::HardwareProfile;

use super::ModelMetadata;

/// Auto-calculated optimal launch parameters for llama.cpp.
#[derive(Debug, Clone)]
pub struct LaunchParams {
    pub thread_count: u32,
    pub n_gpu_layers: u32,
    pub ctx_size: u32,
    pub batch_size: u32,
    pub ubatch_size: u32,
    pub flash_attn: String,
    pub cache_type_k: String,
    pub cache_type_v: String,
    pub parallel_slots: u32,
    pub mlock: bool,
    pub no_kv_offload: bool,
    pub jinja: bool,
    pub host: String,
    pub port: u16,
    pub model_path: String,
    pub model_name: String,
}

/// Derive a display-friendly model name from a file path.
fn model_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_lowercase()
}

/// Calculate optimal launch params using model metadata + hardware profile.
///
/// Uses the model's actual context_length, architecture, and parameter count
/// to determine the best settings, capped by available system resources.
pub fn calculate_optimal_params(
    hardware: &HardwareProfile,
    metadata: &ModelMetadata,
    model_path: &str,
    host: &str,
    port: u16,
) -> LaunchParams {
    // Reserve one core for the TUI and OS
    let thread_count = (hardware.cpu_cores.saturating_sub(1)).clamp(1, 64);

    // If any GPU detected (even without VRAM info), offload everything to GPU
    // Users with AMD/Intel GPUs won't have nvml data but may still have Vulkan
    let n_gpu_layers = if hardware.gpu_vram_bytes.is_some() || hardware.gpu_name.is_some() {
        99
    } else {
        0
    };

    // Use model's native context length, capped by available RAM
    let model_ctx = metadata.context_length;
    let ctx_size = if model_ctx > 0 {
        if hardware.total_ram_bytes > 64_000_000_000 {
            model_ctx.min(131_072) // 128k max for >64 GB RAM
        } else if hardware.total_ram_bytes > 32_000_000_000 {
            model_ctx.min(32_768) // 32k for >32 GB
        } else if hardware.total_ram_bytes > 16_000_000_000 {
            model_ctx.min(16_384) // 16k for >16 GB
        } else {
            model_ctx.min(8_192) // 8k for <=16 GB
        }
    } else {
        // Fallback: no metadata available — use RAM-based heuristic
        if hardware.total_ram_bytes > 64_000_000_000 {
            8_192
        } else if hardware.total_ram_bytes > 32_000_000_000 {
            4_096
        } else {
            2_048
        }
    };

    // Batch size scales with context
    let batch_size = (ctx_size / 4).clamp(64, 2048);

    LaunchParams {
        thread_count,
        n_gpu_layers,
        ctx_size,
        batch_size,
        ubatch_size: 512,
        flash_attn: "auto".to_string(),
        cache_type_k: "f16".to_string(),
        cache_type_v: "f16".to_string(),
        parallel_slots: 1,
        mlock: false,
        no_kv_offload: false,
        jinja: false,
        host: host.to_string(),
        port,
        model_path: model_path.to_string(),
        model_name: model_name_from_path(model_path),
    }
}

/// Calculate optimal default params from a hardware profile and model path.
pub fn calculate_params(
    hardware: &HardwareProfile,
    model_path: &str,
    host: &str,
    port: u16,
) -> LaunchParams {
    // Reserve one core for the TUI and OS
    let thread_count = (hardware.cpu_cores.saturating_sub(1)).clamp(1, 64);

    // Determine GPU layers based on available VRAM
    let n_gpu_layers = match hardware.gpu_vram_bytes {
        Some(vram) if vram > 4_000_000_000 => 99, // >4GB: offload all layers
        Some(vram) if vram > 2_000_000_000 => 35, // >2GB: offload most layers
        Some(vram) if vram > 1_000_000_000 => 20, // >1GB: offload some
        Some(_) => 10,                            // <1GB: minimal offload
        None => 0,                                // No GPU: CPU only
    };

    // Default context: 4096 fits most hardware. Could be larger with >32GB RAM.
    let ctx_size = if hardware.total_ram_bytes > 64_000_000_000 {
        8192
    } else if hardware.total_ram_bytes > 32_000_000_000 {
        4096
    } else {
        2048
    };

    // Batch size scales with context
    let batch_size = (ctx_size / 4).clamp(64, 2048);

    LaunchParams {
        thread_count,
        n_gpu_layers,
        ctx_size,
        batch_size,
        ubatch_size: 512,
        flash_attn: "auto".to_string(),
        cache_type_k: "f16".to_string(),
        cache_type_v: "f16".to_string(),
        parallel_slots: 1,
        mlock: false,
        no_kv_offload: false,
        jinja: false,
        host: host.to_string(),
        port,
        model_path: model_path.to_string(),
        model_name: model_name_from_path(model_path),
    }
}

/// Apply a preset on top of optimal metadata-aware params.
pub fn calculate_optimal_with_preset(
    hardware: &HardwareProfile,
    metadata: &ModelMetadata,
    model_path: &str,
    host: &str,
    port: u16,
    preset: &Preset,
) -> LaunchParams {
    let mut params = calculate_optimal_params(hardware, metadata, model_path, host, port);

    if let Some(ctx) = preset.ctx_size {
        params.ctx_size = ctx;
    }
    if let Some(layers) = preset.n_gpu_layers {
        params.n_gpu_layers = layers;
    }
    if let Some(batch) = preset.batch_size {
        params.batch_size = batch;
    }
    if let Some(threads) = preset.threads {
        params.thread_count = threads;
    }

    params
}

/// Apply a preset on top of auto-calculated params.
#[allow(dead_code)]
pub fn calculate_params_with_preset(
    hardware: &HardwareProfile,
    model_path: &str,
    host: &str,
    port: u16,
    preset: &Preset,
) -> LaunchParams {
    let mut params = calculate_params(hardware, model_path, host, port);

    if let Some(ctx) = preset.ctx_size {
        params.ctx_size = ctx;
    }
    if let Some(layers) = preset.n_gpu_layers {
        params.n_gpu_layers = layers;
    }
    if let Some(batch) = preset.batch_size {
        params.batch_size = batch;
    }
    if let Some(threads) = preset.threads {
        params.thread_count = threads;
    }

    params
}

/// Estimate model memory usage using both file-size heuristic and parameter count.
/// Returns the more conservative (larger) estimate in bytes.
pub fn estimate_model_memory(file_size_bytes: u64, parameter_count: Option<u64>) -> u64 {
    // Heuristic 1: file size * overhead multiplier
    let file_based = (file_size_bytes as f64 * 1.3) as u64;

    // Heuristic 2: parameter-count-based (only if we can extract it)
    let param_based = parameter_count.map(|params| {
        // Estimate bytes per param from quantization level
        // Q2 ~2.5, Q3 ~3.5, Q4 ~4.5, Q5 ~5.5, Q6 ~6.5, Q8 ~8.5
        let bytes_per_param = 4.5; // conservative estimate for Q4 quant
                                   // Context overhead: ~2 bytes per token per layer
        let ctx_overhead = 4096u64 * 2 * 80; // rough upper bound
        (params as f64 * bytes_per_param) as u64 + ctx_overhead
    });

    // Take the more conservative (larger) estimate
    match param_based {
        Some(param_est) => file_based.max(param_est),
        None => file_based,
    }
}
