import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import * as api from '@shuozeli/pwright';
import './browser-page-adapter.test.mjs';
import './cdp-session-adapter.test.mjs';
import './cdp-handshake-headers-adapter.test.mjs';
import './cdp-default-user-agent-adapter.test.mjs';
import './cdp-errors-adapter.test.mjs';

class NativeEnvironment {
  registerSelectorEngine() {}
  setTestIdAttribute() {}
}
class NativeSession {
  setPageConsoleListener() {}
  setContextConsoleListener() {}
  handleDialog() {}
  pageRoute() {}
  nextPageRouteCall() {}
  pageRouteContinue() {}
  newContextWithBypassCsp() {}
  newContextWithHasTouch() {}
  touchscreenTap() {}
  elementHandleTap() {}
  elementHandleDispatchEvent() {}
  pageGoBack() {}
  pageGoForward() {}
  pageSetViewportSize() {}
  nextRequestNotice() {}
  nextResponseNotice() {}
  nextNetworkNotice() {}
  networkNoticeFence() {}
  requestResponse() {}
  requestFailure() {}
  requestRedirectedFrom() {}
  responseFinished() {}
  pageUrlGlob() {}
}
class NativeSessionWithRetainedScroll extends NativeSession {
  elementHandleScrollIntoViewIfNeeded() {}
}
class NativeSessionWithRetainedWait extends NativeSessionWithRetainedScroll {
  elementHandleWaitForSelector() {}
}
class NativeSessionWithSelectOptionValue extends NativeSession {
  elementHandleSelectOptionValue() {}
}
class NativeSessionWithRetainedSelectOptionValue extends NativeSessionWithRetainedWait {
  elementHandleSelectOptionValue() {}
}
class NativeSessionWithRouteHeaderUrlOverrides extends NativeSession {
  pageRouteContinueWithHeaders() {}
  pageRouteFallbackWithUrl() {}
}
class NativeSessionWithUnroute extends NativeSession {
  pageUnroute() {}
}
class NativeSessionWithRouteFulfill extends NativeSession {
  pageRouteFulfill() {}
}
class NativeSessionWithRouteRegExp extends NativeSession {
  pageRouteRegExp() {}
}
class NativeSessionWithRouteRequestHistory extends NativeSession {
  responseRawHeaders() {}
}
class NativeSessionWithContextCookies extends NativeSession {
  contextAddCookies() {}
}
class NativeSessionWithLargeRoutePostData extends NativeSession {
  frameEvaluate() {}
}
class NativeSessionWithRouteHeaderUrlAndUnroute extends NativeSessionWithRouteHeaderUrlOverrides {
  pageUnroute() {}
}
class NativeSessionWithRouteHeaderUrlUnrouteCorsRegExp extends NativeSessionWithRouteHeaderUrlAndUnroute {
  pageRouteFulfill() {}
  pageRouteRegExp() {}
}
class NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistory extends NativeSessionWithRouteHeaderUrlUnrouteCorsRegExp {
  responseRawHeaders() {}
}
class NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistoryCookies extends NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistory {
  contextAddCookies() {}
}
class NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistoryCookiesLargePostData extends NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistoryCookies {
  frameEvaluate() {}
}
class NativeLocator {
  dispatchEvent() {}
  selectText() {}
  screenshot() {}
  waitForElement() {}
  waitForSelectorElement() {}
}
class NativeDialog {}
class NativeFrameTree {
  retiredMetadata() {}
  eventMetadata() {}
  finishDetached() {}
}
class NativeFrameNode {
  parentId() {}
}
const nativeIdentity = Object.freeze({
  NativeEnvironment,
  NativeSession,
  NativeLocator,
  NativeDialog,
  nativeAbiVersion: () => 1,
});
const nativeBinary = (features = {}) => ({ ...nativeIdentity, ...features });

test('CDP session profile requires the complete opt-in native lease, event batch, and watcher', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('cdp-session');
  class CdpNativeSession extends NativeSession {
    cdpSessionCreate() {}
    cdpSessionSend() {}
    cdpSessionClaim() {}
    cdpSessionDetach() {}
    nextCdpSessionEventGeneration() {}
    reportCallbackException() {}
  }
  class NativeCdpSessionLease {
    token() {}
    claim() {}
    abort() {}
  }
  class NativeCdpEventBatch {
    peek() {}
    ackNext() {}
    rollback() {}
  }
  const binary = nativeBinary({ NativeSession: CdpNativeSession, NativeCdpSessionLease, NativeCdpEventBatch });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['cdp-session']);
  assert.match(selected.path, /\/native\/cdp-session\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeCdpEventBatch: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class extends NativeSession {
    cdpSessionCreate() {}
    cdpSessionSend() {}
    cdpSessionClaim() {}
    cdpSessionDetach() {}
    nextCdpSessionEventGeneration() {}
  } }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
  assert.doesNotThrow(() => verifyNativeArtifact(nativeBinary({ NativeSession: class extends NativeSession {
    reportCallbackException() {}
  } }), resolveNativeArtifact('default')));
});

test('fresh Chromium launch has an isolated native profile and exact ownership port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('owned-launch');
  class OwnedEnvironment extends NativeEnvironment {
    launchChromium() {}
  }
  const binary = nativeBinary({
    NativeEnvironment: OwnedEnvironment,
    ownedLaunchEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('owned-launch');

  // Assert
  assert.deepEqual(features, ['owned-launch']);
  assert.match(selected.path, /\/native\/owned-launch\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(nativeBinary({ ownedLaunchEnabled: () => true }), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Playwright trace-directory launch is an additive fail-closed owned profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('playwright-trace');
  class TraceEnvironment extends NativeEnvironment {
    launchChromium() {}
    launchChromiumWithTracesDir() {}
  }
  const binary = nativeBinary({
    NativeEnvironment: TraceEnvironment,
    ownedLaunchEnabled: () => true,
    playwrightTraceDirEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('playwright-trace');

  // Assert
  assert.deepEqual(features, ['owned-launch', 'playwright-trace']);
  assert.match(selected.path, /\/native\/playwright-trace\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, playwrightTraceDirEnabled: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(nativeBinary({
    NativeEnvironment: OwnedEnvironmentWithoutTraceDir,
    ownedLaunchEnabled: () => true,
    playwrightTraceDirEnabled: () => true,
  }), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('owned-launch')), /profile mismatch/);
});

class OwnedEnvironmentWithoutTraceDir extends NativeEnvironment {
  launchChromium() {}
}

test('persistent-Context preflight profile requires its pure decision ports', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('persistent-context-preflight');
  const binary = nativeBinary({
    persistentContextPreflightEnabled: () => true,
    persistentContextCertificateError: () => null,
    persistentContextArgumentError: () => null,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['persistent-context-preflight']);
  assert.match(selected.path, /\/native\/persistent-context-preflight\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, persistentContextArgumentError: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Handle native selection requires its own binary marker and fixed path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const binary = nativeBinary({ handlesEnabled: () => true });
  // Act
  const selected = resolveNativeArtifact('handles');
  // Assert
  assert.deepEqual(nativeFeatures('handles'), ['handles']);
  assert.match(selected.path, /\/native\/handles\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeLocator: class {} }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeLocator: class {
    waitForElement() {}
  } }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({}, selected), /ABI mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Frame owner Element is an exact default-off Handle capability and conformance union', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('frame-element');
  const combined = resolveNativeArtifact('official-conformance-page-frame-element');
  const baseline = resolveNativeArtifact('official-conformance-page-check');
  const binary = nativeBinary({
    handlesEnabled: () => true,
    NativeSession: class extends NativeSession { frameElement() {} },
  });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['handles', 'frame-element']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(baseline.profile), 'frame-element']);
  assert.equal(nativeFeatures('default').includes('frame-element'), false);
  assert.equal(nativeFeatures(baseline.profile).includes('frame-element'), false);
  assert.match(combined.path, /\/native\/official-conformance-page-frame-element\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact(nativeBinary({ handlesEnabled: () => true }), standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('handles')), /profile mismatch/);
});

test('Frame events are a separate default-off Page-events capability', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('frame-events');
  const combined = resolveNativeArtifact('official-conformance-page-frame-events');
  const fullHierarchy = resolveNativeArtifact('official-conformance-page-frame-hierarchy');
  const baseline = resolveNativeArtifact('official-conformance-page-check');
  const binary = nativeBinary({ pageEventsEnabled: () => true, frameEventsEnabled: () => true });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['page-events', 'frame-events']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(baseline.profile), 'frame-events']);
  assert.deepEqual(nativeFeatures(fullHierarchy.profile), [...combinedFeatures, 'frame-detach-events', 'frame-element']);
  assert.equal(nativeFeatures('default').includes('frame-events'), false);
  assert.equal(nativeFeatures(baseline.profile).includes('frame-events'), false);
  assert.equal(nativeFeatures('official-conformance-page-goto-frame-detach').includes('frame-events'), false);
  assert.equal(nativeFeatures(combined.profile).includes('frame-detach-events'), false);
  assert.equal(nativeFeatures(combined.profile).includes('frame-element'), false);
  assert.equal(nativeFeatures('frame-events').includes('frame-detach-events'), false);
  assert.equal(nativeFeatures('official-conformance-page-frame-element').includes('frame-events'), false);
  assert.match(combined.path, /\/native\/official-conformance-page-frame-events\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact({ ...binary, frameEventsEnabled: () => false }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, baseline), /profile mismatch/);
});

test('ElementHandle DOM selection composes onto handles with its own marker and path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('element-handle-dom');
  const binary = nativeBinary({ handlesEnabled: () => true, elementHandleDomEnabled: () => true });
  // Act
  const features = nativeFeatures('element-handle-dom');
  // Assert
  assert.deepEqual(features, ['handles', 'element-handle-dom']);
  assert.match(selected.path, /\/native\/element-handle-dom\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleDomEnabled: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('handles')), /profile mismatch/);
});

test('retained ElementHandle scroll is a separate fail-closed Handle leaf', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('element-handle-scroll-into-view');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRetainedScroll,
    handlesEnabled: () => true,
    timeoutSettingsEnabled: () => true,
    elementHandleScrollIntoViewEnabled: () => true,
    elementHandleScrollTimeoutInheritanceEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['handles', 'timeout-settings', 'element-handle-scroll-into-view']);
  assert.equal(features.includes('element-handle-dom'), false);
  assert.match(selected.path, /\/native\/element-handle-scroll-into-view\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleScrollIntoViewEnabled: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleScrollTimeoutInheritanceEnabled: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleScrollTimeoutInheritanceEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('handles')), /profile mismatch/);
});

test('retained ElementHandle selector wait has an independent fail-closed profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('element-handle-wait-for-selector');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRetainedWait,
    handlesEnabled: () => true,
    timeoutSettingsEnabled: () => true,
    elementHandleWaitForSelectorEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['handles', 'timeout-settings', 'element-handle-wait-for-selector']);
  assert.equal(features.includes('element-handle-dom'), false);
  assert.match(selected.path, /\/native\/element-handle-wait-for-selector\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleWaitForSelectorEnabled: undefined }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithRetainedScroll }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleWaitForSelectorEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('handles')), /profile mismatch/);
});

test('retained ElementHandle value selection has a narrow fail-closed profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('element-handle-select-option-value');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithSelectOptionValue,
    handlesEnabled: () => true,
    timeoutSettingsEnabled: () => true,
    elementHandleSelectOptionValueEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['handles', 'timeout-settings', 'element-handle-select-option-value']);
  assert.equal(features.includes('element-handle-dom'), false);
  assert.match(selected.path, /\/native\/element-handle-select-option-value\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, elementHandleSelectOptionValueEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('element-handle-dom')), /profile mismatch/);
});

test('native profiles select distinct fixed paths without loading a binary', () => {
  // Arrange
  const { resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  // Act
  const base = resolveNativeArtifact('default');
  const selected = resolveNativeArtifact('drag-runtime');
  // Assert
  assert.match(base.path, /\/native\/pwright\.node$/);
  assert.match(selected.path, /\/native\/drag-runtime\/pwright\.node$/);
  assert.notEqual(base.path, selected.path);
  assert.equal(Object.isFrozen(selected), true);
});

test('page-check conformance profile preserves the prior feature closure but not its artifact path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  // Act
  const baseline = resolveNativeArtifact('official-conformance-page-bindings');
  const next = resolveNativeArtifact('official-conformance-page-check');
  // Assert
  assert.deepEqual(nativeFeatures(next.profile), nativeFeatures(baseline.profile));
  assert.notEqual(next.path, baseline.path);
  assert.match(next.path, /\/native\/official-conformance-page-check\/pwright\.node$/);
});

test('light selector conformance profile adds only the opt-in non-piercing engine', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const baseline = resolveNativeArtifact('official-conformance-page-check');
  const selected = resolveNativeArtifact('official-conformance-page-check-light-v1');

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, [...nativeFeatures(baseline.profile), 'light-selectors']);
  assert.equal(nativeFeatures(baseline.profile).includes('light-selectors'), false);
  assert.match(selected.path, /\/native\/official-conformance-page-check-light-v1\/pwright\.node$/);
  assert.throws(() => verifyNativeArtifact(nativeBinary({ lightSelectorsEnabled: () => true }), resolveNativeArtifact('default')), /profile mismatch/);
});

test('official CDP session profile adds Page and Browser sessions to the Page fixture closure', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const baseline = resolveNativeArtifact('official-conformance-page-bindings');
  const selected = resolveNativeArtifact('official-conformance-page-cdp-session');

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, [...nativeFeatures(baseline.profile), 'cdp-session', 'browser-cdp-session']);
  assert.ok(features.includes('init-scripts'));
  assert.notEqual(selected.path, baseline.path);
  assert.match(selected.path, /\/native\/official-conformance-page-cdp-session\/pwright\.node$/);
});

test('page-dispatch-event conformance profile has a distinct artifact and proves its retained Handle port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const previous = resolveNativeArtifact('official-conformance-page-check');
  const selected = resolveNativeArtifact('official-conformance-page-dispatch-event');
  const features = nativeFeatures(selected.profile);

  // Act
  const required = ['selector-registration', 'locator-dom-actions', 'element-handle-dom'];

  // Assert
  assert.deepEqual(features, nativeFeatures(previous.profile));
  assert.ok(required.every(feature => features.includes(feature)));
  assert.notEqual(selected.path, previous.path);
  assert.match(selected.path, /\/native\/official-conformance-page-dispatch-event\/pwright\.node$/);
  assert.equal(Object.isFrozen(selected), true);
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
});

test('page-goto burst profile adds only the 20-wait feature to the official Core union', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const official = resolveNativeArtifact('official-conformance-page-dispatch-event');
  const burst = resolveNativeArtifact('frame-navigation-burst-20');

  // Act
  const selected = resolveNativeArtifact('official-conformance-page-goto-burst-20');
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, [...nativeFeatures(official.profile), 'frame-navigation-burst-20']);
  assert.equal(new Set(features).size, features.length);
  assert.deepEqual(nativeFeatures(burst.profile), ['frame-navigation', 'frame-navigation-burst-20']);
  assert.match(selected.path, /\/native\/official-conformance-page-goto-burst-20\/pwright\.node$/);
  assert.notEqual(selected.path, official.path);
  assert.notEqual(selected.path, burst.path);
  assert.equal(Object.isFrozen(features), true);
  assert.equal(Object.isFrozen(selected), true);
});

test('route header and URL override profiles are separate exact feature unions', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('route-header-url-overrides');
  const combined = resolveNativeArtifact('official-conformance-page-route-header-url');
  const previous = resolveNativeArtifact('official-conformance-page-goto-burst-20');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRouteHeaderUrlOverrides,
    pageRoutesEnabled: () => true,
    routeHeaderUrlOverridesEnabled: () => true,
  });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['page-routes', 'route-header-url-overrides']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(previous.profile), 'route-header-url-overrides']);
  assert.notEqual(combined.path, previous.path);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact({ ...binary, routeHeaderUrlOverridesEnabled: undefined }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Page unroute profiles require only the exact opt-in registration removal port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('page-unroute');
  const combined = resolveNativeArtifact('official-conformance-page-route-header-url-unroute');
  const previous = resolveNativeArtifact('official-conformance-page-route-header-url');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithUnroute,
    pageRoutesEnabled: () => true,
    pageUnrouteEnabled: () => true,
  });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['page-routes', 'page-unroute']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(previous.profile), 'page-unroute']);
  assert.notEqual(combined.path, previous.path);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact({ ...binary, pageUnrouteEnabled: undefined }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Route.fulfill CORS profiles compose without enabling the default or previous route profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('route-fulfill-cors');
  const combined = resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors');
  const previous = resolveNativeArtifact('official-conformance-page-route-header-url-unroute');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRouteFulfill,
    pageRoutesEnabled: () => true,
    routeFulfillCorsEnabled: () => true,
  });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['page-routes', 'route-fulfill-cors']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(previous.profile), 'route-fulfill-cors']);
  assert.equal(new Set(combinedFeatures).size, combinedFeatures.length);
  assert.ok(standalone.path.endsWith('/native/route-fulfill-cors/pwright.node'));
  assert.ok(combined.path.endsWith('/native/official-conformance-page-route-header-url-unroute-cors/pwright.node'));
  assert.notEqual(combined.path, previous.path);
  assert.equal(nativeFeatures('default').includes('route-fulfill-cors'), false);
  assert.equal(nativeFeatures(previous.profile).includes('route-fulfill-cors'), false);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact({ ...binary, routeFulfillCorsEnabled: undefined }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('RegExp route profiles require their exact marker and native registration binding', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const standalone = resolveNativeArtifact('route-regexp');
  const combined = resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors-regexp');
  const previous = resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRouteRegExp,
    pageRoutesEnabled: () => true,
    routeRegExpEnabled: () => true,
  });

  // Act
  const standaloneFeatures = nativeFeatures(standalone.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(standaloneFeatures, ['page-routes', 'route-regexp']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(previous.profile), 'route-regexp']);
  assert.equal(new Set(combinedFeatures).size, combinedFeatures.length);
  assert.ok(standalone.path.endsWith('/native/route-regexp/pwright.node'));
  assert.ok(combined.path.endsWith('/native/official-conformance-page-route-header-url-unroute-cors-regexp/pwright.node'));
  assert.notEqual(combined.path, previous.path);
  assert.equal(nativeFeatures('default').includes('route-regexp'), false);
  assert.equal(nativeFeatures(previous.profile).includes('route-regexp'), false);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, standalone));
  assert.throws(() => verifyNativeArtifact({ ...binary, routeRegExpEnabled: undefined }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, standalone), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('route request history requires its exact marker and bounded response header port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('route-request-history');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithRouteRequestHistory,
    pageRoutesEnabled: () => true,
    routeRequestHistoryEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['page-routes', 'route-request-history']);
  assert.ok(selected.path.endsWith('/native/route-request-history/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, routeRequestHistoryEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('Context cookies select one independent Rust leaf and reject unmarked native bindings', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('context-cookies');
  const previous = resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors-regexp-history');
  const combined = resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithContextCookies,
    contextCookiesEnabled: () => true,
  });

  // Act
  const selectedFeatures = nativeFeatures(selected.profile);
  const combinedFeatures = nativeFeatures(combined.profile);

  // Assert
  assert.deepEqual(selectedFeatures, ['context-cookies']);
  assert.deepEqual(combinedFeatures, [...nativeFeatures(previous.profile), 'context-cookies']);
  assert.equal(nativeFeatures('default').includes('context-cookies'), false);
  assert.equal(nativeFeatures(previous.profile).includes('context-cookies'), false);
  assert.notEqual(combined.path, previous.path);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, contextCookiesEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('large Route POST profile selects only bounded Rust inputs and routing without network observation', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('route-large-post-data');
  const binary = nativeBinary({
    NativeSession: NativeSessionWithLargeRoutePostData,
    frameNavigationEnabled: () => true,
    pageRoutesEnabled: () => true,
    largeEvaluationInputsEnabled: () => true,
    largeRoutePostDataEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['frame-navigation', 'page-routes', 'large-evaluation-inputs', 'large-route-post-data']);
  assert.equal(features.includes('request-events'), false);
  assert.equal(features.includes('response-events'), false);
  assert.equal(nativeFeatures('default').includes('large-route-post-data'), false);
  assert.ok(selected.path.endsWith('/native/route-large-post-data/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, largeEvaluationInputsEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, largeRoutePostDataEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('direct-fill navigation commit remains an independently selectable native feature', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('fill-navigation-commit');
  const binary = nativeBinary({ fillNavigationCommitEnabled: () => true });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['fill-navigation-commit']);
  assert.ok(selected.path.endsWith('/native/fill-navigation-commit/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, fillNavigationCommitEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('select-option navigation commit remains an independently selectable native feature', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('select-option-navigation-commit');
  const binary = nativeBinary({ selectOptionNavigationCommitEnabled: () => true });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['select-option-navigation-commit']);
  assert.ok(selected.path.endsWith('/native/select-option-navigation-commit/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, selectOptionNavigationCommitEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('select-option timeout progress remains an independently selectable native feature', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('select-option-timeout-progress');
  const binary = nativeBinary({ selectOptionTimeoutProgressEnabled: () => true });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['select-option-timeout-progress']);
  assert.ok(selected.path.endsWith('/native/select-option-timeout-progress/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, selectOptionTimeoutProgressEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('React selector support remains an independently selectable native feature', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('react-selectors');
  const binary = nativeBinary({ reactSelectorsEnabled: () => true });

  // Act
  const features = nativeFeatures(selected.profile);

  // Assert
  assert.deepEqual(features, ['react-selectors']);
  assert.ok(selected.path.endsWith('/native/react-selectors/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, reactSelectorsEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('set-content profile requires only its explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('set-content');
  const binary = nativeBinary({ setContentEnabled: () => true });
  // Act
  const features = nativeFeatures('set-content');
  // Assert
  assert.deepEqual(features, ['set-content']);
  assert.ok(selected.path.endsWith('/native/set-content/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('history-navigation profile requires its marker and both native Page methods', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('history-navigation');
  const binary = nativeBinary({ historyNavigationEnabled: () => true });
  // Act
  const features = nativeFeatures('history-navigation');
  // Assert
  assert.deepEqual(features, ['history-navigation']);
  assert.ok(selected.path.endsWith('/native/history-navigation/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, historyNavigationEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class { pageGoBack() {} } }, selected), /profile mismatch/);
});

test('response-request profile composes navigation with bounded request history', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('response-request');
  const binary = nativeBinary({
    frameNavigationEnabled: () => true,
    responseRequestEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('response-request');

  // Assert
  assert.deepEqual(features, ['frame-navigation', 'response-request']);
  assert.ok(selected.path.endsWith('/native/response-request/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, responseRequestEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('frame-navigation')), /profile mismatch/);
});

test('frame-navigation burst profile rejects a stale ordinary navigation binary', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('frame-navigation-burst-20');
  const ordinary = resolveNativeArtifact('frame-navigation');
  const burstBinary = nativeBinary({
    frameNavigationEnabled: () => true,
    frameNavigationBurst20Enabled: () => true,
  });
  const ordinaryBinary = nativeBinary({ frameNavigationEnabled: () => true });

  // Act / Assert
  assert.deepEqual(nativeFeatures(selected.profile), ['frame-navigation', 'frame-navigation-burst-20']);
  assert.match(selected.path, /\/native\/frame-navigation-burst-20\/pwright\.node$/);
  assert.notEqual(selected.path, ordinary.path);
  assert.doesNotThrow(() => verifyNativeArtifact(burstBinary, selected));
  assert.throws(() => verifyNativeArtifact(ordinaryBinary, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...burstBinary, frameNavigationBurst20Enabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...burstBinary, frameNavigationEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(burstBinary, ordinary), /profile mismatch/);
});

test('request-events profile remains independently loadable without JSON decoding', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('request-events');
  const binary = nativeBinary({ requestEventsEnabled: () => true });

  // Act
  const features = nativeFeatures('request-events');

  // Assert
  assert.deepEqual(features, ['request-events']);
  assert.ok(selected.path.endsWith('/native/request-events/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, requestEventsEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, requestPostDataJsonEnabled: () => true }, selected), /profile mismatch/);
});

test('response-events profile remains independently loadable', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('response-events');
  const binary = nativeBinary({ responseEventsEnabled: () => true });

  // Act
  const features = nativeFeatures('response-events');

  // Assert
  assert.deepEqual(features, ['response-events']);
  assert.ok(selected.path.endsWith('/native/response-events/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, responseEventsEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
});

test('response-finished profile adds only terminal response ownership', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('response-finished');
  const binary = nativeBinary({
    responseEventsEnabled: () => true,
    responseFinishedEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('response-finished');

  // Assert
  assert.deepEqual(features, ['response-events', 'response-finished']);
  assert.ok(selected.path.endsWith('/native/response-finished/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, responseFinishedEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('response-events')), /profile mismatch/);
});

test('request post-data profile composes bounded notices and Rust decoding', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('request-post-data-json');
  const binary = nativeBinary({
    requestEventsEnabled: () => true,
    requestPostDataJsonEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('request-post-data-json');

  // Assert
  assert.deepEqual(features, ['request-events', 'request-post-data-json']);
  assert.ok(selected.path.endsWith('/native/request-post-data-json/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, requestEventsEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, requestPostDataJsonEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
});

test('device-emulation profile requires its marker and native Context method', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('device-emulation');
  const binary = nativeBinary({ deviceEmulationEnabled: () => true });

  // Act
  const features = nativeFeatures('device-emulation');

  // Assert
  assert.deepEqual(features, ['device-emulation']);
  assert.ok(selected.path.endsWith('/native/device-emulation/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, deviceEmulationEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
});

test('touchscreen-input profile composes touch Context creation with its native Page port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('touchscreen-input');
  const binary = nativeBinary({
    deviceEmulationEnabled: () => true,
    touchscreenInputEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('touchscreen-input');

  // Assert
  assert.deepEqual(features, ['device-emulation', 'touchscreen-input']);
  assert.ok(selected.path.endsWith('/native/touchscreen-input/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({ ...binary, deviceEmulationEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, touchscreenInputEnabled: () => false }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
});

test('Page-event profiles keep lifecycle delivery independently selectable', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const cases = [
    ['page-events', ['page-events']],
    ['frame-events', ['page-events', 'frame-events']],
    ['set-content-page-events', ['set-content', 'page-events']],
    ['set-content-page-events-console', ['set-content', 'page-events', 'console-events']],
    ['set-content-page-events-console-bindings', ['set-content', 'page-events', 'console-events', 'page-bindings', 'handles']],
    ['official-conformance-page-bindings', ['set-content', 'page-events', 'page-dialogs', 'page-routes', 'console-events', 'page-bindings', 'handles', 'element-handle-dom', 'file-input', 'frame-navigation', 'history-navigation', 'init-scripts', 'timeout-settings', 'wait-for-function', 'wait-for-url', 'wait-for-timeout', 'large-evaluation-results', 'emulate-media', 'device-emulation', 'touchscreen-input', 'page-viewport', 'request-gc', 'response-body', 'response-request', 'request-events', 'response-events', 'response-finished', 'request-frame', 'request-post-data-json', 'locator-bounding-box', 'locator-highlight', 'locator-screenshot', 'locator-dom-actions', 'extra-headers', 'selector-registration', 'bypass-csp', 'api-request', 'context-api-request']],
    ['official-conformance-page-frame-events', [...nativeFeatures('official-conformance-page-bindings'), 'frame-events']],
    ['official-conformance-page-frame-hierarchy', [...nativeFeatures('official-conformance-page-frame-events'), 'frame-detach-events', 'frame-element']],
    ['official-conformance-page-dispatch-event', nativeFeatures('official-conformance-page-bindings')],
    ['official-conformance-page-goto-burst-20', [...nativeFeatures('official-conformance-page-dispatch-event'), 'frame-navigation-burst-20']],
    ['official-conformance-page-goto-frame-detach', [...nativeFeatures('official-conformance-page-goto-burst-20'), 'frame-detach-events']],
    ['official-conformance-page-route-header-url', [...nativeFeatures('official-conformance-page-goto-burst-20'), 'route-header-url-overrides']],
    ['official-conformance-page-route-header-url-unroute', [...nativeFeatures('official-conformance-page-route-header-url'), 'page-unroute']],
    ['official-conformance-page-route-header-url-unroute-cors-regexp', [...nativeFeatures('official-conformance-page-route-header-url-unroute-cors'), 'route-regexp']],
    ['official-conformance-page-route-header-url-unroute-cors-regexp-history', [...nativeFeatures('official-conformance-page-route-header-url-unroute-cors-regexp'), 'route-request-history']],
    ['official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies', [...nativeFeatures('official-conformance-page-route-header-url-unroute-cors-regexp-history'), 'context-cookies']],
    ['official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data', [...nativeFeatures('official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies'), 'large-evaluation-inputs', 'large-route-post-data']],
    ['official-conformance-page-fill-navigation-commit', [...nativeFeatures('official-conformance-page-route-header-url-unroute'), 'fill-navigation-commit']],
    ['official-conformance-page-select-option-timeout-progress', [...nativeFeatures('official-conformance-page-bindings'), 'select-option-timeout-progress']],
    ['official-conformance-page-select-option-navigation-commit', [...nativeFeatures('official-conformance-page-select-option-timeout-progress'), 'select-option-navigation-commit']],
    ['official-conformance-page-react-selectors', [...nativeFeatures('official-conformance-page-bindings'), 'react-selectors']],
    ['official-conformance-page-retained-scroll', [...nativeFeatures('official-conformance-page-bindings'), 'element-handle-scroll-into-view']],
    ['official-conformance-page-retained-wait', [...nativeFeatures('official-conformance-page-retained-scroll'), 'element-handle-wait-for-selector']],
    ['official-conformance-page-retained-select-option-value', [...nativeFeatures('official-conformance-page-retained-wait'), 'element-handle-select-option-value']],
  ];
  for (const [profile, expected] of cases) {
    const binary = nativeBinary({
      NativeSession: expected.includes('frame-element') ? class extends NativeSession { frameElement() {} }
        : expected.includes('element-handle-select-option-value') ? NativeSessionWithRetainedSelectOptionValue
        : expected.includes('element-handle-wait-for-selector') ? NativeSessionWithRetainedWait
        : expected.includes('element-handle-scroll-into-view') ? NativeSessionWithRetainedScroll
        : expected.includes('large-route-post-data') ? NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistoryCookiesLargePostData
        : expected.includes('context-cookies') ? NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistoryCookies
        : expected.includes('route-request-history') ? NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistory
        : expected.includes('route-regexp') ? NativeSessionWithRouteHeaderUrlUnrouteCorsRegExp
        : expected.includes('page-unroute')
        ? expected.includes('route-header-url-overrides')
          ? NativeSessionWithRouteHeaderUrlAndUnroute : NativeSessionWithUnroute
        : expected.includes('route-header-url-overrides') ? NativeSessionWithRouteHeaderUrlOverrides : NativeSession,
      pageEventsEnabled: () => true,
      frameEventsEnabled: () => expected.includes('frame-events'),
      frameDetachEventsEnabled: () => expected.includes('frame-detach-events'),
      NativeFrameTree: expected.includes('frame-detach-events') ? NativeFrameTree : undefined,
      NativeFrameNode: expected.includes('frame-detach-events') ? NativeFrameNode : undefined,
      pageDialogsEnabled: () => expected.includes('page-dialogs'),
      pageRoutesEnabled: () => expected.includes('page-routes'),
      routeHeaderUrlOverridesEnabled: () => expected.includes('route-header-url-overrides'),
      routeFulfillCorsEnabled: () => expected.includes('route-fulfill-cors'),
      routeRegExpEnabled: () => expected.includes('route-regexp'),
      routeRequestHistoryEnabled: () => expected.includes('route-request-history'),
      pageUnrouteEnabled: () => expected.includes('page-unroute'),
      fillNavigationCommitEnabled: () => expected.includes('fill-navigation-commit'),
      selectOptionNavigationCommitEnabled: () => expected.includes('select-option-navigation-commit'),
      selectOptionTimeoutProgressEnabled: () => expected.includes('select-option-timeout-progress'),
      reactSelectorsEnabled: () => expected.includes('react-selectors'),
      consoleEventsEnabled: () => expected.includes('console-events'),
      pageBindingsEnabled: () => expected.includes('page-bindings'),
      initScriptsEnabled: () => expected.includes('init-scripts'),
      waitForFunctionEnabled: () => expected.includes('wait-for-function'),
      waitForUrlEnabled: () => expected.includes('wait-for-url'),
      waitForTimeoutEnabled: () => expected.includes('wait-for-timeout'),
      timeoutSettingsEnabled: () => expected.includes('timeout-settings'),
      emulateMediaEnabled: () => expected.includes('emulate-media'),
      deviceEmulationEnabled: () => expected.includes('device-emulation'),
      touchscreenInputEnabled: () => expected.includes('touchscreen-input'),
      pageViewportEnabled: () => expected.includes('page-viewport'),
      requestGcEnabled: () => expected.includes('request-gc'),
      responseBodyEnabled: () => expected.includes('response-body'),
      responseRequestEnabled: () => expected.includes('response-request'),
      requestEventsEnabled: () => expected.includes('request-events'),
      responseEventsEnabled: () => expected.includes('response-events'),
      responseFinishedEnabled: () => expected.includes('response-finished'),
      requestFrameEnabled: () => expected.includes('request-frame'),
      requestPostDataJsonEnabled: () => expected.includes('request-post-data-json'),
      locatorBoundingBoxEnabled: () => expected.includes('locator-bounding-box'),
      locatorHighlightEnabled: () => expected.includes('locator-highlight'),
      locatorScreenshotEnabled: () => expected.includes('locator-screenshot'),
      locatorDomActionsEnabled: () => expected.includes('locator-dom-actions'),
      largeEvaluationResultsEnabled: () => expected.includes('large-evaluation-results'),
      frameNavigationEnabled: () => expected.includes('frame-navigation'),
      frameNavigationBurst20Enabled: () => expected.includes('frame-navigation-burst-20'),
      historyNavigationEnabled: () => expected.includes('history-navigation'),
      handlesEnabled: () => expected.includes('handles'),
      elementHandleDomEnabled: () => expected.includes('element-handle-dom'),
      elementHandleScrollIntoViewEnabled: () => expected.includes('element-handle-scroll-into-view'),
      elementHandleScrollTimeoutInheritanceEnabled: () => expected.includes('element-handle-scroll-into-view'),
      elementHandleWaitForSelectorEnabled: () => expected.includes('element-handle-wait-for-selector'),
      elementHandleSelectOptionValueEnabled: () => expected.includes('element-handle-select-option-value'),
      fileInputEnabled: () => expected.includes('file-input'),
      setContentEnabled: () => expected.includes('set-content'),
      extraHeadersEnabled: () => expected.includes('extra-headers'),
      selectorRegistrationEnabled: () => expected.includes('selector-registration'),
      bypassCspEnabled: () => expected.includes('bypass-csp'),
      apiRequestEnabled: () => expected.includes('api-request'),
      apiRequestClientCertificatesEnabled: () => expected.includes('api-request-client-certificates'),
      apiRequestHttpProxyEnabled: () => expected.includes('api-request-http-proxy'),
      apiRequestProxyBypassEnabled: () => expected.includes('api-request-proxy-bypass'),
      contextApiRequestEnabled: () => expected.includes('context-api-request'),
      contextCookiesEnabled: () => expected.includes('context-cookies'),
      largeEvaluationInputsEnabled: () => expected.includes('large-evaluation-inputs'),
      largeRoutePostDataEnabled: () => expected.includes('large-route-post-data'),
      contextProxyEnabled: () => expected.includes('context-proxy'),
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    // Assert
    assert.deepEqual(nativeFeatures(profile), expected);
    if (expected.includes('large-route-post-data')) {
      assert.equal(nativeFeatures('default').includes('large-route-post-data'), false);
      assert.throws(() => verifyNativeArtifact({ ...binary, largeEvaluationInputsEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, largeRoutePostDataEnabled: () => false }, selected), /profile mismatch/);
    }
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, pageEventsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('frame-events')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, frameEventsEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-bindings')), /profile mismatch/);
    }
    if (expected.includes('frame-element')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
      assert.equal(nativeFeatures('official-conformance-page-frame-events').includes('frame-element'), false);
    }
    if (expected.includes('page-dialogs')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, pageDialogsEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeDialog: undefined }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('page-routes')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, pageRoutesEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('bypass-csp')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, bypassCspEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('console-events'))
      assert.throws(() => verifyNativeArtifact({ ...binary, consoleEventsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('console-events'))
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    if (expected.includes('page-bindings'))
      assert.throws(() => verifyNativeArtifact({ ...binary, pageBindingsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('init-scripts'))
      assert.throws(() => verifyNativeArtifact({ ...binary, initScriptsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('wait-for-function'))
      assert.throws(() => verifyNativeArtifact({ ...binary, waitForFunctionEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('wait-for-url'))
      assert.throws(() => verifyNativeArtifact({ ...binary, waitForUrlEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('wait-for-timeout'))
      assert.throws(() => verifyNativeArtifact({ ...binary, waitForTimeoutEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('timeout-settings'))
      assert.throws(() => verifyNativeArtifact({ ...binary, timeoutSettingsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('emulate-media'))
      assert.throws(() => verifyNativeArtifact({ ...binary, emulateMediaEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('device-emulation')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, deviceEmulationEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('touchscreen-input')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, touchscreenInputEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('touchscreen-input') && expected.includes('element-handle-dom')) {
      class NativeSessionWithoutElementHandleTap extends NativeSession {}
      NativeSessionWithoutElementHandleTap.prototype.elementHandleTap = undefined;
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithoutElementHandleTap }, selected), /profile mismatch/);
    }
    if (['official-conformance-page-dispatch-event', 'official-conformance-page-goto-burst-20', 'official-conformance-page-goto-frame-detach', 'official-conformance-page-route-header-url', 'official-conformance-page-route-header-url-unroute', 'official-conformance-page-route-header-url-unroute-cors-regexp', 'official-conformance-page-route-header-url-unroute-cors-regexp-history', 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies', 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data', 'official-conformance-page-fill-navigation-commit', 'official-conformance-page-select-option-timeout-progress', 'official-conformance-page-select-option-navigation-commit', 'official-conformance-page-react-selectors', 'official-conformance-page-retained-scroll', 'official-conformance-page-retained-wait'].includes(profile)) {
      class NativeSessionWithoutElementHandleDispatchEvent extends NativeSession {}
      NativeSessionWithoutElementHandleDispatchEvent.prototype.elementHandleDispatchEvent = undefined;
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithoutElementHandleDispatchEvent }, selected), /profile mismatch/);
    }
    if (expected.includes('frame-navigation-burst-20')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, frameNavigationBurst20Enabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-dispatch-event')), /profile mismatch/);
    }
    if (expected.includes('frame-detach-events')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, frameDetachEventsEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeFrameNode: undefined }, selected), /profile mismatch/);
      class NativeFrameTreeWithoutFinish extends NativeFrameTree {}
      NativeFrameTreeWithoutFinish.prototype.finishDetached = undefined;
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeFrameTree: NativeFrameTreeWithoutFinish }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-goto-burst-20')), /profile mismatch/);
    }
    if (expected.includes('route-header-url-overrides')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, routeHeaderUrlOverridesEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-goto-burst-20')), /profile mismatch/);
    }
    if (expected.includes('page-unroute')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, pageUnrouteEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithRouteHeaderUrlOverrides }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-route-header-url')), /profile mismatch/);
    }
    if (expected.includes('route-regexp')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, routeRegExpEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithRouteHeaderUrlAndUnroute }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors')), /profile mismatch/);
    }
    if (expected.includes('route-request-history')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, routeRequestHistoryEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithRouteHeaderUrlUnrouteCorsRegExp }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors-regexp')), /profile mismatch/);
    }
    if (expected.includes('context-cookies')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, contextCookiesEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: NativeSessionWithRouteHeaderUrlUnrouteCorsRegExpHistory }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-route-header-url-unroute-cors-regexp-history')), /profile mismatch/);
    }
    if (expected.includes('fill-navigation-commit')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, fillNavigationCommitEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-route-header-url-unroute')), /profile mismatch/);
    }
    if (expected.includes('select-option-timeout-progress')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, selectOptionTimeoutProgressEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-bindings')), /profile mismatch/);
    }
    if (expected.includes('select-option-navigation-commit')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, selectOptionNavigationCommitEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-select-option-timeout-progress')), /profile mismatch/);
    }
    if (expected.includes('react-selectors')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, reactSelectorsEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('official-conformance-page-bindings')), /profile mismatch/);
    }
    if (expected.includes('page-viewport')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, pageViewportEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeSession: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('request-gc'))
      assert.throws(() => verifyNativeArtifact({ ...binary, requestGcEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('response-body'))
      assert.throws(() => verifyNativeArtifact({ ...binary, responseBodyEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('response-request'))
      assert.throws(() => verifyNativeArtifact({ ...binary, responseRequestEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('request-events'))
      assert.throws(() => verifyNativeArtifact({ ...binary, requestEventsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('response-events'))
      assert.throws(() => verifyNativeArtifact({ ...binary, responseEventsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('request-post-data-json'))
      assert.throws(() => verifyNativeArtifact({ ...binary, requestPostDataJsonEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('locator-bounding-box'))
      assert.throws(() => verifyNativeArtifact({ ...binary, locatorBoundingBoxEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('locator-highlight'))
      assert.throws(() => verifyNativeArtifact({ ...binary, locatorHighlightEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('locator-screenshot')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, locatorScreenshotEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeLocator: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('locator-dom-actions'))
      assert.throws(() => verifyNativeArtifact({ ...binary, locatorDomActionsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('large-evaluation-results'))
      assert.throws(() => verifyNativeArtifact({ ...binary, largeEvaluationResultsEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('handles'))
      assert.throws(() => verifyNativeArtifact({ ...binary, handlesEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('extra-headers'))
      assert.throws(() => verifyNativeArtifact({ ...binary, extraHeadersEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('selector-registration')) {
      assert.throws(() => verifyNativeArtifact({ ...binary, selectorRegistrationEnabled: () => false }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, NativeEnvironment: class {} }, selected), /profile mismatch/);
    }
    if (expected.includes('api-request'))
      assert.throws(() => verifyNativeArtifact({ ...binary, apiRequestEnabled: () => false }, selected), /profile mismatch/);
    if (expected.includes('context-api-request'))
      assert.throws(() => verifyNativeArtifact({ ...binary, contextApiRequestEnabled: () => false }, selected), /profile mismatch/);
  }
});

test('selector-registration profile proves its isolated Rust registry ports', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('selector-registration');
  const binary = nativeBinary({ selectorRegistrationEnabled: () => true });

  // Act
  const features = nativeFeatures('selector-registration');

  // Assert
  assert.deepEqual(features, ['selector-registration']);
  assert.ok(selected.path.endsWith('/native/selector-registration/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeEnvironment: class {} }, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('CSP bypass profile requires only its Rust policy owner and native creation port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('bypass-csp');
  const binary = nativeBinary({ bypassCspEnabled: () => true });

  // Act
  const features = nativeFeatures('bypass-csp');

  // Assert
  assert.deepEqual(features, ['bypass-csp']);
  assert.ok(selected.path.endsWith('/native/bypass-csp/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
});

test('API request profile requires only its explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('api-request');
  const binary = nativeBinary({ apiRequestEnabled: () => true });
  // Act
  const features = nativeFeatures('api-request');
  // Assert
  assert.deepEqual(features, ['api-request']);
  assert.ok(selected.path.endsWith('/native/api-request/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact(nativeBinary(), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('API request client-certificate profile extends only the request profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('api-request-client-certificates');
  const binary = nativeBinary({
    apiRequestEnabled: () => true,
    apiRequestClientCertificatesEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('api-request-client-certificates');

  // Assert
  assert.deepEqual(features, ['api-request', 'api-request-client-certificates']);
  assert.ok(selected.path.endsWith('/native/api-request-client-certificates/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(
    () => verifyNativeArtifact({ ...binary, apiRequestClientCertificatesEnabled: () => false }, selected),
    /profile mismatch/,
  );
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('api-request')), /profile mismatch/);
});

test('API request HTTP-proxy profile extends only the request profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('api-request-http-proxy');
  const binary = nativeBinary({
    apiRequestEnabled: () => true,
    apiRequestHttpProxyEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('api-request-http-proxy');

  // Assert
  assert.deepEqual(features, ['api-request', 'api-request-http-proxy']);
  assert.ok(selected.path.endsWith('/native/api-request-http-proxy/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(
    () => verifyNativeArtifact({ ...binary, apiRequestHttpProxyEnabled: () => false }, selected),
    /profile mismatch/,
  );
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('api-request')), /profile mismatch/);
});

test('API request SOCKS5-proxy profile extends only the request profile', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('api-request-socks5-proxy');
  const binary = nativeBinary({
    apiRequestEnabled: () => true,
    apiRequestSocks5ProxyEnabled: () => true,
  });

  // Act
  const features = nativeFeatures('api-request-socks5-proxy');

  // Assert
  assert.deepEqual(features, ['api-request', 'api-request-socks5-proxy']);
  assert.ok(selected.path.endsWith('/native/api-request-socks5-proxy/pwright.node'));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(
    () => verifyNativeArtifact({ ...binary, apiRequestSocks5ProxyEnabled: () => false }, selected),
    /profile mismatch/,
  );
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('api-request')), /profile mismatch/);
});

test('API request proxy-bypass profiles are exact connector unions', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const cases = [
    ['api-request-http-proxy-bypass', 'api-request-http-proxy', 'apiRequestHttpProxyEnabled'],
    ['api-request-socks5-proxy-bypass', 'api-request-socks5-proxy', 'apiRequestSocks5ProxyEnabled'],
  ];

  // Act
  const observations = cases.map(([profile, proxyFeature, proxyMarker]) => {
    const selected = resolveNativeArtifact(profile);
    const expected = ['api-request', proxyFeature, 'api-request-proxy-bypass'];
    const binary = nativeBinary({
      apiRequestEnabled: () => true,
      apiRequestProxyBypassEnabled: () => true,
      [proxyMarker]: () => true,
    });
    return { profile, expected, proxyMarker, selected, binary };
  });

  // Assert
  for (const { profile, expected, proxyMarker, selected, binary } of observations) {
    assert.deepEqual(nativeFeatures(profile), expected);
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(
      () => verifyNativeArtifact({ ...binary, apiRequestProxyBypassEnabled: () => false }, selected),
      /profile mismatch/,
    );
    assert.throws(
      () => verifyNativeArtifact({ ...binary, [proxyMarker]: () => false }, selected),
      /profile mismatch/,
    );
  }
});

test('API request certificate-proxy profiles are exact feature unions', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const cases = [
    [
      'api-request-client-certificates-http-proxy',
      ['api-request', 'api-request-client-certificates', 'api-request-http-proxy'],
      'apiRequestHttpProxyEnabled',
    ],
    [
      'api-request-client-certificates-socks5-proxy',
      ['api-request', 'api-request-client-certificates', 'api-request-socks5-proxy'],
      'apiRequestSocks5ProxyEnabled',
    ],
  ];

  // Act
  const observations = cases.map(([profile, expected, proxyMarker]) => {
    const selected = resolveNativeArtifact(profile);
    const binary = nativeBinary({
      apiRequestEnabled: () => true,
      apiRequestClientCertificatesEnabled: () => true,
      [proxyMarker]: () => true,
    });
    return { profile, expected, proxyMarker, selected, binary };
  });

  // Assert
  for (const { profile, expected, proxyMarker, selected, binary } of observations) {
    assert.deepEqual(nativeFeatures(profile), expected);
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(
      () => verifyNativeArtifact({ ...binary, [proxyMarker]: () => false }, selected),
      /profile mismatch/,
    );
    assert.throws(
      () => verifyNativeArtifact({ ...binary, apiRequestClientCertificatesEnabled: () => false }, selected),
      /profile mismatch/,
    );
  }
});

test('BrowserContext request proxy profile is an explicit capability union', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const profile = 'context-api-request-proxy';
  const selected = resolveNativeArtifact(profile);
  const expected = [
    'api-request', 'context-api-request', 'device-emulation', 'context-proxy',
    'api-request-http-proxy', 'api-request-socks5-proxy', 'api-request-proxy-bypass',
  ];
  const binary = nativeBinary({
    apiRequestEnabled: () => true,
    contextApiRequestEnabled: () => true,
    deviceEmulationEnabled: () => true,
    contextProxyEnabled: () => true,
    apiRequestHttpProxyEnabled: () => true,
    apiRequestSocks5ProxyEnabled: () => true,
    apiRequestProxyBypassEnabled: () => true,
  });

  // Act
  const features = nativeFeatures(profile);

  // Assert
  assert.deepEqual(features, expected);
  assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  for (const marker of [
    'contextApiRequestEnabled', 'deviceEmulationEnabled', 'contextProxyEnabled', 'apiRequestHttpProxyEnabled',
    'apiRequestSocks5ProxyEnabled', 'apiRequestProxyBypassEnabled',
  ]) {
    assert.throws(
      () => verifyNativeArtifact({ ...binary, [marker]: () => false }, selected),
      /profile mismatch/,
    );
  }
});

test('Page action adapter keeps setContent bounded and selector actions delegated', async () => {
  // Arrange
  const calls = [];
  const adapter = createRequire(import.meta.url)('../src/page-actions.cjs')({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    validateOptions(options, supported) {
      for (const key of Object.keys(options)) if (!supported.includes(key)) throw new Error(`unsupported ${key}`);
    },
    UnsupportedOperationError: class extends Error {},
    locatorAction(locator, operation, options, value, api) {
      calls.push([operation, locator.selector, value, options, api]);
    },
  });
  const session = { async pageSetContent(...args) { calls.push(['setContent', ...args]); } };
  const frame = { locator(selector) { return { selector, first() { return { selector: `${selector}:first` }; } }; } };
  // Act
  await adapter.setContent(session, 'page-1', '<main>ok</main>', { waitUntil: 'domcontentloaded', timeout: 123 });
  await adapter.press(frame, 'textarea', 'Enter', { delay: 4 }, 'frame.press');
  await adapter.type(frame, 'input', 'hello', { delay: 2 }, 'page.type');
  await adapter.focus(frame, '#field', { timeout: 11 }, 'frame.focus');
  await adapter.click(frame, '#button', { force: true }, 'page.click');
  await adapter.tap(frame, '#button', { trial: true }, 'frame.tap');
  // Assert
  assert.deepEqual(calls, [
    ['setContent', 'page-1', '<main>ok</main>', 'domcontentloaded', 123],
    ['press', 'textarea', 'Enter', { delay: 4 }, 'frame.press'],
    ['type', 'input', 'hello', { delay: 2 }, 'page.type'],
    ['focus', '#field', undefined, { timeout: 11 }, 'frame.focus'],
    ['click', '#button:first', undefined, { force: true }, 'page.click'],
    ['tap', '#button', undefined, { trial: true }, 'frame.tap'],
  ]);
});

test('Page media adapter validates Playwright enums and preserves omitted versus null overrides', async () => {
  // Arrange
  const calls = [];
  const emulateMedia = createRequire(import.meta.url)('../src/emulate-media.cjs')({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    validateOptions(options, supported) {
      for (const key of Object.keys(options)) if (!supported.includes(key)) throw new Error(`unsupported ${key}`);
    },
    UnsupportedOperationError: class extends Error {},
  });
  const session = { async pageEmulateMedia(...args) { calls.push(args); } };

  // Act
  await emulateMedia(session, 'page-1', { media: 'print', colorScheme: null, contrast: 'more' });
  const invalid = emulateMedia(session, 'page-1', { reducedMotion: 'fast' });

  // Assert
  assert.deepEqual(calls, [['page-1', 'print', 'null', undefined, undefined, 'more']]);
  await assert.rejects(invalid, { message: 'reducedMotion: expected one of (reduce|no-preference|no-override)' });
});

test('Page event waiter resolves the published value and removes listeners', async () => {
  // Arrange
  const { EventEmitter } = await import('node:events');
  const emitter = new EventEmitter();
  const wait = createRequire(import.meta.url)('../src/wait-for-event.cjs')({
    TimeoutError: class extends Error {}, validateOptions: () => {},
  });
  // Act
  const pending = wait(emitter, 'popup', { predicate: value => value.ready, timeout: 1000 });
  emitter.emit('popup', { ready: false });
  const expected = { ready: true };
  emitter.emit('popup', expected);
  const value = await pending;
  // Assert
  assert.equal(value, expected);
  assert.equal(emitter.listenerCount('popup'), 0);
  assert.equal(emitter.listenerCount('close'), 0);
});

test('native profile selection rejects unknown names and path injection', () => {
  // Arrange
  const { resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['', '../default', '/tmp/custom.node', 'drag-source', undefined, null, ['default'], { toString: () => 'default' }]) {
    // Act / Assert
    assert.throws(() => resolveNativeArtifact(profile), TypeError);
  }
});

test('eight native profiles enforce independent compiled feature combinations', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const profiles = ['default', 'drag-runtime', 'frame-navigation', 'drag-frame-navigation',
    'extra-headers', 'drag-extra-headers', 'frame-extra-headers', 'drag-frame-extra-headers'];
  for (const [index, profile] of profiles.entries()) {
    const selected = resolveNativeArtifact(profile);
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.equal(Object.isFrozen(features), true);
    assert.equal(features.includes('drag-runtime'), Boolean(index & 1));
    assert.equal(features.includes('frame-navigation'), Boolean(index & 2));
    assert.equal(features.includes('extra-headers'), Boolean(index & 4));
    if (profile !== 'default') assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    for (let compiled = 0; compiled < 8; compiled++) {
      const binary = nativeBinary({ dragRuntimeEnabled: () => Boolean(compiled & 1), frameNavigationEnabled: () => Boolean(compiled & 2), extraHeadersEnabled: () => Boolean(compiled & 4) });
      if (compiled === index) assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
      else assert.throws(() => verifyNativeArtifact(binary, selected), /profile mismatch/);
    }
  }
});

test('concurrent close profiles require their own compiled marker and isolated path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['concurrent-close', 'concurrent-close-drag-frame-extra-headers']) {
    const selected = resolveNativeArtifact(profile);
    const combined = profile !== 'concurrent-close';
    const binary = nativeBinary({ dragRuntimeEnabled: () => combined, frameNavigationEnabled: () => combined,
      extraHeadersEnabled: () => combined, concurrentCloseEnabled: () => true });
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.ok(features.includes('concurrent-close'));
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, concurrentCloseEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(combined ? 'drag-frame-extra-headers' : 'default')), /profile mismatch/);
  }
});

test('upload profiles require the upload marker without selecting it in ordinary binaries', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['file-input', 'file-input-concurrent-close-drag-frame-extra-headers']) {
    const combined = profile !== 'file-input';
    const selected = resolveNativeArtifact(profile);
    const binary = nativeBinary({ fileInputEnabled: () => true, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
      concurrentCloseEnabled: () => combined });
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.ok(features.includes('file-input'));
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, fileInputEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, fileInputEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(combined ? 'concurrent-close-drag-frame-extra-headers' : 'default')), /profile mismatch/);
  }
});

test('retained upload preparation profiles require all implied markers and isolated paths', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-file-input', 'retained-file-input-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true, retainedFileInputEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      ...(combined ? ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'] : [])].sort());
    assert.equal(Object.isFrozen(features), true);
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    for (const marker of ['handlesEnabled', 'fileInputEnabled', 'retainedFileInputEnabled']) {
      assert.throws(() => verifyNativeArtifact({ ...binary, [marker]: undefined }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, [marker]: () => false }, selected), /profile mismatch/);
    }
  }
});

test('retained target-marking profiles add only their explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-target-marking', 'retained-target-marking-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetMarkingEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetMarkingEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => true }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-file-input-combined' : 'retained-file-input')), /profile mismatch/);
  }
});

test('retained target-prevalidation profiles add only their explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-target-prevalidation', 'retained-target-prevalidation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation',
      ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-target-marking-combined' : 'retained-target-marking')), /profile mismatch/);
  }
});

test('retained input-instrumentation profiles add only their addon phase marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-input-instrumentation', 'retained-input-instrumentation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedInputInstrumentationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedInputInstrumentationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-target-prevalidation-combined' : 'retained-target-prevalidation')), /profile mismatch/);
  }
});

test('retained call-argument-serialization profiles add only their addon phase marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-call-argument-serialization', 'retained-call-argument-serialization-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedCallArgumentSerializationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedCallArgumentSerializationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-input-instrumentation-combined' : 'retained-input-instrumentation')), /profile mismatch/);
  }
});

test('retained serialized-payload-import profiles add the host and addon ownership leaf', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-serialized-payload-import', 'retained-serialized-payload-import-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      retainedSerializedPayloadImportEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', 'retained-serialized-payload-import',
      ...(combined ? ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadImportEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadImportEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-call-argument-serialization-combined' : 'retained-call-argument-serialization')), /profile mismatch/);
  }
});

test('retained serialized-payload-mutation profiles add only the terminal mutation leaf', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-serialized-payload-mutation', 'retained-serialized-payload-mutation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = nativeBinary({
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      retainedSerializedPayloadImportEnabled: () => true,
      retainedSerializedPayloadMutationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    });
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', 'retained-serialized-payload-import',
      'retained-serialized-payload-mutation', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadMutationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadMutationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-serialized-payload-import-combined' : 'retained-serialized-payload-import')), /profile mismatch/);
  }
});

test('existing artifact profiles keep their exact features and reject retained preparation binaries', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const existing = {
    default: [], handles: ['handles'], 'file-input': ['file-input'],
    'file-input-concurrent-close-drag-frame-extra-headers': ['file-input', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'],
    'drag-runtime': ['drag-runtime'], 'frame-navigation': ['frame-navigation'],
    'drag-frame-navigation': ['drag-runtime', 'frame-navigation'],
    'extra-headers': ['extra-headers'], 'drag-extra-headers': ['drag-runtime', 'extra-headers'],
    'frame-extra-headers': ['frame-navigation', 'extra-headers'],
    'drag-frame-extra-headers': ['drag-runtime', 'frame-navigation', 'extra-headers'],
    'concurrent-close': ['concurrent-close'],
    'concurrent-close-drag-frame-extra-headers': ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'],
  };
  for (const [profile, expected] of Object.entries(existing)) {
    const binary = nativeBinary({
      handlesEnabled: () => expected.includes('handles'), fileInputEnabled: () => expected.includes('file-input'),
      dragRuntimeEnabled: () => expected.includes('drag-runtime'), frameNavigationEnabled: () => expected.includes('frame-navigation'),
      extraHeadersEnabled: () => expected.includes('extra-headers'), concurrentCloseEnabled: () => expected.includes('concurrent-close'),
      retainedTargetPrevalidationEnabled: () => false,
    });
    // Act
    const features = nativeFeatures(profile);
    const selected = resolveNativeArtifact(profile);
    // Assert
    assert.deepEqual(features, expected, `${profile}: previous exact feature selection`);
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedFileInputEnabled: () => true }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => true }, selected), /profile mismatch/);
  }
});

test('selected native profile requires a matching compiled capability', () => {
  // Arrange
  const { verifyNativeArtifact, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('drag-runtime');
  // Act / Assert
  assert.throws(() => verifyNativeArtifact({}, selected), /ABI mismatch/);
  assert.throws(() => verifyNativeArtifact(nativeBinary({ dragRuntimeEnabled: () => false }), selected), /profile mismatch/);
  assert.doesNotThrow(() => verifyNativeArtifact(nativeBinary({ dragRuntimeEnabled: () => true }), selected));
});

test('default requires the current ABI and core environment export', () => {
  // Arrange
  const { nativeAbiVersion, verifyNativeArtifact, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const base = resolveNativeArtifact('default');
  // Act / Assert
  assert.equal(nativeAbiVersion, 1);
  assert.throws(() => verifyNativeArtifact({}, base), /ABI mismatch/);
  assert.throws(() => verifyNativeArtifact({ nativeAbiVersion: () => 1 }, base), /ABI mismatch/);
  assert.throws(() => verifyNativeArtifact({ nativeAbiVersion: () => 2, NativeEnvironment }, base), /ABI mismatch/);
  assert.doesNotThrow(() => verifyNativeArtifact(nativeBinary(), base));
  assert.doesNotThrow(() => verifyNativeArtifact(nativeBinary({ dragRuntimeEnabled: () => false }), base));
  assert.throws(() => verifyNativeArtifact(nativeBinary({ dragRuntimeEnabled: () => true }), base), /profile mismatch/);
});

test('Keyboard facade delegates five operations and fractional delay with the same Page ID', async () => {
  // Arrange
  const calls = [];
  const Keyboard = createRequire(import.meta.url)('../src/keyboard.cjs')({ mapError: error => error, validateOptions: () => {} });
  const keyboard = new Keyboard({ async keyboard(...args) { calls.push(args); } }, '42');
  // Act
  await keyboard.down('Shift');
  await keyboard.up('Shift');
  await keyboard.press('KeyA', { delay: 0.5 });
  await keyboard.insertText('你好');
  await keyboard.type('text');
  // Assert
  assert.deepEqual(calls, [['42', 'down', 'Shift', 0], ['42', 'up', 'Shift', 0], ['42', 'press', 'KeyA', 0.5], ['42', 'insertText', '你好', 0], ['42', 'type', 'text', 0]]);
});

test('Keyboard invalid arguments reject asynchronously before native delegation', async () => {
  // Arrange
  let called = false;
  const Keyboard = createRequire(import.meta.url)('../src/keyboard.cjs')({ mapError: error => error, validateOptions: () => {} });
  const keyboard = new Keyboard({ async keyboard() { called = true; } }, '42');
  let result;
  // Act
  assert.doesNotThrow(() => { result = keyboard.press('A', { delay: NaN }); });
  // Assert
  await assert.rejects(result, TypeError);
  assert.equal(called, false);
});

test('Mouse facade delegates six methods without maintaining pointer state', async () => {
  // Arrange
  const calls = [];
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: error => error, validateOptions: () => {} });
  const mouse = new Mouse({ async mouse(...args) { calls.push(args); } }, '42');
  const defaults = { button: 0, steps: 1, clickCount: 1, delayMs: 0 };
  // Act
  await mouse.move(10.5, 20, { steps: 4 });
  await mouse.down({ button: 'right', clickCount: 2 });
  await mouse.up();
  await mouse.click(30, 40, { delay: 0.5, clickCount: 3 });
  await mouse.dblclick(50, 60, { button: 'middle' });
  await mouse.wheel(-1.5, 2.5);
  // Assert
  assert.deepEqual(calls, [
    ['42', 'move', 10.5, 20, { ...defaults, steps: 4 }],
    ['42', 'down', 0, 0, { ...defaults, button: 1, clickCount: 2 }],
    ['42', 'up', 0, 0, defaults],
    ['42', 'click', 30, 40, { ...defaults, delayMs: 0.5, clickCount: 3 }],
    ['42', 'dblclick', 50, 60, { ...defaults, button: 2 }],
    ['42', 'wheel', -1.5, 2.5, defaults],
  ]);
});

test('Mouse rejects wrong JS types asynchronously before native delegation', async () => {
  // Arrange
  let calls = 0;
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: error => error, validateOptions: () => {} });
  const mouse = new Mouse({ async mouse() { calls++; } }, '42');
  // Act
  const outcomes = await Promise.allSettled([
    mouse.move('1', 2), mouse.wheel(0, Infinity), mouse.down({ button: 'back' }),
    mouse.click(1, 2, { clickCount: '3' }),
  ]);
  // Assert
  assert.equal(calls, 0);
  for (const outcome of outcomes) {
    assert.equal(outcome.status, 'rejected');
    assert.ok(outcome.reason instanceof TypeError);
  }
});

test('Mouse maps Rust failures with the public method name', async () => {
  // Arrange
  const nativeError = new Error('native failure');
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: (error, api) => new Error(`${api}: ${error.message}`), validateOptions: () => {} });
  const mouse = new Mouse({ async mouse() { throw nativeError; } }, '42');
  // Act
  const result = mouse.dblclick(10, 20);
  // Assert
  await assert.rejects(result, { message: 'mouse.dblclick: native failure' });
});

test('ESM and CommonJS share the public API and error identity', () => {
  // Arrange
  const require = createRequire(import.meta.url);
  // Act
  const cjs = require('@shuozeli/pwright');
  // Assert
  assert.equal(cjs.chromium, api.chromium);
  assert.equal(cjs.firefox, api.firefox);
  assert.equal(cjs.webkit, api.webkit);
  assert.equal(cjs.request, api.request);
  assert.equal(cjs.devices, api.devices);
  assert.equal(cjs.errors.TimeoutError, api.errors.TimeoutError);
  assert.equal(cjs.chromium.name(), 'chromium');
  assert.equal(cjs.compatibility.apiCoverage, 'experimental-subset');
  assert.equal(Object.keys(require.cache).some(path => path.endsWith('/pwright.node')), false,
    'import alone must not load the native runtime');
});

test('browser type metadata names all public engines without launching them', () => {
  // Arrange
  const browserTypes = [api.chromium, api.firefox, api.webkit];
  // Act
  const names = browserTypes.map(browserType => browserType.name());
  // Assert
  assert.deepEqual(names, ['chromium', 'firefox', 'webkit']);
  assert.notEqual(api.firefox, api.webkit);
});

test('Firefox CDP connection rejects with the upstream error before native access', async () => {
  // Arrange
  const endpoint = { endpointURL: 'ws://foo' };
  // Act
  const connection = api.firefox.connectOverCDP(endpoint);
  // Assert
  await assert.rejects(connection, error =>
    error.constructor === Error && error.message === 'Connecting over CDP is only supported in Chromium.');
});

test('WebKit CDP connection string overload rejects with the upstream error', async () => {
  // Arrange
  const endpoint = 'ws://foo';
  // Act
  const connection = api.webkit.connectOverCDP(endpoint);
  // Assert
  await assert.rejects(connection, error =>
    error.constructor === Error && error.message === 'Connecting over CDP is only supported in Chromium.');
});

test('non-Chromium CDP object overload still requires an endpoint', async () => {
  // Arrange
  const options = {};
  // Act
  const connection = api.firefox.connectOverCDP(options);
  // Assert
  await assert.rejects(connection, { message: 'Cannot connect over CDP without wsEndpoint.' });
});

test('Firefox launch remains unsupported without a browser implementation', async () => {
  // Arrange
  const browserType = api.firefox;
  // Act
  const launch = browserType.launch();
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('launchServer rejects null channel with Playwright pre-launch validation and no native load', async () => {
  // Arrange
  const require = createRequire(import.meta.url);
  const childProcess = require('node:child_process');
  const net = require('node:net');
  const originalSpawn = childProcess.spawn;
  const originalCreateServer = net.createServer;
  const sideEffects = [];
  childProcess.spawn = () => { sideEffects.push('spawn'); throw new Error('unexpected process spawn'); };
  net.createServer = () => { sideEffects.push('server'); throw new Error('unexpected server creation'); };
  try {
    // Act
    const launch = api.chromium.launchServer({ channel: null });
    // Assert
    await assert.rejects(launch, error =>
      error.constructor.name === 'ValidationError' &&
      error.message === 'channel: expected string, got object Failed to launch browser.');
    assert.deepEqual(sideEffects, []);
    assert.equal(Object.keys(require.cache).some(path => path.endsWith('/pwright.node')), false);
  } finally {
    childProcess.spawn = originalSpawn;
    net.createServer = originalCreateServer;
  }
});

test('launchServer with a valid channel remains explicitly unsupported', async () => {
  // Arrange
  const browserType = api.chromium;
  const require = createRequire(import.meta.url);
  const childProcess = require('node:child_process');
  const net = require('node:net');
  const originalSpawn = childProcess.spawn;
  const originalCreateServer = net.createServer;
  const sideEffects = [];
  childProcess.spawn = () => { sideEffects.push('spawn'); throw new Error('unexpected process spawn'); };
  net.createServer = () => { sideEffects.push('server'); throw new Error('unexpected server creation'); };
  try {
    // Act
    const launch = browserType.launchServer({ channel: 'chrome' });
    // Assert
    await assert.rejects(launch, {
      code: 'PWRIGHT_UNSUPPORTED',
      message: 'browserType.launchServer is not supported by the current Pwright native profile',
    });
    assert.deepEqual(sideEffects, []);
    assert.equal(Object.keys(require.cache).some(path => path.endsWith('/pwright.node')), false);
  } finally {
    childProcess.spawn = originalSpawn;
    net.createServer = originalCreateServer;
  }
});

test('launchServer accepts boxed string channels before the unsupported capability boundary', async () => {
  // Arrange
  const browserType = api.firefox;
  // Act
  const launch = browserType.launchServer({ channel: new String('firefox') });
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('Chromium launchServer rejects a remote debugging pipe argument before launcher side effects', async () => {
  // Arrange
  const browserType = api.chromium;
  const options = { args: ['--remote-debugging-pipe'] };

  await assertNoLauncherSideEffects(async () => {
    // Act
    const launch = browserType.launchServer(options);
    // Assert
    await assert.rejects(launch, {
      message: 'Playwright manages remote debugging connection itself.',
    });
  });
});

async function assertNoLauncherSideEffects(assertion) {
  const require = createRequire(import.meta.url);
  const { nativeArtifact } = require('../src/native-artifact.cjs');
  const childProcess = require('node:child_process');
  const net = require('node:net');
  const fs = require('node:fs');
  const originalSpawn = childProcess.spawn;
  const originalCreateServer = net.createServer;
  const originalMkdtemp = fs.promises.mkdtemp;
  const sideEffects = [];
  childProcess.spawn = () => { sideEffects.push('spawn'); throw new Error('unexpected process spawn'); };
  net.createServer = () => { sideEffects.push('server'); throw new Error('unexpected server creation'); };
  fs.promises.mkdtemp = () => { sideEffects.push('mkdtemp'); throw new Error('unexpected temp directory'); };
  try {
    await assertion();
    assert.deepEqual(sideEffects, []);
    const loadedNativePaths = Object.keys(require.cache).filter(path => path.endsWith('/pwright.node'));
    const allowedPureLeaf = nativeArtifact.profile === 'persistent-context-preflight'
      ? nativeArtifact.path : undefined;
    assert.ok(loadedNativePaths.every(path => path === allowedPureLeaf),
      'pre-launch validation must not load a browser-capable native profile');
  } finally {
    childProcess.spawn = originalSpawn;
    net.createServer = originalCreateServer;
    fs.promises.mkdtemp = originalMkdtemp;
  }
}

test('launch rejects a truthy userDataDir option before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.chromium;
    // Act
    const launch = browserType.launch({ userDataDir: 'random-path' });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === 'userDataDir option is not supported in `browserType.launch`. Use `browserType.launchPersistentContext` instead');
  });
});

test('launch rejects a truthy port option before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.firefox;
    // Act
    const launch = browserType.launch({ port: 1234 });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === 'Cannot specify a port without launching as a server.');
  });
});

test('launch rejects a user-data-dir command-line argument before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.chromium;
    // Act
    const launch = browserType.launch({ args: ['--user-data-dir=random-path', '--profile=random-path'] });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message.includes("Pass userDataDir parameter to 'browserType.launchPersistentContext"));
  });
});

test('launch rejects a page command-line argument before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.chromium;
    // Act
    const launch = browserType.launch({ args: ['http://example.com'] });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === 'Arguments can not specify page to be opened');
  });
});

test('launch rejects a non-string proxy server before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.chromium;
    // Act
    const launch = browserType.launch({ proxy: { server: 123 } });
    // Assert
    await assert.rejects(launch, error => error.constructor.name === 'ValidationError' &&
      error.message === 'proxy.server: expected string, got number');
  });
});

test('Firefox launch rejects a nonexistent executable before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.firefox;
    // Act
    const launch = browserType.launch({ executablePath: 'random-invalid-path' });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === "browserType.launch: Failed to launch firefox because executable doesn't exist at random-invalid-path");
  });
});

test('WebKit launch rejects a nonexistent executable before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.webkit;
    // Act
    const launch = browserType.launch({ executablePath: 'random-invalid-path' });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === "browserType.launch: Failed to launch webkit because executable doesn't exist at random-invalid-path");
  });
});

test('valid Firefox executable still cannot enter an unsupported launch provider', async () => {
  // Arrange
  const browserType = api.firefox;
  // Act
  const launch = browserType.launch({ executablePath: process.execPath });
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('launchPersistentContext rejects a truthy port before launcher side effects', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const browserType = api.webkit;
    // Act
    const launch = browserType.launchPersistentContext('foo', { port: 1234 });
    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === 'Cannot specify a port without launching as a server.');
  });
});

for (const { name, browserType, options, message } of [
  {
    name: 'missing client-certificate material',
    browserType: api.chromium,
    options: { clientCertificates: [{ origin: 'test' }] },
    message: 'browserType.launchPersistentContext: None of cert, key, passphrase or pfx is specified',
  },
  {
    name: 'mixed PFX and PEM material after reading paths',
    browserType: api.webkit,
    options: { clientCertificates: [{
      origin: 'test',
      certPath: fileURLToPath(import.meta.url),
      keyPath: fileURLToPath(import.meta.url),
      pfxPath: fileURLToPath(import.meta.url),
      passphrase: 'unused',
    }] },
    message: 'browserType.launchPersistentContext: pfx is specified together with cert, key or passphrase',
  },
  {
    name: 'Chromium page argument',
    browserType: api.chromium,
    options: { args: ['http://example.test/page'] },
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  },
  {
    name: 'WebKit page argument',
    browserType: api.webkit,
    options: { args: ['http://example.test/page'] },
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  },
  {
    name: 'BiDi Firefox page argument with explicit channel',
    browserType: api.firefox,
    options: { channel: 'moz-firefox', args: ['http://example.test/page'] },
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  },
  {
    name: 'Chromium user-data-dir argument before a page argument',
    browserType: api.chromium,
    options: { args: ['--user-data-dir=unused-profile', 'http://example.test/page'] },
    message: "browserType.launchPersistentContext: Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--user-data-dir' argument",
  },
  {
    name: 'Chromium remote debugging pipe before a page argument',
    browserType: api.chromium,
    options: { args: ['--remote-debugging-pipe', 'http://example.test/page'] },
    message: 'browserType.launchPersistentContext: Playwright manages remote debugging connection itself.',
  },
  {
    name: 'certificate validation before default argument validation',
    browserType: api.chromium,
    options: { clientCertificates: [{ origin: 'test' }], args: ['http://example.test/page'] },
    message: 'browserType.launchPersistentContext: None of cert, key, passphrase or pfx is specified',
  },
]) {
  test(`launchPersistentContext rejects ${name} before launcher side effects`, async () => {
    await assertNoLauncherSideEffects(async () => {
      // Arrange
      const userDataDir = 'unused-profile';

      // Act
      const launch = browserType.launchPersistentContext(userDataDir, options);

      // Assert
      await assert.rejects(launch, error => error.constructor === Error && error.message === message);
    });
  });
}

test('launchPersistentContext reads certificate paths before checking certificate combinations', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const options = { clientCertificates: [{
      origin: 'test',
      certPath: `${fileURLToPath(import.meta.url)}/cert.pem`,
      keyPath: `${fileURLToPath(import.meta.url)}/key.pem`,
      pfxPath: `${fileURLToPath(import.meta.url)}/client.pfx`,
    }], args: ['http://example.test/page'] };

    // Act
    const launch = api.chromium.launchPersistentContext('unused-profile', options);

    // Assert
    await assert.rejects(launch, error => error.code === 'ENOTDIR');
  });
});

test('launchPersistentContext retains the client port error before certificate and argument checks', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const options = { port: 1234, clientCertificates: [{ origin: 'test' }], args: ['http://example.test/page'] };

    // Act
    const launch = api.chromium.launchPersistentContext('unused-profile', options);

    // Assert
    await assert.rejects(launch, error => error.constructor === Error &&
      error.message === 'Cannot specify a port without launching as a server.');
  });
});

test('plain Firefox page argument remains outside the pre-spawn rejection', async () => {
  // Arrange
  const options = { args: ['http://example.test/page'] };

  // Act
  const launch = api.firefox.launchPersistentContext('unused-profile', options);

  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('inherited fixture channel defaults enable BiDi Firefox page-argument validation', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const implementation = createRequire(import.meta.url)('../src/index.cjs');
    const adapted = Object.assign(Object.create(implementation), implementation);
    adapted._defaultLaunchOptions = { channel: 'moz-firefox' };

    try {
      // Act
      const launch = adapted.firefox.launchPersistentContext('unused-profile', {
        args: ['http://example.test/page'],
      });

      // Assert
      await assert.rejects(launch, {
        message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
      });
      assert.equal(Object.hasOwn(adapted, '_defaultLaunchOptions'), false);
    } finally {
      adapted._defaultLaunchOptions = undefined;
    }
  });
});

test('explicit persistent-context options override inherited fixture defaults', async () => {
  await assertNoLauncherSideEffects(async () => {
    // Arrange
    const implementation = createRequire(import.meta.url)('../src/index.cjs');
    const adapted = Object.assign(Object.create(implementation), implementation);
    adapted._defaultLaunchOptions = {
      channel: 'moz-firefox',
      args: ['http://example.test/default-page'],
    };

    try {
      // Act
      const launch = adapted.firefox.launchPersistentContext('unused-profile', {
        channel: 'firefox',
        args: ['http://example.test/explicit-page'],
      });

      // Assert
      await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
    } finally {
      adapted._defaultLaunchOptions = undefined;
    }
  });
});

test('ignoreDefaultArgs true bypasses default-argument checks', async () => {
  // Arrange
  const args = ['http://example.test/page'];

  // Act
  const launch = api.chromium.launchPersistentContext('unused-profile', { args, ignoreDefaultArgs: true });

  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('ignoreDefaultArgs array still validates default arguments', async () => {
  // Arrange
  const args = ['http://example.test/page'];

  // Act
  const launch = api.chromium.launchPersistentContext('unused-profile', { args, ignoreDefaultArgs: [] });

  // Assert
  await assert.rejects(launch, {
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  });
});

test('falsy launch port follows the unsupported capability boundary', async () => {
  // Arrange
  const browserType = api.chromium;
  // Act
  const launch = browserType.launch({ port: 0 });
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('falsy launch userDataDir follows the unsupported capability boundary', async () => {
  // Arrange
  const browserType = api.chromium;
  // Act
  const launch = browserType.launch({ userDataDir: '' });
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('falsy persistent-context port follows the unsupported capability boundary', async () => {
  // Arrange
  const browserType = api.chromium;
  // Act
  const launch = browserType.launchPersistentContext('foo', { port: 0 });
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('valid persistent-context launch remains unsupported', async () => {
  // Arrange
  const browserType = api.chromium;
  // Act
  const launch = browserType.launchPersistentContext('foo');
  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('browserType.connect remains unsupported without a network attempt', async () => {
  // Arrange
  const browserType = api.chromium;
  // Act
  const connection = browserType.connect('ws://example.invalid');
  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('launch rejects asynchronously according to the selected native profile', async () => {
  // Arrange
  let result;
  const ownedLaunch = api.compatibility.browserOwnership === 'external-and-owned';
  // Act
  assert.doesNotThrow(() => { result = api.chromium.launch(); });
  // Assert
  if (ownedLaunch) {
    await assert.rejects(result, {
      name: 'TypeError',
      message: 'browserType.launch: executablePath must be a nonempty absolute path',
    });
  } else {
    await assert.rejects(result, api.errors.UnsupportedOperationError);
  }
});

test('unsupported options fail before loading native code or connecting', async () => {
  // Arrange
  const options = { unsupportedOption: 'unused' };
  // Act
  const result = api.chromium.connectOverCDP('http://127.0.0.1:9222', options);
  // Assert
  await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('invalid timeouts are not truncated by the FFI integer conversion', async () => {
  // Arrange
  const timeout = 2 ** 32;
  // Act
  const result = api.chromium.connectOverCDP('http://127.0.0.1:9222', { timeout });
  // Assert
  await assert.rejects(result, TypeError);
});
