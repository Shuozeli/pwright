'use strict';

// Import-time language capture only, matching the pinned client's debug module
// lifetime. Rust chooses undefined-only fallback and inspector classification.
module.exports = Object.freeze([
  process.env.PWDEBUG,
  process.env.npm_config_pwdebug,
  process.env.npm_package_config_pwdebug,
]);
