mod cpu;
mod gpu;

pub use cpu::detect_cpu;
pub use gpu::detect_gpu;

/// A snapshot of the detected hardware resources.
#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub cpu_cores: u32,
    pub total_ram_bytes: u64,
    #[allow(dead_code)]
    pub available_ram_bytes: u64,
    pub gpu_name: Option<String>,
    pub gpu_vram_bytes: Option<u64>,
}

/// Run all hardware detection and return a combined profile.
pub fn detect_hardware() -> HardwareProfile {
    let cpu = detect_cpu();
    let gpu = detect_gpu();

    HardwareProfile {
        cpu_cores: cpu.cores,
        total_ram_bytes: cpu.total_ram,
        available_ram_bytes: cpu.available_ram,
        gpu_name: gpu.name,
        gpu_vram_bytes: gpu.vram_bytes,
    }
}
