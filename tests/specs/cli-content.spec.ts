/**
 * CLI comparison: page content and cookies
 */
import { defineCliSuite, normalizeEvalOutput, normalizeOutput, stripAnsi } from "../src/cli-harness.js";

export default defineCliSuite("cli-content", (t) => {
  t.test("should get page URL after navigation", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "window.location.href");
    const url = normalizeEvalOutput(result.stdout);
    return { hasContentPath: url.includes("/content.html") };
  });

  t.test("should get page title", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.title");
    return { title: normalizeEvalOutput(result.stdout) };
  });

  t.test("should get inner text", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.getElementById('heading').innerText");
    return { text: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate document.cookie", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    // Set a cookie
    await cli.exec("eval", "document.cookie = 'test=123'");
    const result = await cli.exec("eval", "document.cookie");
    const val = normalizeEvalOutput(result.stdout);
    return { hasCookie: val.includes("test=123") };
  });

  t.test("should evaluate HTML content", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.getElementById('heading').tagName");
    return { tag: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate element count", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.querySelectorAll('li').length");
    return { count: normalizeEvalOutput(result.stdout) };
  });

  t.test("should check hidden element visibility", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "getComputedStyle(document.getElementById('hidden')).display");
    return { display: normalizeEvalOutput(result.stdout) };
  });
});
