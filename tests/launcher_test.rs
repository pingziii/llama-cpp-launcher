use llm_launcher::launcher::build_server_args;
use llm_launcher::params::LaunchParams;

fn make_params() -> LaunchParams {
    LaunchParams {
        thread_count: 7,
        n_gpu_layers: 99,
        ctx_size: 4096,
        batch_size: 1024,
        ubatch_size: 512,
        flash_attn: "auto".to_string(),
        cache_type_k: "f16".to_string(),
        cache_type_v: "f16".to_string(),
        parallel_slots: 1,
        mlock: false,
        no_kv_offload: false,
        jinja: false,
        host: "127.0.0.1".to_string(),
        port: 8080,
        model_path: "/models/test.gguf".to_string(),
        model_name: "test".to_string(),
    }
}

#[test]
fn test_build_server_args_contains_expected_flags() {
    let params = make_params();
    let args = build_server_args(&params);
    assert!(args.contains(&"--host".to_string()));
    assert!(args.contains(&"--port".to_string()));
    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"--threads".to_string()));
    assert!(args.contains(&"--n-gpu-layers".to_string()));
    assert!(args.contains(&"--ctx-size".to_string()));
    assert!(args.contains(&"--batch-size".to_string()));
}

#[test]
fn test_build_server_args_contains_correct_values() {
    let params = make_params();
    let args = build_server_args(&params);

    let assert_flag_value = |flag: &str, expected: &str| {
        let pos = args.iter().position(|a| a == flag);
        assert!(pos.is_some(), "Flag {flag} not found");
        let pos = pos.unwrap();
        assert!(pos + 1 < args.len(), "No value after flag {flag}");
        assert_eq!(args[pos + 1], expected, "Flag {flag} value mismatch");
    };

    assert_flag_value("--host", "127.0.0.1");
    assert_flag_value("--port", "8080");
    assert_flag_value("--model", "/models/test.gguf");
    assert_flag_value("--threads", "7");
    assert_flag_value("--n-gpu-layers", "99");
    assert_flag_value("--ctx-size", "4096");
    assert_flag_value("--batch-size", "1024");
}

#[test]
fn test_build_server_args_even_length() {
    let params = make_params();
    let args = build_server_args(&params);
    assert!(args.len() % 2 == 0, "Args should be flag-value pairs");
}
