use sysinfo::System;

pub struct CpuInfo {
    pub cores: u32,
    pub total_ram: u64,
    pub available_ram: u64,
}

/// Detect CPU core count and system RAM using sysinfo.
pub fn detect_cpu() -> CpuInfo {
    let mut sys = System::new_all();
    sys.refresh_memory();
    sys.refresh_cpu_usage();

    let cores = sys
        .physical_core_count()
        .or_else(|| {
            // Fallback to logical core count if physical not available
            Some(std::thread::available_parallelism().ok()?.get() as u64 as usize)
        })
        .unwrap_or(4) as u32;

    let total_ram = sys.total_memory();
    let available_ram = sys.available_memory();

    CpuInfo {
        cores,
        total_ram,
        available_ram,
    }
}
