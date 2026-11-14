'use strict';

const os = require('node:os');

function fromEnvironment(environment, name) {
  const direct = environment[name];
  if (direct !== undefined) return direct;
  const lower = name.toLowerCase();
  const npmConfig = environment[`npm_config_${lower}`];
  if (npmConfig !== undefined) return npmConfig;
  return environment[`npm_package_config_${lower}`];
}

// The official registry computes its directory while the package is imported.
// Capture inputs here, but leave path policy and executable metadata in Rust.
module.exports = function captureBrowserRegistrySnapshot(
  environment = process.env,
  cwd = process.cwd(),
  home = os.homedir(),
) {
  return Object.freeze({
    browsersPath: fromEnvironment(environment, 'PLAYWRIGHT_BROWSERS_PATH'),
    initCwd: fromEnvironment(environment, 'INIT_CWD'),
    xdgCacheHome: environment.XDG_CACHE_HOME,
    localAppData: environment.LOCALAPPDATA,
    home,
    cwd,
    hostPlatformOverride: environment.PLAYWRIGHT_HOST_PLATFORM_OVERRIDE,
  });
};
