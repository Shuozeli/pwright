import { copyFileSync } from 'node:fs';
import { join } from 'node:path';

// Packaging only; these notices accompany each selected native binary.
export function installNativeNotices(workspace, destination, features = []) {
  for (const name of ['NOTICE', 'LICENSE-MIT', 'LICENSE-APACHE-2.0']) {
    copyFileSync(join(workspace, 'crates/pwright-node-host', name),
      join(destination, `pwright-node-host.${name}`));
  }
  if (features.includes('file-input')) {
    for (const [crate, names] of [
      ['pwright-file-input-fs', ['MIME-LICENSE']],
      ['pwright-backend-cdp-file-input', ['NOTICE', 'LICENSE.playwright']],
    ]) {
      for (const name of names)
        copyFileSync(join(workspace, 'crates', crate, name),
          join(destination, `${crate}.${name}`));
    }
  }
}
