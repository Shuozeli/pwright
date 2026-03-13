/**
 * pwright CLI adapter — runs the pwright binary as subprocess.
 */

import { resolve } from "path";
import { execCli, type CliRunner, type CliResult } from "./cli-harness.js";

const PWRIGHT_BIN = resolve(import.meta.dirname!, "../../target/debug/pwright");

export function createPwrightCliRunner(cdpUrl: string = "http://localhost:9222"): CliRunner {
  return {
    async exec(command: string, ...args: string[]): Promise<CliResult> {
      return execCli(PWRIGHT_BIN, ["--cdp", cdpUrl, command, ...args], {
        timeout: 20000,
      });
    },

    async cleanup(): Promise<void> {
      await execCli(PWRIGHT_BIN, ["close"], { timeout: 5000 });
    },
  };
}
