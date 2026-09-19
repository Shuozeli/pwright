'use strict';
const { join } = require('node:path');

// Packaging selection only. Every browser operation remains in the Rust host.
const profiles = Object.freeze({
  default: Object.freeze([]),
  handles: Object.freeze(['handles']),
  'retained-file-input': Object.freeze(['handles', 'file-input', 'retained-file-input']),
  'retained-file-input-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-target-marking': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking']),
  'retained-target-marking-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-target-prevalidation': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation']),
  'retained-target-prevalidation-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-input-instrumentation': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation']),
  'retained-input-instrumentation-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-call-argument-serialization': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization']),
  'retained-call-argument-serialization-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-serialized-payload-import': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization', 'retained-serialized-payload-import']),
  'retained-serialized-payload-import-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization', 'retained-serialized-payload-import', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'retained-serialized-payload-mutation': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization', 'retained-serialized-payload-import', 'retained-serialized-payload-mutation']),
  'retained-serialized-payload-mutation-combined': Object.freeze(['handles', 'file-input', 'retained-file-input', 'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation', 'retained-call-argument-serialization', 'retained-serialized-payload-import', 'retained-serialized-payload-mutation', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'file-input': Object.freeze(['file-input']),
  'file-input-concurrent-close-drag-frame-extra-headers': Object.freeze(['file-input', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'drag-runtime': Object.freeze(['drag-runtime']),
  'frame-navigation': Object.freeze(['frame-navigation']),
  'drag-frame-navigation': Object.freeze(['drag-runtime', 'frame-navigation']),
  'extra-headers': Object.freeze(['extra-headers']),
  'drag-extra-headers': Object.freeze(['drag-runtime', 'extra-headers']),
  'frame-extra-headers': Object.freeze(['frame-navigation', 'extra-headers']),
  'drag-frame-extra-headers': Object.freeze(['drag-runtime', 'frame-navigation', 'extra-headers']),
  'concurrent-close': Object.freeze(['concurrent-close']),
  'concurrent-close-drag-frame-extra-headers': Object.freeze(['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
});
function nativeFeatures(profile) {
  if (typeof profile !== 'string' || !Object.hasOwn(profiles, profile)) throw new TypeError('Unknown PWRIGHT_NATIVE_PROFILE');
  return profiles[profile];
}
function resolveNativeArtifact(profile) {
  nativeFeatures(profile);
  return Object.freeze({
    profile,
    path: profile === 'default'
      ? join(__dirname, '../native/pwright.node')
      : join(__dirname, '../native', profile, 'pwright.node'),
  });
}

// Capture once for this process, shared by the loader and artifact reporters.
const nativeArtifact = resolveNativeArtifact(process.env.PWRIGHT_NATIVE_PROFILE ?? 'default');
function verifyNativeArtifact(binary, selection) {
  // Earlier binaries may predate an export. Absence means that capability is
  // not selected; every requested capability must be proved by its binary.
  const features = nativeFeatures(selection.profile);
  const drag = typeof binary.dragRuntimeEnabled === 'function' && binary.dragRuntimeEnabled();
  const frame = typeof binary.frameNavigationEnabled === 'function' && binary.frameNavigationEnabled();
  const headers = typeof binary.extraHeadersEnabled === 'function' && binary.extraHeadersEnabled();
  const close = typeof binary.concurrentCloseEnabled === 'function' && binary.concurrentCloseEnabled();
  const upload = typeof binary.fileInputEnabled === 'function' && binary.fileInputEnabled();
  const handles = typeof binary.handlesEnabled === 'function' && binary.handlesEnabled();
  const retained = typeof binary.retainedFileInputEnabled === 'function' && binary.retainedFileInputEnabled();
  const marking = typeof binary.retainedTargetMarkingEnabled === 'function' && binary.retainedTargetMarkingEnabled();
  const prevalidation = typeof binary.retainedTargetPrevalidationEnabled === 'function' && binary.retainedTargetPrevalidationEnabled();
  const instrumentation = typeof binary.retainedInputInstrumentationEnabled === 'function' && binary.retainedInputInstrumentationEnabled();
  const serialization = typeof binary.retainedCallArgumentSerializationEnabled === 'function' && binary.retainedCallArgumentSerializationEnabled();
  const payloadImport = typeof binary.retainedSerializedPayloadImportEnabled === 'function' && binary.retainedSerializedPayloadImportEnabled();
  const payloadMutation = typeof binary.retainedSerializedPayloadMutationEnabled === 'function' && binary.retainedSerializedPayloadMutationEnabled();
  if (drag !== features.includes('drag-runtime') || frame !== features.includes('frame-navigation') || headers !== features.includes('extra-headers') || close !== features.includes('concurrent-close') || upload !== features.includes('file-input') || handles !== features.includes('handles') || retained !== features.includes('retained-file-input') || marking !== features.includes('retained-target-marking') || prevalidation !== features.includes('retained-target-prevalidation') || instrumentation !== features.includes('retained-input-instrumentation') || serialization !== features.includes('retained-call-argument-serialization') || payloadImport !== features.includes('retained-serialized-payload-import') || payloadMutation !== features.includes('retained-serialized-payload-mutation'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
}
module.exports = { nativeArtifact, resolveNativeArtifact, verifyNativeArtifact, nativeFeatures };
