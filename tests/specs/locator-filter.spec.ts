import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-filter", (t) => {
  t.test("should filter by text content", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-filter.html");
    // Get the text of the first .item that contains "Ban"
    const text = await ctx.evaluate(
      `(() => {
        const els = [...document.querySelectorAll('.item')];
        const el = els.find(e => e.textContent.includes('Ban'));
        return el ? el.textContent : null;
      })()`
    );
    return { text };
  });
});
