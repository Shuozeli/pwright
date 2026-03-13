import { defineSuite } from "../src/harness.js";

export default defineSuite("locator-role", (t) => {
  t.test("should find button by implicit role", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-role.html");
    const text = await ctx.getByRole("button", { name: "Submit" });
    return { text };
  });

  t.test("should find link by implicit role", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-role.html");
    const text = await ctx.getByRole("link", { name: "About" });
    return { text };
  });

  t.test("should find element by explicit role", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/locator-role.html");
    const text = await ctx.getByRole("alert");
    return { text };
  });
});
