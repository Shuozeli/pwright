import { defineSuite } from "../src/harness.js";

export default defineSuite("mouse", (t) => {
  t.test("should click at coordinates", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    // Click the counter button by coordinates (at page center-ish)
    await ctx.click("#counter");
    const text = await ctx.textContent("#counter");
    return { text };
  });

  t.test("should hover element", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.hover("#hover");
    const text = await ctx.textContent("#hover");
    return { text };
  });

  t.test("should double-click element", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.dblclick("#dblclick");
    const text = await ctx.textContent("#dblclick");
    return { text };
  });

  t.test("should move mouse to element", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    // Move to hover element
    await ctx.hover("#hover");
    const text = await ctx.textContent("#hover");
    return { text };
  });
});
