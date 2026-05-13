mod process;
pub mod server;

pub use process::LLamaProcess;
pub use server::LLamaClient;

use crate::error::{AppError, Result};
use crate::params::LaunchParams;

/// Attempt to locate the llama.cpp binary. Checks the configured path first,
/// then searches the system PATH.
pub fn find_llama_binary(configured_path: Option<&str>) -> Result<std::process::Command> {
    const CANDIDATES: &[&str] = &["llama-server", "llama.cpp", "llama-cli"];

    if let Some(path) = configured_path {
        let cmd_path = std::path::Path::new(path);
        // If it's directly a file, use it
        if cmd_path.is_file() {
            return Ok(std::process::Command::new(cmd_path));
        }
        // If it's a directory, look for candidate binaries inside it
        if cmd_path.is_dir() {
            for candidate in CANDIDATES {
                for exe_name in [candidate.to_string(), format!("{}.exe", candidate)] {
                    let full_path = cmd_path.join(&exe_name);
                    if full_path.is_file() {
                        return Ok(std::process::Command::new(full_path));
                    }
                }
            }
            return Err(AppError::BinaryNotFound {
                path: format!("{path} (no llama-server found in directory)"),
            });
        }
        // Path exists but is neither file nor directory — error
        return Err(AppError::BinaryNotFound {
            path: path.to_string(),
        });
    }

    for candidate in CANDIDATES {
        if let Ok(cmd) = which::which(candidate) {
            return Ok(std::process::Command::new(cmd));
        }
    }

    Err(AppError::BinaryNotFound {
        path: "None".to_string(),
    })
}

/// Build the argument list for llama.cpp server mode.
pub fn build_server_args(params: &LaunchParams) -> Vec<String> {
    let mut args = vec![
        "--host".to_string(),
        params.host.clone(),
        "--port".to_string(),
        params.port.to_string(),
        "--model".to_string(),
        params.model_path.clone(),
        "--threads".to_string(),
        params.thread_count.to_string(),
        "--n-gpu-layers".to_string(),
        params.n_gpu_layers.to_string(),
        "--ctx-size".to_string(),
        params.ctx_size.to_string(),
        "--batch-size".to_string(),
        params.batch_size.to_string(),
    ];

    // Only emit non-default values to keep command line clean
    if params.ubatch_size != 512 {
        args.push("--ubatch-size".to_string());
        args.push(params.ubatch_size.to_string());
    }
    if params.flash_attn != "auto" {
        args.push("--flash-attn".to_string());
        args.push(params.flash_attn.clone());
    }
    if params.cache_type_k != "f16" {
        args.push("--cache-type-k".to_string());
        args.push(params.cache_type_k.clone());
    }
    if params.cache_type_v != "f16" {
        args.push("--cache-type-v".to_string());
        args.push(params.cache_type_v.clone());
    }
    if params.parallel_slots > 1 {
        args.push("--parallel".to_string());
        args.push(params.parallel_slots.to_string());
    }
    if params.mlock {
        args.push("--mlock".to_string());
    }
    if params.no_kv_offload {
        args.push("--no-kv-offload".to_string());
    }
    if params.jinja {
        args.push("--jinja".to_string());
    }

    args
}
