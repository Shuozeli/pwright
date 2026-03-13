import { defineSuite } from "../src/harness.js";

export default defineSuite("touchscreen", (t) => {
  t.test("should dispatch touch tap", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/touchscreen.html");
    // Tap the center of the target element (100, 100)
    await ctx.mouseClick(100, 100);
    const result = await ctx.evaluate(
      `document.getElementById('result').textContent`
    );
    return { result };
  });
});
