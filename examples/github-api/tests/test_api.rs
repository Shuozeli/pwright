//! GitHub API Tests
//!
//! Equivalent of playwright/examples/github-api/tests/test-api.spec.ts
//!
//! pwright doesn't have a standalone HTTP request API, so these are
//! modeled as page.evaluate(fetch(...)) calls.

use pwright_bridge::playwright::Page;
use pwright_bridge::test_utils::MockCdpClient;
use std::sync::Arc;

fn mock_with_navigation() -> Arc<MockCdpClient> {
    let mock = Arc::new(MockCdpClient::new());
    mock.set_navigate_response(serde_json::json!({"frameId": "F1"}));
    mock.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));
    mock
}

/// ```typescript
/// // Playwright
/// test.beforeAll(async ({ request }) => {
///   const response = await request.post('/user/repos', {
///     data: { name: 'Test-Repo-1' }
///   });
///   expect(response.ok()).toBeTruthy();
/// });
///
/// test('should create bug report', async ({ request }) => {
///   const newIssue = await request.post('/repos/user/Test-Repo-1/issues', {
///     data: { title: '[Bug] report 1', body: 'Bug description' }
///   });
///   expect(newIssue.ok()).toBeTruthy();
///   const issues = await request.get('/repos/user/Test-Repo-1/issues');
///   expect(await issues.json()).toContainEqual(
///     expect.objectContaining({ title: '[Bug] report 1' })
///   );
/// });
/// ```
#[tokio::test]
async fn should_create_bug_report() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    // Create repo via fetch
    page.evaluate(
        r#"fetch('https://api.github.com/user/repos', {
            method: 'POST',
            headers: { 'Authorization': 'token test' },
            body: JSON.stringify({ name: 'Test-Repo-1' })
        }).then(r => r.status)"#,
    )
    .await
    .unwrap();

    // Create bug report
    page.evaluate(
        r#"fetch('https://api.github.com/repos/user/Test-Repo-1/issues', {
            method: 'POST',
            headers: { 'Authorization': 'token test' },
            body: JSON.stringify({ title: '[Bug] report 1', body: 'Bug description' })
        }).then(r => r.status)"#,
    )
    .await
    .unwrap();

    let eval_calls = mock.calls_for("Runtime.evaluate");
    assert_eq!(eval_calls.len(), 2);
    assert!(eval_calls[0].args[0].as_str().unwrap().contains("repos"));
    assert!(eval_calls[1].args[0].as_str().unwrap().contains("issues"));
}

/// ```typescript
/// // Playwright
/// test('should create feature request', async ({ request }) => {
///   const newIssue = await request.post('/repos/user/Test-Repo-1/issues', {
///     data: { title: '[Feature] request 1', body: 'Feature description' }
///   });
///   expect(newIssue.ok()).toBeTruthy();
/// });
/// ```
#[tokio::test]
async fn should_create_feature_request() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    page.evaluate(
        r#"fetch('https://api.github.com/repos/user/Test-Repo-1/issues', {
            method: 'POST',
            body: JSON.stringify({ title: '[Feature] request 1', body: 'Feature description' })
        }).then(r => r.json())"#,
    )
    .await
    .unwrap();

    let eval_calls = mock.calls_for("Runtime.evaluate");
    assert_eq!(eval_calls.len(), 1);
    let expr = eval_calls[0].args[0].as_str().unwrap();
    assert!(expr.contains("[Feature] request 1"));
}
