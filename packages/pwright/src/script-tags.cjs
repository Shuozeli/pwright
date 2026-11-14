'use strict';

const { readFile } = require('node:fs/promises');

// Filesystem source loading is language-specific. DOM execution, remote value
// ownership and the returned ElementHandle all reuse the existing Rust
// evaluation/Handle leaves; this adapter owns no browser session or queue.
module.exports = function scriptTags({ validateOptions }) {
  async function add(frame, options = {}) {
    const api = 'page.addScriptTag';
    if (!options || typeof options !== 'object' || Array.isArray(options))
      throw new Error('Provide an object with a `url`, `path` or `content` property');
    validateOptions(options, ['url', 'path', 'content', 'type'], api);
    const copy = { ...options };
    if (copy.path) {
      copy.content = await readFile(copy.path, 'utf8');
      copy.content += `\n//# sourceURL=${copy.path.replace(/\n/g, '')}`;
    }
    const url = copy.url ?? null;
    const content = copy.content ?? null;
    const type = copy.type ?? '';
    if (!url && !content)
      throw new Error('Provide an object with a `url`, `path` or `content` property');
    if (url !== null && typeof url !== 'string') throw new TypeError(`${api}: url must be a string`);
    if (content !== null && typeof content !== 'string') throw new TypeError(`${api}: content must be a string`);
    if (typeof type !== 'string') throw new TypeError(`${api}: type must be a string`);
    return frame.evaluateHandle(async ({ url, content, type }) => {
      const script = document.createElement('script');
      if (url !== null) script.src = url;
      if (type) script.type = type;
      if (content !== null) script.text = content;
      if (url !== null) {
        const completion = new Promise((resolve, reject) => {
          script.onload = resolve;
          script.onerror = () => reject(new Error(`Failed to load script at ${script.src}`));
        });
        document.head.appendChild(script);
        await completion;
        return script;
      }

      // Inline module execution is asynchronous. Playwright publishes the
      // ElementHandle immediately and leaves readiness to waitForFunction;
      // waiting for a module load event here can stall forever. Keep one task
      // boundary solely to observe the page's CSP violation signal.
      let cspError = null;
      const onViolation = event => {
        if (event.blockedURI === 'inline' || event.blockedURI === '')
          cspError = new Error(`Refused to execute inline script because it violates ${event.violatedDirective || 'Content Security Policy'}`);
      };
      document.addEventListener('securitypolicyviolation', onViolation);
      script.onerror = event => {
        cspError ??= new Error(`Failed to add inline script: ${event.type}`);
      };
      document.head.appendChild(script);
      await new Promise(resolve => setTimeout(resolve, 0));
      document.removeEventListener('securitypolicyviolation', onViolation);
      if (cspError) throw cspError;
      return script;
    }, { url, content, type });
  }

  return { add };
};
