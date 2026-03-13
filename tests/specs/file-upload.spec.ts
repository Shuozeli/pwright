import { defineSuite } from "../src/harness.js";

export default defineSuite("file-upload", (t) => {
  t.test("should set file on input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/file-upload.html");
    
    // Set files on the file input
    await ctx.setInputFiles("#file-input", ["/tmp/test-file.txt"]);
    
    // Verify the file was set by checking if the input has files
    const hasFiles = await ctx.evaluate(
      `document.querySelector('#file-input').files.length > 0 || document.querySelector('#file-input').getAttribute('data-files-set') !== null`
    );
    
    return { hasFiles };
  });
});
