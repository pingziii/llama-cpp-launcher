use llm_launcher::launcher::LLamaClient;

#[tokio::test]
async fn test_health_check_returns_false_on_bad_port() {
    let client = LLamaClient::new("127.0.0.1", 1);
    let result = client.health_check().await;
    assert!(result.is_ok(), "health_check should not panic on bad port");
    assert!(
        !result.unwrap(),
        "health_check should return false on bad port"
    );
}

#[tokio::test]
async fn test_health_check_does_not_panic_on_refused() {
    let client = LLamaClient::new("127.0.0.1", 65535);
    let result = client.health_check().await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_client_creation_with_defaults() {
    let client = LLamaClient::new("127.0.0.1", 8080);
    let _ = client.health_check().await;
}

#[tokio::test]
async fn test_health_check_timeout_behavior() {
    let client = LLamaClient::new("10.255.255.1", 8080);
    let result = client.health_check().await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_llama_client_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<LLamaClient>();
}
