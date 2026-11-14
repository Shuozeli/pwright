'use strict';
const { join } = require('node:path');

const nativeAbiVersion = 1;
const officialConformancePageFeatures = Object.freeze(['set-content', 'page-events', 'page-dialogs', 'page-routes', 'console-events', 'page-bindings', 'handles', 'element-handle-dom', 'file-input', 'frame-navigation', 'history-navigation', 'init-scripts', 'timeout-settings', 'wait-for-function', 'wait-for-url', 'wait-for-timeout', 'large-evaluation-results', 'emulate-media', 'device-emulation', 'touchscreen-input', 'page-viewport', 'request-gc', 'response-body', 'response-request', 'request-events', 'response-events', 'response-finished', 'request-frame', 'request-post-data-json', 'locator-bounding-box', 'locator-highlight', 'locator-screenshot', 'locator-dom-actions', 'extra-headers', 'selector-registration', 'bypass-csp', 'api-request', 'context-api-request']);
const officialConformancePageLightFeatures = Object.freeze([...officialConformancePageFeatures, 'light-selectors']);
const officialConformancePageFrameElementFeatures = Object.freeze([...officialConformancePageFeatures, 'frame-element']);
const officialConformancePageFrameEventsFeatures = Object.freeze([...officialConformancePageFeatures, 'frame-events']);
const officialConformancePageFrameHierarchyFeatures = Object.freeze([...officialConformancePageFrameEventsFeatures, 'frame-detach-events', 'frame-element']);
const officialConformancePageCdpHandshakeHeadersFeatures = Object.freeze([...officialConformancePageFeatures, 'cdp-handshake-headers']);
const officialConformancePageCdpDefaultUserAgentFeatures = Object.freeze([...officialConformancePageCdpHandshakeHeadersFeatures, 'cdp-default-user-agent']);
const officialConformancePageCdpErrorsFeatures = Object.freeze([...officialConformancePageCdpDefaultUserAgentFeatures, 'cdp-discovery-errors', 'cdp-ws-close-reason']);
const officialConformancePageCdpSessionFeatures = Object.freeze([...officialConformancePageFeatures, 'cdp-session', 'browser-cdp-session']);
const officialConformancePageGotoBurst20Features = Object.freeze([...officialConformancePageFeatures, 'frame-navigation-burst-20']);
const officialConformancePageGotoFrameDetachFeatures = Object.freeze([...officialConformancePageGotoBurst20Features, 'frame-detach-events']);
const officialConformancePageRouteHeaderUrlFeatures = Object.freeze([...officialConformancePageGotoBurst20Features, 'route-header-url-overrides']);
const officialConformancePageRouteHeaderUrlUnrouteFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlFeatures, 'page-unroute']);
const officialConformancePageRouteHeaderUrlUnrouteCorsFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteFeatures, 'route-fulfill-cors']);
const officialConformancePageRouteHeaderUrlUnrouteCorsRegexpFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteCorsFeatures, 'route-regexp']);
const officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteCorsRegexpFeatures, 'route-request-history']);
const officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryCookiesFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryFeatures, 'context-cookies']);
const officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryCookiesLargePostDataFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryCookiesFeatures, 'large-evaluation-inputs', 'large-route-post-data']);
const officialConformancePageFillNavigationCommitFeatures = Object.freeze([...officialConformancePageRouteHeaderUrlUnrouteFeatures, 'fill-navigation-commit']);
const officialConformancePageSelectOptionTimeoutFeatures = Object.freeze([...officialConformancePageFeatures, 'select-option-timeout-progress']);
const officialConformancePageSelectOptionNavigationCommitFeatures = Object.freeze([...officialConformancePageSelectOptionTimeoutFeatures, 'select-option-navigation-commit']);
const officialConformancePageReactSelectorsFeatures = Object.freeze([...officialConformancePageFeatures, 'react-selectors']);
const officialConformancePageRetainedScrollFeatures = Object.freeze([...officialConformancePageFeatures, 'element-handle-scroll-into-view']);
const officialConformancePageRetainedWaitFeatures = Object.freeze([...officialConformancePageRetainedScrollFeatures, 'element-handle-wait-for-selector']);
const officialConformancePageRetainedSelectOptionValueFeatures = Object.freeze([...officialConformancePageRetainedWaitFeatures, 'element-handle-select-option-value']);

// Packaging selection only. Every browser operation remains in the Rust host.
const profiles = Object.freeze({
  default: Object.freeze([]),
  'cdp-handshake-headers': Object.freeze(['cdp-handshake-headers']),
  'cdp-default-user-agent': Object.freeze(['cdp-handshake-headers', 'cdp-default-user-agent']),
  'cdp-discovery-errors': Object.freeze(['cdp-discovery-errors']),
  'cdp-ws-close-reason': Object.freeze(['cdp-ws-close-reason']),
  'cdp-session': Object.freeze(['cdp-session']),
  'browser-cdp-session': Object.freeze(['cdp-session', 'browser-cdp-session']),
  'browser-registry': Object.freeze(['browser-registry']),
  'playwright-server-connect-ws-error': Object.freeze(['playwright-server-connect-ws-error']),
  'playwright-server-connect-close-error': Object.freeze(['playwright-server-connect-close-error']),
  'playwright-server-connect-headers': Object.freeze(['playwright-server-connect-headers']),
  'playwright-server-connect-session': Object.freeze(['playwright-server-connect-session']),
  'playwright-server-connect-page-events': Object.freeze(['playwright-server-connect-session', 'playwright-server-connect-page-events']),
  'persistent-context-preflight': Object.freeze(['persistent-context-preflight']),
  'owned-launch': Object.freeze(['owned-launch']),
  'playwright-trace': Object.freeze(['owned-launch', 'playwright-trace']),
  'bidi-startup-failure': Object.freeze(['owned-launch', 'playwright-trace', 'bidi-startup-failure']),
  handles: Object.freeze(['handles']),
  'frame-element': Object.freeze(['handles', 'frame-element']),
  'frame-events': Object.freeze(['page-events', 'frame-events']),
  'element-handle-dom': Object.freeze(['handles', 'element-handle-dom']),
  'element-handle-select-option-value': Object.freeze(['handles', 'timeout-settings', 'element-handle-select-option-value']),
  'element-handle-wait-for-selector': Object.freeze(['handles', 'timeout-settings', 'element-handle-wait-for-selector']),
  'element-handle-scroll-into-view': Object.freeze(['handles', 'timeout-settings', 'element-handle-scroll-into-view']),
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
  'frame-navigation-burst-20': Object.freeze(['frame-navigation', 'frame-navigation-burst-20']),
  'response-request': Object.freeze(['frame-navigation', 'response-request']),
  'route-header-url-overrides': Object.freeze(['page-routes', 'route-header-url-overrides']),
  'route-fulfill-cors': Object.freeze(['page-routes', 'route-fulfill-cors']),
  'route-regexp': Object.freeze(['page-routes', 'route-regexp']),
  'route-request-history': Object.freeze(['page-routes', 'route-request-history']),
  'context-cookies': Object.freeze(['context-cookies']),
  'route-large-post-data': Object.freeze(['frame-navigation', 'page-routes', 'large-evaluation-inputs', 'large-route-post-data']),
  'page-unroute': Object.freeze(['page-routes', 'page-unroute']),
  'fill-navigation-commit': Object.freeze(['fill-navigation-commit']),
  'select-option-navigation-commit': Object.freeze(['select-option-navigation-commit']),
  'select-option-timeout-progress': Object.freeze(['select-option-timeout-progress']),
  'react-selectors': Object.freeze(['react-selectors']),
  'request-events': Object.freeze(['request-events']),
  'response-events': Object.freeze(['response-events']),
  'response-finished': Object.freeze(['response-events', 'response-finished']),
  'request-post-data-json': Object.freeze(['request-events', 'request-post-data-json']),
  'history-navigation': Object.freeze(['history-navigation']),
  'drag-frame-navigation': Object.freeze(['drag-runtime', 'frame-navigation']),
  'extra-headers': Object.freeze(['extra-headers']),
  'drag-extra-headers': Object.freeze(['drag-runtime', 'extra-headers']),
  'frame-extra-headers': Object.freeze(['frame-navigation', 'extra-headers']),
  'drag-frame-extra-headers': Object.freeze(['drag-runtime', 'frame-navigation', 'extra-headers']),
  'concurrent-close': Object.freeze(['concurrent-close']),
  'concurrent-close-drag-frame-extra-headers': Object.freeze(['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers']),
  'set-content': Object.freeze(['set-content']),
  'page-events': Object.freeze(['page-events']),
  'set-content-page-events': Object.freeze(['set-content', 'page-events']),
  'set-content-page-events-console': Object.freeze(['set-content', 'page-events', 'console-events']),
  'set-content-page-events-console-bindings': Object.freeze(['set-content', 'page-events', 'console-events', 'page-bindings', 'handles']),
  'selector-registration': Object.freeze(['selector-registration']),
  'context-offline': Object.freeze(['context-offline']),
  'bypass-csp': Object.freeze(['bypass-csp']),
  'api-request': Object.freeze(['api-request']),
  'api-request-http-proxy': Object.freeze(['api-request', 'api-request-http-proxy']),
  'api-request-socks5-proxy': Object.freeze(['api-request', 'api-request-socks5-proxy']),
  'api-request-http-proxy-bypass': Object.freeze(['api-request', 'api-request-http-proxy', 'api-request-proxy-bypass']),
  'api-request-socks5-proxy-bypass': Object.freeze(['api-request', 'api-request-socks5-proxy', 'api-request-proxy-bypass']),
  'api-request-client-certificates': Object.freeze(['api-request', 'api-request-client-certificates']),
  'api-request-client-certificates-http-proxy': Object.freeze(['api-request', 'api-request-client-certificates', 'api-request-http-proxy']),
  'api-request-client-certificates-socks5-proxy': Object.freeze(['api-request', 'api-request-client-certificates', 'api-request-socks5-proxy']),
  'device-emulation': Object.freeze(['device-emulation']),
  'touchscreen-input': Object.freeze(['device-emulation', 'touchscreen-input']),
  'context-api-request-proxy': Object.freeze(['api-request', 'context-api-request', 'device-emulation', 'context-proxy', 'api-request-http-proxy', 'api-request-socks5-proxy', 'api-request-proxy-bypass']),
  'init-scripts': Object.freeze(['init-scripts']),
  'official-conformance-page-bindings': officialConformancePageFeatures,
  'official-conformance-page-cdp-handshake-headers': officialConformancePageCdpHandshakeHeadersFeatures,
  'official-conformance-page-cdp-default-user-agent': officialConformancePageCdpDefaultUserAgentFeatures,
  'official-conformance-page-cdp-errors': officialConformancePageCdpErrorsFeatures,
  'official-conformance-page-cdp-session': officialConformancePageCdpSessionFeatures,
  'official-conformance-page-check': officialConformancePageFeatures,
  'official-conformance-page-frame-element': officialConformancePageFrameElementFeatures,
  'official-conformance-page-frame-events': officialConformancePageFrameEventsFeatures,
  'official-conformance-page-frame-hierarchy': officialConformancePageFrameHierarchyFeatures,
  'official-conformance-page-check-light-v1': officialConformancePageLightFeatures,
  'official-conformance-page-dispatch-event': officialConformancePageFeatures,
  'official-conformance-page-goto-burst-20': officialConformancePageGotoBurst20Features,
  'official-conformance-page-goto-frame-detach': officialConformancePageGotoFrameDetachFeatures,
  'official-conformance-page-route-header-url': officialConformancePageRouteHeaderUrlFeatures,
  'official-conformance-page-route-header-url-unroute': officialConformancePageRouteHeaderUrlUnrouteFeatures,
  'official-conformance-page-route-header-url-unroute-cors': officialConformancePageRouteHeaderUrlUnrouteCorsFeatures,
  'official-conformance-page-route-header-url-unroute-cors-regexp': officialConformancePageRouteHeaderUrlUnrouteCorsRegexpFeatures,
  'official-conformance-page-route-header-url-unroute-cors-regexp-history': officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryFeatures,
  'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies': officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryCookiesFeatures,
  'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data': officialConformancePageRouteHeaderUrlUnrouteCorsRegexpHistoryCookiesLargePostDataFeatures,
  'official-conformance-page-fill-navigation-commit': officialConformancePageFillNavigationCommitFeatures,
  'official-conformance-page-select-option-timeout-progress': officialConformancePageSelectOptionTimeoutFeatures,
  'official-conformance-page-select-option-navigation-commit': officialConformancePageSelectOptionNavigationCommitFeatures,
  'official-conformance-page-react-selectors': officialConformancePageReactSelectorsFeatures,
  'official-conformance-page-retained-scroll': officialConformancePageRetainedScrollFeatures,
  'official-conformance-page-retained-wait': officialConformancePageRetainedWaitFeatures,
  'official-conformance-page-retained-select-option-value': officialConformancePageRetainedSelectOptionValueFeatures,
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
  if (!binary || typeof binary !== 'object'
      || typeof binary.nativeAbiVersion !== 'function'
      || binary.nativeAbiVersion() !== nativeAbiVersion
      || typeof binary.NativeEnvironment !== 'function') {
    throw new Error(`Pwright native ABI mismatch: expected ABI ${nativeAbiVersion} with NativeEnvironment. Rebuild ${selection.profile} explicitly.`);
  }
  // Every requested capability must be proved by the selected binary.
  const features = nativeFeatures(selection.profile);
  const cdpHandshakeHeaders = typeof binary.cdpHandshakeHeadersEnabled === 'function'
    && binary.cdpHandshakeHeadersEnabled();
  const cdpHandshakeHeadersPort = typeof binary.NativeEnvironment?.prototype?.connectOverCDPWithHeaders === 'function';
  if (cdpHandshakeHeaders !== features.includes('cdp-handshake-headers')
      || cdpHandshakeHeadersPort !== features.includes('cdp-handshake-headers'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const cdpDefaultUserAgent = typeof binary.cdpDefaultUserAgentEnabled === 'function'
    && binary.cdpDefaultUserAgentEnabled();
  const cdpDefaultUserAgentPort = typeof binary.NativeEnvironment?.prototype?.connectOverCDPWithDefaultUserAgent === 'function';
  if (cdpDefaultUserAgent !== features.includes('cdp-default-user-agent')
      || cdpDefaultUserAgentPort !== features.includes('cdp-default-user-agent'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const cdpDiscoveryErrors = typeof binary.cdpDiscoveryErrorsEnabled === 'function'
    && binary.cdpDiscoveryErrorsEnabled();
  const cdpWsCloseReason = typeof binary.cdpWsCloseReasonEnabled === 'function'
    && binary.cdpWsCloseReasonEnabled();
  if (cdpDiscoveryErrors !== features.includes('cdp-discovery-errors')
      || cdpWsCloseReason !== features.includes('cdp-ws-close-reason'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const cdpSessionBindings = [
    binary.NativeCdpSessionLease,
    binary.NativeCdpEventBatch,
    binary.NativeSession?.prototype?.cdpSessionCreate,
    binary.NativeSession?.prototype?.cdpSessionSend,
    binary.NativeSession?.prototype?.cdpSessionClaim,
    binary.NativeSession?.prototype?.cdpSessionDetach,
    binary.NativeSession?.prototype?.nextCdpSessionEventGeneration,
    binary.NativeCdpSessionLease?.prototype?.token,
    binary.NativeCdpSessionLease?.prototype?.claim,
    binary.NativeCdpSessionLease?.prototype?.abort,
    binary.NativeCdpEventBatch?.prototype?.peek,
    binary.NativeCdpEventBatch?.prototype?.ackNext,
    binary.NativeCdpEventBatch?.prototype?.rollback,
  ];
  const cdpSession = cdpSessionBindings.every(binding => typeof binding === 'function');
  const anyCdpSessionBinding = cdpSessionBindings.some(binding => typeof binding === 'function');
  if (cdpSession !== features.includes('cdp-session')
      || anyCdpSessionBinding !== features.includes('cdp-session')
      || (cdpSession && typeof binary.NativeSession?.prototype?.reportCallbackException !== 'function'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  if ((typeof binary.NativeSession?.prototype?.cdpBrowserSessionCreate === 'function')
      !== features.includes('browser-cdp-session'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const browserRegistry = typeof binary.browserRegistryEnabled === 'function'
    && binary.browserRegistryEnabled();
  // A distinct export makes the import-time snapshot contract fail closed
  // against an older two-argument browser-registry binary.
  const browserExecutablePath = typeof binary.browserExecutablePathSnapshot === 'function';
  if (browserRegistry !== features.includes('browser-registry')
      || browserExecutablePath !== browserRegistry
      || typeof binary.browserExecutablePath === 'function')
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const serverConnectWsError = typeof binary.playwrightServerConnectWsErrorEnabled === 'function'
    && binary.playwrightServerConnectWsErrorEnabled();
  const serverConnectWsErrorPort = typeof binary.NativeEnvironment?.prototype?.connectPlaywrightServerError === 'function';
  if (serverConnectWsError !== features.includes('playwright-server-connect-ws-error')
      || serverConnectWsErrorPort !== serverConnectWsError)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const serverConnectCloseError = typeof binary.playwrightServerConnectCloseErrorEnabled === 'function'
    && binary.playwrightServerConnectCloseErrorEnabled();
  const serverConnectCloseErrorPort = typeof binary.NativeEnvironment?.prototype?.connectPlaywrightServerCloseError === 'function';
  if (serverConnectCloseError !== features.includes('playwright-server-connect-close-error')
      || serverConnectCloseErrorPort !== serverConnectCloseError)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const serverConnectHeaders = typeof binary.playwrightServerConnectHeadersEnabled === 'function'
    && binary.playwrightServerConnectHeadersEnabled();
  const serverConnectHeadersPort = typeof binary.NativeEnvironment?.prototype?.connectPlaywrightServerHeaders === 'function';
  const serverConnectHeadersAbiShape = typeof binary.playwrightServerConnectHeadersAbiShape === 'function'
    ? binary.playwrightServerConnectHeadersAbiShape() : 0;
  if (serverConnectHeaders !== features.includes('playwright-server-connect-headers')
      || serverConnectHeadersPort !== serverConnectHeaders
      || serverConnectHeadersAbiShape !== (serverConnectHeaders ? 2 : 0))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const serverConnectSession = typeof binary.playwrightServerConnectSessionEnabled === 'function'
    && binary.playwrightServerConnectSessionEnabled();
  const serverConnectSessionAbiShape = typeof binary.playwrightServerConnectSessionAbiShape === 'function'
    ? binary.playwrightServerConnectSessionAbiShape() : 0;
  const remoteBindings = [
    binary.NativeEnvironment?.prototype?.connectPlaywrightServer,
    binary.NativeRemoteBrowser?.prototype?.version,
    binary.NativeRemoteBrowser?.prototype?.name,
    binary.NativeRemoteBrowser?.prototype?.isConnected,
    binary.NativeRemoteBrowser?.prototype?.close,
    binary.NativeRemoteBrowser?.prototype?.waitTerminal,
    binary.NativeRemoteBrowser?.prototype?.reportCallbackException,
    binary.NativeRemoteBrowser?.prototype?.contextGuids,
    binary.NativeRemoteBrowser?.prototype?.newContext,
    binary.NativeRemoteContext?.prototype?.guid,
    binary.NativeRemoteContext?.prototype?.isOpen,
    binary.NativeRemoteContext?.prototype?.pageGuids,
    binary.NativeRemoteContext?.prototype?.newPage,
    binary.NativeRemotePage?.prototype?.guid,
    binary.NativeRemotePage?.prototype?.isOpen,
  ];
  if (serverConnectSession !== features.includes('playwright-server-connect-session')
      || remoteBindings.every(binding => typeof binding === 'function') !== serverConnectSession
      || remoteBindings.some(binding => typeof binding === 'function') !== serverConnectSession
      || serverConnectSessionAbiShape !== (serverConnectSession ? 2 : 0))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const serverPageEvents = typeof binary.playwrightServerConnectPageEventsEnabled === 'function'
    && binary.playwrightServerConnectPageEventsEnabled();
  const serverPageEventsAbiShape = typeof binary.playwrightServerConnectPageEventsAbiShape === 'function'
    ? binary.playwrightServerConnectPageEventsAbiShape() : 0;
  const remotePageEventBindings = [
    binary.NativeEnvironment?.prototype?.connectPlaywrightServerPageEvents,
    binary.NativeRemoteBrowser?.prototype?.nextPageNotice,
    binary.NativeRemoteContext?.prototype?.createdSequence,
    binary.NativeRemotePage?.prototype?.createdSequence,
    binary.NativeRemotePage?.prototype?.urlGlob,
  ];
  if (serverPageEvents !== features.includes('playwright-server-connect-page-events')
      || remotePageEventBindings.every(binding => typeof binding === 'function') !== serverPageEvents
      || remotePageEventBindings.some(binding => typeof binding === 'function') !== serverPageEvents
      || serverPageEventsAbiShape !== (serverPageEvents ? 1 : 0))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const persistentContextPreflight = typeof binary.persistentContextPreflightEnabled === 'function'
    && binary.persistentContextPreflightEnabled();
  const persistentContextCertificateError = typeof binary.persistentContextCertificateError === 'function';
  const persistentContextArgumentError = typeof binary.persistentContextArgumentError === 'function';
  if (persistentContextPreflight !== features.includes('persistent-context-preflight')
      || persistentContextCertificateError !== persistentContextPreflight
      || persistentContextArgumentError !== persistentContextPreflight)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const ownedLaunch = typeof binary.ownedLaunchEnabled === 'function'
    && binary.ownedLaunchEnabled()
    && typeof binary.NativeEnvironment?.prototype?.launchChromium === 'function';
  if (ownedLaunch !== features.includes('owned-launch'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const playwrightTraceDir = typeof binary.playwrightTraceDirEnabled === 'function'
    && binary.playwrightTraceDirEnabled();
  const playwrightTraceDirLaunch = typeof binary.NativeEnvironment?.prototype?.launchChromiumWithTracesDir === 'function';
  if (playwrightTraceDir !== features.includes('playwright-trace')
      || playwrightTraceDirLaunch !== features.includes('playwright-trace'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const bidiStartupFailure = typeof binary.bidiStartupFailureEnabled === 'function'
    && binary.bidiStartupFailureEnabled();
  const bidiStartupFailurePort = typeof binary.NativeEnvironment?.prototype?.observeBidiStartupFailure === 'function';
  if (bidiStartupFailure !== features.includes('bidi-startup-failure')
      || bidiStartupFailurePort !== features.includes('bidi-startup-failure'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const drag = typeof binary.dragRuntimeEnabled === 'function' && binary.dragRuntimeEnabled();
  const frame = typeof binary.frameNavigationEnabled === 'function' && binary.frameNavigationEnabled();
  const frameNavigationBurst20 = typeof binary.frameNavigationBurst20Enabled === 'function'
    && binary.frameNavigationBurst20Enabled();
  if (frameNavigationBurst20 !== features.includes('frame-navigation-burst-20'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const historyNavigation = typeof binary.historyNavigationEnabled === 'function' && binary.historyNavigationEnabled()
    && typeof binary.NativeSession?.prototype?.pageGoBack === 'function'
    && typeof binary.NativeSession?.prototype?.pageGoForward === 'function';
  const setContent = typeof binary.setContentEnabled === 'function' && binary.setContentEnabled();
  const pageEvents = typeof binary.pageEventsEnabled === 'function' && binary.pageEventsEnabled();
  const frameEvents = typeof binary.frameEventsEnabled === 'function'
    && binary.frameEventsEnabled();
  if (frameEvents !== features.includes('frame-events'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const frameDetachEvents = typeof binary.frameDetachEventsEnabled === 'function'
    && binary.frameDetachEventsEnabled();
  const frameDetachBindings = typeof binary.NativeFrameTree?.prototype?.retiredMetadata === 'function'
    && typeof binary.NativeFrameTree?.prototype?.eventMetadata === 'function'
    && typeof binary.NativeFrameTree?.prototype?.finishDetached === 'function'
    && typeof binary.NativeFrameNode?.prototype?.parentId === 'function';
  if (frameDetachEvents !== features.includes('frame-detach-events')
      || (frameDetachEvents && !frameDetachBindings))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const pageDialogs = typeof binary.pageDialogsEnabled === 'function' && binary.pageDialogsEnabled()
    && typeof binary.NativeDialog === 'function'
    && typeof binary.NativeSession?.prototype?.handleDialog === 'function';
  const pageRoutes = typeof binary.pageRoutesEnabled === 'function' && binary.pageRoutesEnabled()
    && typeof binary.NativeSession?.prototype?.pageRoute === 'function'
    && typeof binary.NativeSession?.prototype?.nextPageRouteCall === 'function'
    && typeof binary.NativeSession?.prototype?.pageRouteContinue === 'function';
  const pageUnroute = typeof binary.pageUnrouteEnabled === 'function'
    && binary.pageUnrouteEnabled();
  const pageUnrouteBinding = typeof binary.NativeSession?.prototype?.pageUnroute === 'function';
  if (pageUnroute !== features.includes('page-unroute')
      || pageUnrouteBinding !== pageUnroute)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const routeHeaderUrlOverrides = typeof binary.routeHeaderUrlOverridesEnabled === 'function'
    && binary.routeHeaderUrlOverridesEnabled();
  const routeHeaderUrlBindings = typeof binary.NativeSession?.prototype?.pageRouteContinueWithHeaders === 'function'
    && typeof binary.NativeSession?.prototype?.pageRouteFallbackWithUrl === 'function';
  if (routeHeaderUrlOverrides !== features.includes('route-header-url-overrides')
      || routeHeaderUrlBindings !== routeHeaderUrlOverrides)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const routeFulfillCors = typeof binary.routeFulfillCorsEnabled === 'function'
    && binary.routeFulfillCorsEnabled();
  const routeFulfillCorsBinding = typeof binary.NativeSession?.prototype?.pageRouteFulfill === 'function';
  if (routeFulfillCors !== features.includes('route-fulfill-cors')
      || (routeFulfillCors && !routeFulfillCorsBinding))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const routeRegExp = typeof binary.routeRegExpEnabled === 'function'
    && binary.routeRegExpEnabled();
  const routeRegExpBinding = typeof binary.NativeSession?.prototype?.pageRouteRegExp === 'function';
  if (routeRegExp !== features.includes('route-regexp')
      || routeRegExpBinding !== routeRegExp)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const routeRequestHistory = typeof binary.routeRequestHistoryEnabled === 'function'
    && binary.routeRequestHistoryEnabled();
  const routeRequestHistoryBinding = typeof binary.NativeSession?.prototype?.responseRawHeaders === 'function'
    && typeof binary.NativeSession?.prototype?.requestResponse === 'function';
  if (routeRequestHistory !== features.includes('route-request-history')
      || (routeRequestHistory && !routeRequestHistoryBinding))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const contextCookies = typeof binary.contextCookiesEnabled === 'function'
    && binary.contextCookiesEnabled();
  const contextCookiesBinding = typeof binary.NativeSession?.prototype?.contextAddCookies === 'function';
  if (contextCookies !== features.includes('context-cookies')
      || contextCookiesBinding !== contextCookies)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const largeEvaluationInputs = typeof binary.largeEvaluationInputsEnabled === 'function'
    && binary.largeEvaluationInputsEnabled();
  const largeRoutePostData = typeof binary.largeRoutePostDataEnabled === 'function'
    && binary.largeRoutePostDataEnabled();
  if (largeEvaluationInputs !== features.includes('large-evaluation-inputs')
      || (largeEvaluationInputs && typeof binary.NativeSession?.prototype?.frameEvaluate !== 'function')
      || largeRoutePostData !== features.includes('large-route-post-data')
      || (largeRoutePostData && typeof binary.NativeSession?.prototype?.nextPageRouteCall !== 'function'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const consoleEvents = typeof binary.consoleEventsEnabled === 'function' && binary.consoleEventsEnabled();
  const consoleListenerControls = typeof binary.NativeSession?.prototype?.setPageConsoleListener === 'function'
    && typeof binary.NativeSession?.prototype?.setContextConsoleListener === 'function';
  const pageBindings = typeof binary.pageBindingsEnabled === 'function' && binary.pageBindingsEnabled();
  const initScripts = typeof binary.initScriptsEnabled === 'function' && binary.initScriptsEnabled();
  const waitForFunction = typeof binary.waitForFunctionEnabled === 'function' && binary.waitForFunctionEnabled();
  const waitForUrl = typeof binary.waitForUrlEnabled === 'function' && binary.waitForUrlEnabled();
  const waitForTimeout = typeof binary.waitForTimeoutEnabled === 'function' && binary.waitForTimeoutEnabled();
  const timeoutSettings = typeof binary.timeoutSettingsEnabled === 'function' && binary.timeoutSettingsEnabled();
  const emulateMedia = typeof binary.emulateMediaEnabled === 'function' && binary.emulateMediaEnabled();
  const deviceEmulation = typeof binary.deviceEmulationEnabled === 'function' && binary.deviceEmulationEnabled()
    && typeof binary.NativeSession?.prototype?.newContextWithHasTouch === 'function';
  const touchscreenInput = typeof binary.touchscreenInputEnabled === 'function' && binary.touchscreenInputEnabled()
    && typeof binary.NativeSession?.prototype?.touchscreenTap === 'function';
  const elementHandleTap = typeof binary.NativeSession?.prototype?.elementHandleTap === 'function';
  const needsElementHandleTap = features.includes('element-handle-dom')
    && features.includes('touchscreen-input');
  const elementHandleDispatchEvent = !['official-conformance-page-cdp-session', 'official-conformance-page-dispatch-event', 'official-conformance-page-goto-burst-20', 'official-conformance-page-goto-frame-detach', 'official-conformance-page-route-header-url', 'official-conformance-page-route-header-url-unroute', 'official-conformance-page-route-header-url-unroute-cors', 'official-conformance-page-route-header-url-unroute-cors-regexp', 'official-conformance-page-route-header-url-unroute-cors-regexp-history', 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies', 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data', 'official-conformance-page-fill-navigation-commit', 'official-conformance-page-select-option-timeout-progress', 'official-conformance-page-select-option-navigation-commit', 'official-conformance-page-react-selectors', 'official-conformance-page-retained-scroll', 'official-conformance-page-retained-wait'].includes(selection.profile)
    || typeof binary.NativeSession?.prototype?.elementHandleDispatchEvent === 'function';
  const pageViewport = typeof binary.pageViewportEnabled === 'function' && binary.pageViewportEnabled()
    && typeof binary.NativeSession?.prototype?.pageSetViewportSize === 'function';
  const requestGc = typeof binary.requestGcEnabled === 'function' && binary.requestGcEnabled();
  const responseBody = typeof binary.responseBodyEnabled === 'function' && binary.responseBodyEnabled();
  const responseRequest = typeof binary.responseRequestEnabled === 'function' && binary.responseRequestEnabled();
  const requestEvents = typeof binary.requestEventsEnabled === 'function' && binary.requestEventsEnabled()
    && typeof binary.NativeSession?.prototype?.nextRequestNotice === 'function'
    && typeof binary.NativeSession?.prototype?.pageUrlGlob === 'function';
  const responseEvents = typeof binary.responseEventsEnabled === 'function' && binary.responseEventsEnabled()
    && typeof binary.NativeSession?.prototype?.nextResponseNotice === 'function'
    && typeof binary.NativeSession?.prototype?.pageUrlGlob === 'function';
  const responseFinished = typeof binary.responseFinishedEnabled === 'function' && binary.responseFinishedEnabled()
    && typeof binary.NativeSession?.prototype?.responseFinished === 'function';
  const requestFrame = typeof binary.requestFrameEnabled === 'function' && binary.requestFrameEnabled();
  const orderedNetwork = !features.includes('request-events') || !features.includes('response-events')
    || (typeof binary.NativeSession?.prototype?.nextNetworkNotice === 'function'
      && typeof binary.NativeSession?.prototype?.networkNoticeFence === 'function');
  const requestLifecycle = !features.includes('response-request') || !features.includes('request-events')
    || (typeof binary.NativeSession?.prototype?.requestResponse === 'function'
      && typeof binary.NativeSession?.prototype?.requestFailure === 'function'
      && typeof binary.NativeSession?.prototype?.requestRedirectedFrom === 'function');
  const requestPostDataJson = typeof binary.requestPostDataJsonEnabled === 'function'
    && binary.requestPostDataJsonEnabled();
  const locatorBoundingBox = typeof binary.locatorBoundingBoxEnabled === 'function' && binary.locatorBoundingBoxEnabled();
  const locatorHighlight = typeof binary.locatorHighlightEnabled === 'function' && binary.locatorHighlightEnabled();
  const locatorScreenshot = typeof binary.locatorScreenshotEnabled === 'function' && binary.locatorScreenshotEnabled()
    && typeof binary.NativeLocator?.prototype?.screenshot === 'function';
  const locatorDomActions = typeof binary.locatorDomActionsEnabled === 'function' && binary.locatorDomActionsEnabled()
    && typeof binary.NativeLocator?.prototype?.dispatchEvent === 'function'
    && typeof binary.NativeLocator?.prototype?.selectText === 'function';
  const selectorRegistration = typeof binary.selectorRegistrationEnabled === 'function' && binary.selectorRegistrationEnabled()
    && typeof binary.NativeEnvironment?.prototype?.registerSelectorEngine === 'function'
    && typeof binary.NativeEnvironment?.prototype?.setTestIdAttribute === 'function';
  const largeEvaluationResults = typeof binary.largeEvaluationResultsEnabled === 'function' && binary.largeEvaluationResultsEnabled();
  const headers = typeof binary.extraHeadersEnabled === 'function' && binary.extraHeadersEnabled();
  const close = typeof binary.concurrentCloseEnabled === 'function' && binary.concurrentCloseEnabled();
  const upload = typeof binary.fileInputEnabled === 'function' && binary.fileInputEnabled();
  const handles = typeof binary.handlesEnabled === 'function' && binary.handlesEnabled()
    && typeof binary.NativeLocator?.prototype?.waitForElement === 'function'
    && typeof binary.NativeLocator?.prototype?.waitForSelectorElement === 'function';
  const frameElement = typeof binary.NativeSession?.prototype?.frameElement === 'function';
  if (frameElement !== features.includes('frame-element'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const elementHandleDom = typeof binary.elementHandleDomEnabled === 'function' && binary.elementHandleDomEnabled();
  const elementHandleWaitForSelector = typeof binary.elementHandleWaitForSelectorEnabled === 'function'
    && binary.elementHandleWaitForSelectorEnabled();
  const elementHandleWaitForSelectorBinding = typeof binary.NativeSession?.prototype?.elementHandleWaitForSelector === 'function';
  if (elementHandleWaitForSelector !== features.includes('element-handle-wait-for-selector')
      || elementHandleWaitForSelectorBinding !== elementHandleWaitForSelector)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const elementHandleSelectOptionValue = typeof binary.elementHandleSelectOptionValueEnabled === 'function'
    && binary.elementHandleSelectOptionValueEnabled();
  const elementHandleSelectOptionValueBinding = typeof binary.NativeSession?.prototype?.elementHandleSelectOptionValue === 'function';
  if (elementHandleSelectOptionValue !== features.includes('element-handle-select-option-value')
      || elementHandleSelectOptionValueBinding !== elementHandleSelectOptionValue)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const elementHandleScrollIntoView = typeof binary.elementHandleScrollIntoViewEnabled === 'function'
    && binary.elementHandleScrollIntoViewEnabled()
    && typeof binary.elementHandleScrollTimeoutInheritanceEnabled === 'function'
    && binary.elementHandleScrollTimeoutInheritanceEnabled()
    && typeof binary.NativeSession?.prototype?.elementHandleScrollIntoViewIfNeeded === 'function';
  if (elementHandleScrollIntoView !== features.includes('element-handle-scroll-into-view'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const retained = typeof binary.retainedFileInputEnabled === 'function' && binary.retainedFileInputEnabled();
  const marking = typeof binary.retainedTargetMarkingEnabled === 'function' && binary.retainedTargetMarkingEnabled();
  const prevalidation = typeof binary.retainedTargetPrevalidationEnabled === 'function' && binary.retainedTargetPrevalidationEnabled();
  const instrumentation = typeof binary.retainedInputInstrumentationEnabled === 'function' && binary.retainedInputInstrumentationEnabled();
  const serialization = typeof binary.retainedCallArgumentSerializationEnabled === 'function' && binary.retainedCallArgumentSerializationEnabled();
  const payloadImport = typeof binary.retainedSerializedPayloadImportEnabled === 'function' && binary.retainedSerializedPayloadImportEnabled();
  const payloadMutation = typeof binary.retainedSerializedPayloadMutationEnabled === 'function' && binary.retainedSerializedPayloadMutationEnabled();
  const apiRequest = typeof binary.apiRequestEnabled === 'function' && binary.apiRequestEnabled();
  const apiRequestClientCertificates = typeof binary.apiRequestClientCertificatesEnabled === 'function' && binary.apiRequestClientCertificatesEnabled();
  const apiRequestHttpProxy = typeof binary.apiRequestHttpProxyEnabled === 'function' && binary.apiRequestHttpProxyEnabled();
  const apiRequestSocks5Proxy = typeof binary.apiRequestSocks5ProxyEnabled === 'function' && binary.apiRequestSocks5ProxyEnabled();
  const apiRequestProxyBypass = typeof binary.apiRequestProxyBypassEnabled === 'function' && binary.apiRequestProxyBypassEnabled();
  const contextApiRequest = typeof binary.contextApiRequestEnabled === 'function' && binary.contextApiRequestEnabled();
  const contextProxy = typeof binary.contextProxyEnabled === 'function' && binary.contextProxyEnabled();
  const contextOffline = typeof binary.contextOfflineEnabled === 'function' && binary.contextOfflineEnabled()
    && typeof binary.NativeSession?.prototype?.newContextWithOffline === 'function'
    && typeof binary.NativeSession?.prototype?.setContextOffline === 'function';
  const bypassCsp = typeof binary.bypassCspEnabled === 'function' && binary.bypassCspEnabled()
    && typeof binary.NativeSession?.prototype?.newContextWithBypassCsp === 'function';
  const fillNavigationCommit = typeof binary.fillNavigationCommitEnabled === 'function' && binary.fillNavigationCommitEnabled();
  if (fillNavigationCommit !== features.includes('fill-navigation-commit'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const selectOptionNavigationCommit = typeof binary.selectOptionNavigationCommitEnabled === 'function'
    && binary.selectOptionNavigationCommitEnabled();
  if (selectOptionNavigationCommit !== features.includes('select-option-navigation-commit'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const selectOptionTimeoutProgress = typeof binary.selectOptionTimeoutProgressEnabled === 'function'
    && binary.selectOptionTimeoutProgressEnabled();
  if (selectOptionTimeoutProgress !== features.includes('select-option-timeout-progress'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const reactSelectors = typeof binary.reactSelectorsEnabled === 'function' && binary.reactSelectorsEnabled();
  if (reactSelectors !== features.includes('react-selectors'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  const lightSelectors = typeof binary.lightSelectorsEnabled === 'function' && binary.lightSelectorsEnabled();
  if (lightSelectors !== features.includes('light-selectors'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  if (responseRequest !== features.includes('response-request'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  if (!elementHandleDispatchEvent)
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  if (requestEvents !== features.includes('request-events')
      || responseEvents !== features.includes('response-events')
      || responseFinished !== features.includes('response-finished')
      || requestFrame !== features.includes('request-frame')
      || !orderedNetwork || !requestLifecycle
      || requestPostDataJson !== features.includes('request-post-data-json'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
  if (drag !== features.includes('drag-runtime') || frame !== features.includes('frame-navigation') || historyNavigation !== features.includes('history-navigation') || setContent !== features.includes('set-content') || pageEvents !== features.includes('page-events') || pageDialogs !== features.includes('page-dialogs') || pageRoutes !== features.includes('page-routes') || consoleEvents !== features.includes('console-events') || (consoleEvents && !consoleListenerControls) || pageBindings !== features.includes('page-bindings') || initScripts !== features.includes('init-scripts') || timeoutSettings !== features.includes('timeout-settings') || emulateMedia !== features.includes('emulate-media') || deviceEmulation !== features.includes('device-emulation') || touchscreenInput !== features.includes('touchscreen-input') || (needsElementHandleTap && !elementHandleTap) || pageViewport !== features.includes('page-viewport') || requestGc !== features.includes('request-gc') || responseBody !== features.includes('response-body') || locatorBoundingBox !== features.includes('locator-bounding-box') || locatorHighlight !== features.includes('locator-highlight') || locatorScreenshot !== features.includes('locator-screenshot') || locatorDomActions !== features.includes('locator-dom-actions') || selectorRegistration !== features.includes('selector-registration') || waitForFunction !== features.includes('wait-for-function') || waitForUrl !== features.includes('wait-for-url') || waitForTimeout !== features.includes('wait-for-timeout') || largeEvaluationResults !== features.includes('large-evaluation-results') || headers !== features.includes('extra-headers') || contextOffline !== features.includes('context-offline') || bypassCsp !== features.includes('bypass-csp') || close !== features.includes('concurrent-close') || upload !== features.includes('file-input') || handles !== features.includes('handles') || elementHandleDom !== features.includes('element-handle-dom') || retained !== features.includes('retained-file-input') || marking !== features.includes('retained-target-marking') || prevalidation !== features.includes('retained-target-prevalidation') || instrumentation !== features.includes('retained-input-instrumentation') || serialization !== features.includes('retained-call-argument-serialization') || payloadImport !== features.includes('retained-serialized-payload-import') || payloadMutation !== features.includes('retained-serialized-payload-mutation') || apiRequest !== features.includes('api-request') || apiRequestClientCertificates !== features.includes('api-request-client-certificates') || apiRequestHttpProxy !== features.includes('api-request-http-proxy') || apiRequestSocks5Proxy !== features.includes('api-request-socks5-proxy') || apiRequestProxyBypass !== features.includes('api-request-proxy-bypass') || contextApiRequest !== features.includes('context-api-request') || contextProxy !== features.includes('context-proxy'))
    throw new Error(`Pwright native profile mismatch: expected ${selection.profile}. Rebuild this profile explicitly.`);
}
module.exports = { nativeAbiVersion, nativeArtifact, resolveNativeArtifact, verifyNativeArtifact, nativeFeatures };
