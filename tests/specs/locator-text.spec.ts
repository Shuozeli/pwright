import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-text", (t) => {
  t.test("should find by substring text", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-text.html");
    const text = await ctx.getByText("Hello");
    return { text };
  });

  t.test("should find by exact text", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-text.html");
    const text = await ctx.getByText("Click Me", { exact: true });
    return { text };
  });
});
