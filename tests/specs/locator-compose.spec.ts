import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-compose", (t) => {
  t.test("should find with OR composition", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-compose.html");
    // Count elements matching "button.primary, a.primary" (OR)
    const count = await ctx.locatorCount("button.primary, a.primary");
    return { count };
  });

  t.test("should find with AND composition", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-compose.html");
    // Find button that is also .primary (AND)
    const text = await ctx.textContent("button.primary");
    return { text };
  });
});
