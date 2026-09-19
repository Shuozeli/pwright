'use strict';

// Public option marshalling only; no capture, geometry, CDP or byte decoding.
module.exports = function screenshotApi({ validateOptions }) {
  return function options(value = {}) {
    validateOptions(value, ['type', 'quality', 'fullPage', 'clip', 'scale', 'timeout'], 'page.screenshot');
    const type = value.type ?? 'png';
    if (!['png', 'jpeg'].includes(type)) throw new TypeError('page.screenshot: type must be png or jpeg');
    if (value.quality !== undefined && (!Number.isInteger(value.quality) || value.quality < 0 || value.quality > 100 || type !== 'jpeg'))
      throw new TypeError('page.screenshot: quality requires jpeg and an integer in [0, 100]');
    if (value.fullPage !== undefined && typeof value.fullPage !== 'boolean') throw new TypeError('page.screenshot: fullPage must be a boolean');
    const scale = value.scale ?? 'device';
    if (!['device', 'css'].includes(scale)) throw new TypeError('page.screenshot: scale must be device or css');
    const timeout = value.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff) throw new TypeError('page.screenshot: timeout must be an integer in [0, 4294967295]');
    let clip;
    if (value.clip !== undefined) {
      validateOptions(value.clip, ['x', 'y', 'width', 'height'], 'page.screenshot clip');
      clip = { x: value.clip.x, y: value.clip.y, width: value.clip.width, height: value.clip.height };
      if (!Object.values(clip).every(Number.isFinite) || clip.x < 0 || clip.y < 0 || clip.width <= 0 || clip.height <= 0)
        throw new TypeError('page.screenshot: clip requires finite non-negative coordinates and positive dimensions');
    }
    return { format: type === 'png' ? 0 : 1, quality: value.quality, fullPage: value.fullPage ?? false,
      scale: scale === 'device' ? 0 : 1, clip, timeoutMs: timeout };
  };
};
