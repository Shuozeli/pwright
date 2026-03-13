/**
 * Navigation extras golden test — reload, goBack, goForward.
 */
import { defineSuite } from "../src/harness.js";

export default defineSuite("navigation-extras", (t) => {
  t.test("reload preserves page", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/navigation-extras.html");
    const titleBefore = await ctx.title();

    await ctx.reload();
    const titleAfter = await ctx.title();

    return { titleBefore, titleAfter, same: titleBefore === titleAfter };
  });

  t.test("go back after navigation", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/navigation-extras.html");
    const page1Title = await ctx.title();

    // Navigate to page 2
    await ctx.goto(ctx.server.PREFIX + "/navigation-extras-page2.html");
    const page2Title = await ctx.title();

    // Go back
    await ctx.goBack();
    await new Promise((r) => setTimeout(r, 500));
    const backTitle = await ctx.title();

    return { page1Title, page2Title, backTitle };
  });

  t.test("go forward after going back", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/navigation-extras.html");
    await ctx.goto(ctx.server.PREFIX + "/navigation-extras-page2.html");

    // Go back, then forward
    await ctx.goBack();
    await new Promise((r) => setTimeout(r, 500));

    await ctx.goForward();
    await new Promise((r) => setTimeout(r, 500));
    const forwardTitle = await ctx.title();

    return { forwardTitle };
  });
});
