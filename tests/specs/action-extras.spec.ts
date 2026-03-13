/**
 * Action extras golden test — check, uncheck, dblclick.
 */
import { defineSuite } from "../src/harness.js";

export default defineSuite("action-extras", (t) => {
  t.test("check checkbox", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/action-extras.html");
    const before = await ctx.isChecked("#agree");
    await ctx.check("#agree");
    const after = await ctx.isChecked("#agree");
    return { before, after };
  });

  t.test("uncheck checkbox", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/action-extras.html");
    // First check it
    await ctx.check("#agree");
    const checked = await ctx.isChecked("#agree");
    // Then uncheck
    await ctx.uncheck("#agree");
    const unchecked = await ctx.isChecked("#agree");
    return { checked, unchecked };
  });

  t.test("double-click fires dblclick event", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/action-extras.html");
    const before = await ctx.textContent("#dblclick-result");
    await ctx.dblclick("#dblclick-target");
    const after = await ctx.textContent("#dblclick-result");
    return { before, after };
  });
});
