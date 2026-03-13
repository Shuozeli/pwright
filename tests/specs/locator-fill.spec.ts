import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-fill", (t) => {
  t.test("should fill text input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    await ctx.fill("#text", "Hello World");
    const value = await ctx.inputValue("#text");
    return { value };
  });

  t.test("should fill email input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    await ctx.fill("#email", "test@example.com");
    const value = await ctx.inputValue("#email");
    return { value };
  });

  t.test("should type into input", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    await ctx.click("#password");
    await ctx.type("#password", "secret123");
    const value = await ctx.inputValue("#password");
    return { value };
  });

  t.test("should check and uncheck checkbox", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/input.html");
    const initial = await ctx.isChecked("#checkbox");
    await ctx.check("#checkbox");
    const afterCheck = await ctx.isChecked("#checkbox");
    await ctx.uncheck("#checkbox");
    const afterUncheck = await ctx.isChecked("#checkbox");
    return { initial, afterCheck, afterUncheck };
  });
});
