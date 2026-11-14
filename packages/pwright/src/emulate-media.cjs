'use strict';

module.exports = function emulateMediaApi({ mapError, validateOptions, UnsupportedOperationError }) {
  const choices = Object.freeze({
    media: Object.freeze({ values: ['screen', 'print'], expected: 'screen|print|no-override' }),
    colorScheme: Object.freeze({ values: ['dark', 'light', 'no-preference'], expected: 'dark|light|no-preference|no-override' }),
    reducedMotion: Object.freeze({ values: ['reduce', 'no-preference'], expected: 'reduce|no-preference|no-override' }),
    forcedColors: Object.freeze({ values: ['active', 'none'], expected: 'active|none|no-override' }),
    contrast: Object.freeze({ values: ['more', 'no-preference'], expected: 'more|no-preference|no-override' }),
  });

  function value(options, name) {
    if (options[name] === undefined) return undefined;
    if (options[name] === null) return 'null';
    if (!choices[name].values.includes(options[name]))
      throw new TypeError(`${name}: expected one of (${choices[name].expected})`);
    return options[name];
  }

  return async function emulateMedia(session, pageId, options = {}) {
    const api = 'page.emulateMedia';
    validateOptions(options, Object.keys(choices), api);
    if (typeof session.pageEmulateMedia !== 'function')
      throw new UnsupportedOperationError(`${api} (select emulate-media native feature)`);
    const media = value(options, 'media');
    const colorScheme = value(options, 'colorScheme');
    const reducedMotion = value(options, 'reducedMotion');
    const forcedColors = value(options, 'forcedColors');
    const contrast = value(options, 'contrast');
    try {
      await session.pageEmulateMedia(pageId, media, colorScheme, reducedMotion, forcedColors, contrast);
    } catch (error) {
      throw mapError(error, api);
    }
  };
};
