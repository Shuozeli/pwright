//! Mock Filesystem — Directory Reader
//!
//! Equivalent of playwright/examples/mock-filesystem/tests/directory-reader.spec.js
//!
//! Uses FakeCdpClient to verify text content from a real in-memory DOM.

use pwright_bridge::playwright::Page;
use pwright_fake::FakeCdpClient;
use std::sync::Arc;

/// ```javascript
/// // Playwright
/// test('should display directory tree', async ({ page }) => {
///   await page.goto('/ls-dir.html');
///   await page.locator('button', { hasText: 'Open directory' }).click();
///   await expect(page.locator('#dir')).toContainText([
///     'file1', 'dir1', 'file2', 'file3', 'dir2', 'file4', 'file5'
///   ]);
/// });
/// ```
#[tokio::test]
async fn should_display_directory_tree() {
    let fake = Arc::new(FakeCdpClient::from_html(
        r#"
        <div>
            <button>Open directory</button>
            <div id="dir">file1
dir1
  file2
  file3
dir2
  file4
  file5</div>
        </div>
    "#,
    ));
    let page = Page::new(fake.clone());

    // Click "Open directory" button
    page.locator("button").click().await.unwrap();

    // Read directory listing from real DOM
    let text = page.locator("#dir").text_content().await.unwrap();
    assert!(text.is_some());
    let content = text.unwrap();
    for expected in &["file1", "dir1", "file2", "file3", "dir2", "file4", "file5"] {
        assert!(content.contains(expected), "should contain {}", expected);
    }
}
