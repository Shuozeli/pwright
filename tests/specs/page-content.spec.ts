import { defineSuite } from "../src/harness.js";

export default defineSuite("page-content", (t) => {
  t.test("should get page title", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const title = await ctx.title();
    return { title };
  });

  t.test("should get page content containing elements", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const html = await ctx.content();
    return {
      hasHeading: html.includes("Hello pwright"),
      hasList: html.includes("Item 1"),
      hasHidden: html.includes("Hidden content"),
    };
  });

  t.test("should get text content of heading", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const text = await ctx.textContent("#heading");
    return { text };
  });

  t.test("should get text content of nested element", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const text = await ctx.textContent("#nested");
    return { text };
  });
});
