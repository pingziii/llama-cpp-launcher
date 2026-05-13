use llm_launcher::hardware::HardwareProfile;
use llm_launcher::params::{calculate_params, estimate_model_memory};

fn make_hardware(
    cpu_cores: u32,
    total_ram: u64,
    available_ram: u64,
    gpu_vram: Option<u64>,
) -> HardwareProfile {
    HardwareProfile {
        cpu_cores,
        total_ram_bytes: total_ram,
        available_ram_bytes: available_ram,
        gpu_name: gpu_vram.map(|_| "Test GPU".to_string()),
        gpu_vram_bytes: gpu_vram,
    }
}

#[test]
fn test_calculate_params_thread_count() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.thread_count, 7);
}

#[test]
fn test_calculate_params_min_threads() {
    let hw = make_hardware(1, 8_000_000_000, 4_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.thread_count, 1);
}

#[test]
fn test_calculate_params_gpu_layers_no_gpu() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.n_gpu_layers, 0);
}

#[test]
fn test_calculate_params_gpu_layers_high_vram() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, Some(8_000_000_000));
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.n_gpu_layers, 99);
}

#[test]
fn test_calculate_params_gpu_layers_medium_vram() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, Some(3_000_000_000));
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.n_gpu_layers, 35);
}

#[test]
fn test_calculate_params_gpu_layers_low_vram() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, Some(500_000_000));
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.n_gpu_layers, 10);
}

#[test]
fn test_calculate_params_ctx_size_high_ram() {
    let hw = make_hardware(16, 128_000_000_000, 64_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.ctx_size, 8192);
}

#[test]
fn test_calculate_params_ctx_size_medium_ram() {
    let hw = make_hardware(8, 48_000_000_000, 24_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.ctx_size, 4096);
}

#[test]
fn test_calculate_params_ctx_size_low_ram() {
    let hw = make_hardware(4, 16_000_000_000, 8_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.ctx_size, 2048);
}

#[test]
fn test_calculate_params_batch_size_scales_with_ctx() {
    let hw = make_hardware(8, 128_000_000_000, 64_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert!(params.batch_size >= 64);
    assert!(params.batch_size <= 2048);
    assert_eq!(params.batch_size, params.ctx_size / 4);
}

#[test]
fn test_calculate_params_host_port_propagated() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "0.0.0.0", 9090);
    assert_eq!(params.host, "0.0.0.0");
    assert_eq!(params.port, 9090);
}

#[test]
fn test_estimate_model_memory_file_based() {
    let mem = estimate_model_memory(1_000_000_000, None);
    assert_eq!(mem, (1_000_000_000f64 * 1.3) as u64);
}

#[test]
fn test_estimate_model_memory_param_based() {
    let file_size = 1_000_000_000u64;
    let param_count = Some(7_000_000_000u64);
    let mem = estimate_model_memory(file_size, param_count);
    let param_est = (7_000_000_000f64 * 4.5) as u64 + 4096 * 2 * 80;
    assert!(mem >= param_est);
}

#[test]
fn test_estimate_model_memory_takes_max() {
    let mem = estimate_model_memory(100_000_000_000, Some(1_000_000_000));
    let file_based = (100_000_000_000f64 * 1.3) as u64;
    assert_eq!(mem, file_based);
}

#[test]
fn test_calculate_params_model_path_propagated() {
    let hw = make_hardware(8, 32_000_000_000, 16_000_000_000, None);
    let params = calculate_params(&hw, "/models/test.gguf", "127.0.0.1", 8080);
    assert_eq!(params.model_path, "/models/test.gguf");
}
