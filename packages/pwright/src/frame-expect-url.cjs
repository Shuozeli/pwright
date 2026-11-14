'use strict';

// Playwright's matcher serializes string/RegExp semantics in JavaScript. Rust
// owns the URL observation, lifecycle ordering, admission and timeout loop.
module.exports = function frameUrlExpectation({ enabled, mapError, validateOptions, UnsupportedOperationError }) {
  function setContextBaseURL(context, baseURL) {
    context._baseURL = baseURL;
    if (enabled) context._options = Object.freeze({ baseURL });
  }

  function expectedMatcher(expectedText) {
    if (!Array.isArray(expectedText) || expectedText.length !== 1)
      throw new TypeError('frame._expect: expectedText must contain exactly one value');
    const expected = expectedText[0];
    if (!expected || typeof expected !== 'object' || Array.isArray(expected))
      throw new TypeError('frame._expect: expectedText value must be an object');
    const string = typeof expected.string === 'string' ? expected.string : undefined;
    const regexSource = typeof expected.regexSource === 'string' ? expected.regexSource : undefined;
    if ((string === undefined) === (regexSource === undefined))
      throw new TypeError('frame._expect: expectedText value must contain either string or regexSource');
    if (expected.ignoreCase !== undefined && typeof expected.ignoreCase !== 'boolean')
      throw new TypeError('frame._expect: ignoreCase must be a boolean');
    if (expected.normalizeWhiteSpace !== undefined && typeof expected.normalizeWhiteSpace !== 'boolean')
      throw new TypeError('frame._expect: normalizeWhiteSpace must be a boolean');
    if (expected.matchSubstring !== undefined && typeof expected.matchSubstring !== 'boolean')
      throw new TypeError('frame._expect: matchSubstring must be a boolean');

    if (regexSource !== undefined) {
      if (expected.regexFlags !== undefined && typeof expected.regexFlags !== 'string')
        throw new TypeError('frame._expect: regexFlags must be a string');
      const flags = new Set(expected.regexFlags ?? '');
      if (expected.ignoreCase === false) flags.delete('i');
      if (expected.ignoreCase === true) flags.add('i');
      const regex = new RegExp(regexSource, [...flags].join(''));
      return candidate => {
        regex.lastIndex = 0;
        return regex.test(candidate);
      };
    }

    const normalize = value => {
      let result = expected.normalizeWhiteSpace
        ? value.replace(/[\u200b\u00ad]/g, '').trim().replace(/\s+/g, ' ')
        : value;
      if (expected.ignoreCase) result = result.toLocaleLowerCase();
      return result;
    };
    const target = normalize(string);
    return expected.matchSubstring
      ? candidate => normalize(candidate).includes(target)
      : candidate => normalize(candidate) === target;
  }

  async function expect(frame, expression, options = {}) {
    if (!enabled) throw new UnsupportedOperationError('frame._expect (select wait-for-url native profile)');
    if (expression !== 'to.have.url') throw new UnsupportedOperationError(`frame._expect ${expression}`);
    validateOptions(options, ['expectedText', 'isNot', 'timeout', 'signal', 'title'], 'frame._expect');
    if (options.signal !== undefined)
      throw new UnsupportedOperationError('frame._expect to.have.url signal');
    if (options.title !== undefined && typeof options.title !== 'string')
      throw new TypeError('frame._expect: title must be a string');
    const timeout = options.timeout ?? 5_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError('frame._expect: timeout must be an integer in [0, 4294967295]');
    if (options.isNot !== undefined && typeof options.isNot !== 'boolean')
      throw new TypeError('frame._expect: isNot must be a boolean');
    const match = expectedMatcher(options.expectedText);
    const isNot = options.isNot ?? false;
    const page = frame._page;
    const session = page._context._browser._session;
    if (typeof session.frameWaitForUrl !== 'function')
      throw new UnsupportedOperationError('frame._expect to.have.url (rebuild wait-for-url native profile)');

    let received;
    let matches;
    try {
      const url = await session.frameWaitForUrl(page._id, frame._id, candidate => {
        received = candidate;
        matches = match(candidate);
        return matches !== isNot;
      }, 'commit', timeout);
      return { matches, received: { value: url } };
    } catch (error) {
      const mapped = mapError(error, 'frame._expect');
      if (mapped?.code !== 'PWRIGHT_TIMEOUT') throw mapped;
      return {
        matches: isNot,
        received: { value: received ?? frame.url() },
        timedOut: true,
        log: [`Timeout ${timeout}ms exceeded`],
      };
    }
  }

  return { setContextBaseURL, expect };
};
