'use strict';

const { readFile } = require('node:fs/promises');

// Language-shaped source normalization only. Rust owns bounded registration,
// target/session routing, persistence across documents, and rollback.
module.exports = function initScripts({ mapError, UnsupportedOperationError }) {
  async function source(script, api = 'page.addInitScript') {
    if (typeof script === 'function') return `(${String(script)})();\n`;
    if (typeof script === 'string') return `${script}\n`;
    if (!script || typeof script !== 'object' || Array.isArray(script))
      throw new TypeError(`${api}: script must be a function, string, or { path | content }`);
    if (typeof script.content === 'string') return `${script.content}\n`;
    if (typeof script.path === 'string') return `${await readFile(script.path, 'utf8')}\n`;
    throw new Error('Either path or content property must be present');
  }

  async function add(target, script, hasArgument, api, method) {
    if (hasArgument) throw new UnsupportedOperationError(`${api} argument`);
    const value = await source(script, api);
    const context = api === 'page.addInitScript' ? target.context() : target;
    const session = context.browser()._session;
    if (typeof session[method] !== 'function')
      throw new UnsupportedOperationError(`${api} (select init-scripts native profile)`);
    try { await session[method](target._id, value); }
    catch (error) { throw mapError(error, api); }
  }

  return {
    add: (page, script, hasArgument) => add(
      page, script, hasArgument, 'page.addInitScript', 'pageAddInitScript',
    ),
    addContext: (context, script, hasArgument) => add(
      context, script, hasArgument, 'browserContext.addInitScript', 'contextAddInitScript',
    ),
    source,
  };
};
