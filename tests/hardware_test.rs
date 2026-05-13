use llm_launcher::hardware::detect_hardware;

#[test]
fn test_detect_hardware_returns_nonzero_cpu_cores() {
    let hw = detect_hardware();
    assert!(hw.cpu_cores > 0, "CPU cores should be at least 1");
}

#[test]
fn test_detect_hardware_returns_positive_ram() {
    let hw = detect_hardware();
    assert!(hw.total_ram_bytes > 0, "Total RAM should be positive");
}

#[test]
fn test_detect_hardware_available_ram_not_exceeding_total() {
    let hw = detect_hardware();
    assert!(
        hw.available_ram_bytes <= hw.total_ram_bytes,
        "Available RAM should not exceed total RAM"
    );
}

#[test]
fn test_detect_hardware_struct_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<llm_launcher::hardware::HardwareProfile>();
}

#[test]
fn test_detect_hardware_struct_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<llm_launcher::hardware::HardwareProfile>();
}
