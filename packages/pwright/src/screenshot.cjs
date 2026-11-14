'use strict';

// Public option marshalling only; no capture, geometry, CDP or byte decoding.
module.exports = function screenshotApi({ validateOptions }) {
  return function options(value = {}, api = 'page.screenshot', locator = false) {
    const fields = locator
      ? ['type', 'quality', 'scale', 'timeout']
      : ['type', 'quality', 'fullPage', 'clip', 'scale', 'timeout'];
    validateOptions(value, fields, api);
    const type = value.type ?? 'png';
    if (!['png', 'jpeg'].includes(type)) throw new TypeError(`${api}: type must be png or jpeg`);
    if (value.quality !== undefined && (!Number.isInteger(value.quality) || value.quality < 0 || value.quality > 100 || type !== 'jpeg'))
      throw new TypeError(`${api}: quality requires jpeg and an integer in [0, 100]`);
    if (value.fullPage !== undefined && typeof value.fullPage !== 'boolean') throw new TypeError(`${api}: fullPage must be a boolean`);
    const scale = value.scale ?? 'device';
    if (!['device', 'css'].includes(scale)) throw new TypeError(`${api}: scale must be device or css`);
    const timeout = value.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff) throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    let clip;
    if (value.clip !== undefined) {
      validateOptions(value.clip, ['x', 'y', 'width', 'height'], 'page.screenshot clip');
      clip = { x: value.clip.x, y: value.clip.y, width: value.clip.width, height: value.clip.height };
      if (!Object.values(clip).every(Number.isFinite) || clip.x < 0 || clip.y < 0 || clip.width <= 0 || clip.height <= 0)
        throw new TypeError(`${api}: clip requires finite non-negative coordinates and positive dimensions`);
    }
    return { format: type === 'png' ? 0 : 1, quality: value.quality, fullPage: value.fullPage ?? false,
      scale: scale === 'device' ? 0 : 1, clip, timeoutMs: timeout };
  };
};
