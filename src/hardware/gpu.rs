pub struct GpuInfo {
    pub name: Option<String>,
    pub vram_bytes: Option<u64>,
}

/// Detect GPU hardware. Tries NVIDIA NVML first, then Vulkan/AMD/Intel fallback.
pub fn detect_gpu() -> GpuInfo {
    match detect_nvidia_gpu() {
        Ok(info) => return info,
        Err(e) => {
            tracing::debug!("NVIDIA GPU detection failed: {e}");
        }
    }

    // Fallback: try Apple Silicon detection
    if let Some(info) = detect_apple_gpu() {
        return info;
    }

    // Fallback: try Vulkan-capable GPU on Windows
    detect_vulkan_gpu().unwrap_or(GpuInfo {
        name: None,
        vram_bytes: None,
    })
}

fn detect_nvidia_gpu() -> Result<GpuInfo, String> {
    let nvml = nvml_wrapper::Nvml::init().map_err(|e| format!("NVML init failed: {e:?}"))?;

    let device = nvml
        .device_by_index(0)
        .map_err(|e| format!("NVML device 0 failed: {e:?}"))?;

    let name = device.name().ok();
    let vram = device.memory_info().ok().map(|m| m.total);

    Ok(GpuInfo {
        name,
        vram_bytes: vram,
    })
}

/// Minimal Apple Silicon detection via sysctl.
fn detect_apple_gpu() -> Option<GpuInfo> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let output = Command::new("sysctl")
            .args(["-n", "hw.model"])
            .output()
            .ok()?;

        let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if model.to_lowercase().contains("apple") || model.starts_with("Mac") {
            return Some(GpuInfo {
                name: Some("Apple Silicon".to_string()),
                vram_bytes: None,
            });
        }
    }

    None
}

/// Detect any Vulkan-capable GPU on Windows by probing the Vulkan loader.
/// Works for AMD, Intel, and NVIDIA GPUs with Vulkan drivers installed.
fn detect_vulkan_gpu() -> Option<GpuInfo> {
    #[cfg(target_os = "windows")]
    {
        for path in &[
            "C:\\Windows\\System32\\vulkan-1.dll",
            "C:\\Windows\\SysWOW64\\vulkan-1.dll",
        ] {
            if std::path::Path::new(path).exists() {
                tracing::debug!("Vulkan loader found at {path}, assuming GPU available");
                return Some(GpuInfo {
                    name: Some("Vulkan GPU".to_string()),
                    vram_bytes: None,
                });
            }
        }
    }

    None
}
