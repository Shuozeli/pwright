'use strict';

const bindingValues = require('./binding-values.cjs');

module.exports = function pageBindings({ mapError, UnsupportedOperationError, createHandle }) {

  async function dispatch(page, frame, call) {
    const session = page._context._browser._session;
    const binding = page._bindings.get(call.name);
    let published = false;
    const acknowledge = async () => {
      if (published) return;
      published = true;
      await session.acknowledgePageBindingCall(call.callId);
    };
    let payload;
    if (!binding) {
      payload = { error: new Error(`Function "${call.name}" is not exposed`) };
      await acknowledge();
    } else {
      try {
        let args;
        if (binding.needsHandle) {
          const native = await session.takePageBindingHandle(call.callId).catch(() => null);
          const handle = native
            ? await createHandle(session, native, id => page._context._resolveHandleFrame(id))
            : null;
          args = [handle];
        } else {
          args = bindingValues.parseArguments(JSON.parse(call.argumentsJson));
        }
        const result = binding.callback({ context: page.context(), page, frame }, ...args);
        await acknowledge();
        payload = { result: await result };
      } catch (error) {
        await acknowledge();
        payload = { error };
      }
    }
    try {
      await session.deliverPageBindingResult(call.callId, payload);
    } catch {}
  }

  function watch(session, browserRef) {
    if (typeof session.nextPageBindingCall !== 'function') return;
    const next = () => {
      Promise.resolve().then(() => session.nextPageBindingCall()).then(call => {
        if (!call) return;
        const browser = browserRef.deref();
        if (!browser) {
          Promise.resolve(session.acknowledgePageBindingCall(call.callId)).catch(() => {});
          return;
        }
        const context = browser._contexts.get(call.contextId);
        const page = context?._pages.get(call.pageId);
        const frame = page?.frames().find(frame => frame._id === call.frameId);
        if (!page || !frame) {
          const payload = { error: new Error('Binding source frame has been closed') };
          session.deliverPageBindingResult(call.callId, payload).catch(() => {});
          Promise.resolve(session.acknowledgePageBindingCall(call.callId)).catch(() => {});
        } else {
          dispatch(page, frame, call).catch(error => browser._didWatchFail(error));
        }
        next();
      }, error => browserRef.deref()?._didWatchFail(error));
    };
    next();
  }

  async function expose(page, name, callback, api = 'page.exposeBinding', needsHandle = false) {
    const session = page._context._browser._session;
    if (typeof session.pageExposeBinding !== 'function')
      throw new UnsupportedOperationError(`${api} (select page-bindings native profile)`);
    if (page._bindings.has(name))
      throw new Error(`${api}: Function "${name}" has been already registered`);
    page._bindings.set(name, { callback, needsHandle });
    try {
      await session.pageExposeBinding(page._id, name, needsHandle);
    } catch (error) {
      page._bindings.delete(name);
      throw mapError(error, api);
    }
  }

  return { dispatch, expose, watch };
};
