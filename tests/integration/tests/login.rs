//! Login flow integration test against real Chrome.
//!
//! Requires: docker compose -f tests/integration/docker-compose.yml up -d

use pwright_integration_tests::connect_and_navigate;

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn login_flow_end_to_end() {
    let page = connect_and_navigate("/login").await;

    // Submit button should be disabled initially
    assert!(page.locator("#submit").is_disabled().await.unwrap());

    // Fill email
    page.locator("#email")
        .fill("user@example.com")
        .await
        .unwrap();

    // Submit still disabled (password empty)
    assert!(page.locator("#submit").is_disabled().await.unwrap());

    // Fill password
    page.locator("#password").fill("secret123").await.unwrap();

    // Now submit should be enabled
    // (give the JS event handler time to fire)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(page.locator("#submit").is_enabled().await.unwrap());

    // Submit the form
    page.locator("#submit").click().await.unwrap();

    // Wait for result to appear
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify result message
    let result = page.locator("#result").text_content().await.unwrap();
    assert_eq!(result, Some("Welcome, user@example.com".to_string()));
}
