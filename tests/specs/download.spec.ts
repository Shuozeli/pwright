import { defineSuite } from "../src/harness.js";

export default defineSuite("download", (t) => {
  t.test("should trigger download and verify", async (ctx) => {
    // Generate a simple download link
    const html = `
      <!DOCTYPE html>
      <html>
      <head><title>Download Test</title></head>
      <body>
        <a id="dl" href="data:text/plain;base64,VGhpcyBpcyBhIHRlc3QgZG93bmxvYWQh" download="test_file.txt">Download File</a>
      </body>
      </html>
    `;
    
    await ctx.goto(`data:text/html;base64,${Buffer.from(html).toString("base64")}`);
    
    // We wait for the download to complete
    const downloadPath = await ctx.waitForDownload(async () => {
      await ctx.click("#dl");
    });
    
    // The download path is mocked in golden tests to be deterministic
    return { 
      download_triggered: true,
      downloaded_file: downloadPath
    };
  });
});
