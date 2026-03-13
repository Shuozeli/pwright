import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-query", (t) => {
  t.test("should get text content", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const text = await ctx.textContent(".description");
    return { text };
  });

  t.test("should get inner text", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const text = await ctx.innerText(".description");
    return { text };
  });

  t.test("should get attribute", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const href = await ctx.getAttribute("#link", "href");
    const target = await ctx.getAttribute("#link", "target");
    const missing = await ctx.getAttribute("#link", "data-missing");
    return { href, target, missing };
  });

  t.test("should get data attributes", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const value = await ctx.getAttribute("#data", "data-value");
    const label = await ctx.getAttribute("#data", "data-label");
    return { value, label };
  });

  t.test("should count elements", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const items = await ctx.locatorCount("#list li");
    const links = await ctx.locatorCount("a");
    const missing = await ctx.locatorCount(".nonexistent");
    return { items, links, missing };
  });

  t.test("should check visibility", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const heading = await ctx.isVisible("#heading");
    const hidden = await ctx.isVisible("#hidden");
    const invisible = await ctx.isVisible("#invisible");
    return { heading, hidden, invisible };
  });
});
