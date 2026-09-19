import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, renameSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import artifactConfig from '../src/native-artifact.cjs';
import { installNativeNotices } from './native-notices.mjs';

const workspace = fileURLToPath(new URL('../../../', import.meta.url));
const run = (args, capture = false) => {
  const result = spawnSync('cargo', args, {
    cwd: workspace,
    env: { ...process.env, CARGO_INCREMENTAL: '0' },
    encoding: 'utf8',
    stdio: capture ? ['ignore', 'pipe', 'inherit'] : 'inherit',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`cargo ${args[0]} failed (${result.status})`);
  return result.stdout;
};

const { nativeArtifact, nativeFeatures } = artifactConfig;
const features = nativeFeatures(nativeArtifact.profile);
run(['build', '--locked', '--release', '-p', 'pwright-node-addon',
  ...(features.length ? ['--features', features.join(',')] : [])]);
const metadata = JSON.parse(run(['metadata', '--no-deps', '--format-version', '1'], true));
const library = process.platform === 'darwin' ? 'libpwright_node_addon.dylib'
  : process.platform === 'win32' ? 'pwright_node_addon.dll'
  : 'libpwright_node_addon.so';
const destination = dirname(nativeArtifact.path);
mkdirSync(destination, { recursive: true });
installNativeNotices(workspace, destination, features);
// Never truncate a dynamic library that another Node process may have mapped.
const staged = join(destination, `pwright.${process.pid}.node`);
copyFileSync(join(metadata.target_directory, 'release', library), staged);
renameSync(staged, nativeArtifact.path);
console.log(`Built ${process.platform}-${process.arch} Node-API addon (${nativeArtifact.profile}): ${nativeArtifact.path}`);
