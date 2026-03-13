import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-click", (t) => {
  t.test("should click button", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.click("#counter");
    const text = await ctx.textContent("#counter");
    return { text };
  });

  t.test("should click button multiple times", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.click("#counter");
    await ctx.click("#counter");
    await ctx.click("#counter");
    const text = await ctx.textContent("#counter");
    return { text };
  });

  t.test("should dblclick element", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.dblclick("#dblclick");
    const text = await ctx.textContent("#dblclick");
    return { text };
  });

  t.test("should focus input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.focus("#focus-input");
    const focused = await ctx.getAttribute("#focus-input", "data-focused");
    return { focused };
  });
});
