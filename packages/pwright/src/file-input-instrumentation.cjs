'use strict';

// Internal composition port only. Pwright does not yet expose Playwright's
// tracing/debugger/recorder/screencast listener APIs, so the selected leaf uses
// the same awaited empty-listener checkpoint without inventing a public hook.
const emptyInputInstrumentation = Object.freeze({
  async onBeforeInputAction() {},
});

function progressView(progress, callId) {
  return Object.freeze({
    callId,
    race(value) { return progress.race(value); },
  });
}

async function beforeInputAction(port, progress, callId, receiver) {
  if (!port || typeof port.onBeforeInputAction !== 'function')
    throw new TypeError('Input instrumentation port must implement onBeforeInputAction');
  // Preserve the four-value call position for a future adapter. `receiver` is
  // today's internal Node facade identity, not yet Playwright's server-side
  // SdkObject contract. Public ElementHandle.setInputFiles supplies neither
  // point nor box.
  await port.onBeforeInputAction(progressView(progress, callId), receiver, undefined, undefined);
}

module.exports = { beforeInputAction, emptyInputInstrumentation };
