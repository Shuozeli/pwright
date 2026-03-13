/**
 * playwright-cli adapter — runs playwright-cli as subprocess.
 *
 * Note: playwright-cli snapshot saves to a YAML file.
 * This adapter reads the file and returns its content as stdout.
 */

import { resolve } from "path";
import { readFileSync, existsSync, readdirSync, unlinkSync } from "fs";
import { execCli, type CliRunner, type CliResult } from "./cli-harness.js";

const CLI_JS = resolve(import.meta.dirname!, "../../playwright-cli/playwright-cli.js");
const PW_CLI_DIR = resolve(import.meta.dirname!, "../../playwright-cli");
const SNAPSHOT_DIR = resolve(PW_CLI_DIR, ".playwright-cli");

/** Find the most recent snapshot YAML file */
function latestSnapshot(): string | null {
  if (!existsSync(SNAPSHOT_DIR)) return null;
  const files = readdirSync(SNAPSHOT_DIR)
    .filter((f) => f.endsWith(".yml"))
    .sort()
    .reverse();
  return files.length > 0 ? resolve(SNAPSHOT_DIR, files[0]) : null;
}

/** Clean all snapshot files */
function cleanSnapshots() {
  if (!existsSync(SNAPSHOT_DIR)) return;
  for (const f of readdirSync(SNAPSHOT_DIR).filter((f) => f.endsWith(".yml"))) {
    try { unlinkSync(resolve(SNAPSHOT_DIR, f)); } catch {}
  }
}

export function createPlaywrightCliRunner(): CliRunner {
  return {
    async exec(command: string, ...args: string[]): Promise<CliResult> {
      // Clean snapshots before snapshot command so we can find the new one
      if (command === "snapshot") {
        cleanSnapshots();
      }

      const result = await execCli("node", [CLI_JS, command, ...args], {
        cwd: PW_CLI_DIR,
        timeout: 20000,
      });

      // For snapshot, read the YAML file content and append to stdout
      if (command === "snapshot") {
        const snapFile = latestSnapshot();
        if (snapFile) {
          const content = readFileSync(snapFile, "utf-8");
          result.stdout = result.stdout + "\n" + content;
        }
      }

      return result;
    },

    async cleanup(): Promise<void> {
      await execCli("node", [CLI_JS, "close"], { cwd: PW_CLI_DIR, timeout: 5000 });
      cleanSnapshots();
    },
  };
}
