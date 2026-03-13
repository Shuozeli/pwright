import { defineSuite } from "../src/harness.js";

export default defineSuite("page-goto", (t) => {
  t.test("should navigate to empty page", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE);
    const url = await ctx.url();
    return { url: url.replace(ctx.server.PREFIX, "SERVER") };
  });

  t.test("should navigate to content page", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const title = await ctx.title();
    return { title };
  });

  t.test("should navigate to data URL", async (ctx) => {
    await ctx.goto("data:text/html,<h1>Hello</h1>");
    const url = await ctx.url();
    return { isDataUrl: url.startsWith("data:") };
  });

  t.test("should reload page", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    await ctx.reload();
    const title = await ctx.title();
    return { title };
  });

  t.test("should navigate with hash", async (ctx) => {
    await ctx.goto(ctx.server.EMPTY_PAGE + "#section1");
    const url = await ctx.url();
    return { hasHash: url.includes("#section1") };
  });
});
