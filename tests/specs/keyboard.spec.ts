import { defineSuite } from "../src/harness.js";

export default defineSuite("keyboard", (t) => {
  t.test("should type text into focused input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    await ctx.click("#text");
    await ctx.keyboardType("Hello");
    const value = await ctx.inputValue("#text");
    return { value };
  });

  t.test("should press Enter key", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.focus("#focus-input");
    await ctx.keyboardPress("Enter");
    const output = await ctx.textContent("#keyboard-output");
    return { hasEnter: output?.includes("key:Enter") ?? false };
  });

  t.test("should press Tab key", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.focus("#focus-input");
    await ctx.keyboardPress("Tab");
    const output = await ctx.textContent("#keyboard-output");
    return { hasTab: output?.includes("key:Tab") ?? false };
  });

  t.test("should press arrow keys", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/interactive.html");
    await ctx.focus("#focus-input");
    await ctx.keyboardPress("ArrowDown");
    const output = await ctx.textContent("#keyboard-output");
    return { hasArrow: output?.includes("key:ArrowDown") ?? false };
  });
});
