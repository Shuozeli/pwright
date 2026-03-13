import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-label", (t) => {
  t.test("should find input by label for attribute", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-label.html");
    const tag = await ctx.getByLabel("Email");
    return { tag };
  });

  t.test("should find input by wrapping label", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-label.html");
    const tag = await ctx.getByLabel("Username");
    return { tag };
  });
});
