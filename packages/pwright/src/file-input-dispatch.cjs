'use strict';

const { setImmediate: schedule } = require('node:timers');
const { continueAfterChannel } = require('./file-input-preflight.cjs');
const { serverValidationPorts } = require('./file-input-channel.cjs');
const { asyncMaterialize } = require('./file-input-materialization.cjs');
const { createProgress } = require('./file-input-progress.cjs');
const { projectionPorts } = require('./file-input-projection.cjs');
const { beforeInputAction, emptyInputInstrumentation } = require('./file-input-instrumentation.cjs');
const { serializeFileInputCallArgument } = require('./file-input-call-argument.cjs');

// Empty-instrumentation timing only, not a hook implementation. The server
// always awaits onBeforeCall even with no listeners. Never await params here:
// doing so would assimilate an inherited then that the validator leaves raw.
async function beforeCallCheckpoint() {}

async function serverPayloads(session, preparation, markTarget, projectTarget, prevalidateTarget,
  instrumentTarget, serializePayloads, importPayloads, mutatePayloads, instrumentation) {
  let progress;
  try {
    session.handleCheckFileInputDispatch(preparation);
    preparation.validateServerParams(serverValidationPorts);
    const selected = typeof session.handleBeginFileInputCall === 'function';
    const callId = selected ? session.handleBeginFileInputCall(preparation) : undefined;
    await beforeCallCheckpoint();
    if (selected) {
      const timeout = session.handleStartFileInputDeadline(preparation);
      progress = createProgress(preparation, timeout);
    } else {
      session.handleCheckFileInputDispatch(preparation);
    }
    const materialization = asyncMaterialize(preparation);
    const carrier = await (progress ? progress.race(materialization) : materialization);
    preparation.finishMaterialization(carrier);
    if (markTarget) {
      if (!progress || typeof session.handleMarkFileInputTarget !== 'function')
        throw new Error('Retained target marking requires its selected native profile');
      await progress.race(session.handleMarkFileInputTarget(preparation));
    }
    if (projectTarget) {
      if (!markTarget || typeof preparation.projectAfterMarking !== 'function')
        throw new Error('Retained carrier projection requires marked-target preparation');
      preparation.projectAfterMarking(projectionPorts);
    }
    if (prevalidateTarget) {
      if (!projectTarget || typeof session.handlePrevalidateFileInputTarget !== 'function')
        throw new Error('Retained target prevalidation requires projected-target preparation');
      await progress.race(session.handlePrevalidateFileInputTarget(preparation));
    }
    if (instrumentTarget) {
      if (!prevalidateTarget || !progress || typeof session.handleBeginFileInputInstrumentation !== 'function' ||
          typeof session.handleFinishFileInputInstrumentation !== 'function')
        throw new Error('Retained input instrumentation requires prevalidated-target preparation');
      const receiver = session.handleBeginFileInputInstrumentation(preparation);
      await progress.race(beforeInputAction(instrumentation, progress, callId, receiver));
      session.handleFinishFileInputInstrumentation(preparation);
    }
    if (serializePayloads) {
      if (!instrumentTarget || typeof session.handleSerializeFileInputPayloads !== 'function')
        throw new Error('Retained call-argument serialization requires input instrumentation');
      session.handleSerializeFileInputPayloads(preparation, serializeFileInputCallArgument);
    }
    if (importPayloads) {
      if (!serializePayloads || typeof session.handleImportSerializedFileInputPayloads !== 'function')
        throw new Error('Retained serialized payload import requires call-argument serialization');
      session.handleImportSerializedFileInputPayloads(preparation);
    }
    if (mutatePayloads) {
      if (!importPayloads || !progress || typeof session.handleMutateSerializedFileInputPayloads !== 'function')
        throw new Error('Retained serialized-payload mutation requires imported payload ownership');
      await progress.race(session.handleMutateSerializedFileInputPayloads(preparation));
    }
    // Internal stop point only: no public API or Playwright error transport.
    // The selected mutation path terminally settles all native preparation
    // ownership before reaching this boundary.
    // The caller already owns the ticket. Never return it from an async body:
    // a user-installed ticket.then would be assimilated outside this guard.
  } catch (error) {
    preparation.abort();
    throw error;
  } finally {
    progress?.close();
  }
}

function prepareServerPayloads(session, preparation, markTarget = false, projectTarget = false,
  prevalidateTarget = false, instrumentTarget = false, serializePayloads = false, importPayloads = false,
  mutatePayloads = false, instrumentation = emptyInputInstrumentation) {
  return continueAfterChannel(preparation, ticket => new Promise((resolve, reject) => {
    try {
      schedule(() => {
        // Capture the async failure; no ignored rejection and no JS roots or
        // native task token are smuggled into a Rust worker while this waits.
        serverPayloads(session, ticket, markTarget, projectTarget, prevalidateTarget,
          instrumentTarget, serializePayloads, importPayloads, mutatePayloads, instrumentation).then(resolve, reject);
      });
    } catch (error) {
      ticket.abort();
      reject(error);
    }
  }));
}

function prepareMarkedTarget(session, preparation) {
  return prepareServerPayloads(session, preparation, true);
}

function prepareProjectedTarget(session, preparation) {
  return prepareServerPayloads(session, preparation, true, true);
}

function preparePrevalidatedTarget(session, preparation) {
  return prepareServerPayloads(session, preparation, true, true, true);
}

function prepareInstrumentedTarget(session, preparation, instrumentation = emptyInputInstrumentation) {
  return prepareServerPayloads(session, preparation, true, true, true, true, false, false, false, instrumentation);
}

function prepareSerializedTarget(session, preparation, instrumentation = emptyInputInstrumentation) {
  return prepareServerPayloads(session, preparation, true, true, true, true, true, false, false, instrumentation);
}

function prepareImportedTarget(session, preparation, instrumentation = emptyInputInstrumentation) {
  return prepareServerPayloads(session, preparation, true, true, true, true, true, true, false, instrumentation);
}

function prepareMutatedTarget(session, preparation, instrumentation = emptyInputInstrumentation) {
  return prepareServerPayloads(session, preparation, true, true, true, true, true, true, true, instrumentation);
}

module.exports = {
  prepareServerPayloads,
  prepareMarkedTarget,
  prepareProjectedTarget,
  preparePrevalidatedTarget,
  prepareInstrumentedTarget,
  prepareSerializedTarget,
  prepareImportedTarget,
  prepareMutatedTarget,
};
