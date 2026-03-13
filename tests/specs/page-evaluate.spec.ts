import { defineSuite } from "../src/harness.js";

export default defineSuite("page-evaluate", (t) => {
  t.test("should evaluate arithmetic", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE);
    const result = await ctx.evaluate("1 + 2 + 3");
    return { result };
  });

  t.test("should evaluate string expression", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE);
    const result = await ctx.evaluate("'hello' + ' ' + 'world'");
    return { result };
  });

  t.test("should evaluate boolean", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE);
    const result = await ctx.evaluate("true && false");
    return { result };
  });

  t.test("should evaluate null", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE);
    const result = await ctx.evaluate("null");
    return { result };
  });

  t.test("should evaluate DOM query", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const result = await ctx.evaluate("document.title");
    return { result };
  });
});
