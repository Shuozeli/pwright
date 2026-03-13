import { defineSuite } from "../src/harness.js";

export default defineSuite("screenshot", (t) => {
  t.test("should capture PNG screenshot", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const buf = await ctx.screenshot("png");
    return {
      hasData: buf.length > 0,
      isPng: buf[0] === 0x89 && buf[1] === 0x50, // PNG magic bytes
    };
  });

  t.test("should capture screenshot of different pages", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    const buf = await ctx.screenshot("png");
    return {
      hasData: buf.length > 100,
      isPng: buf[0] === 0x89 && buf[1] === 0x50,
    };
  });
});
