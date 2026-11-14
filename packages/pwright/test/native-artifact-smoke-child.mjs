import { writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const [artifact, profile] = process.argv.slice(2);
const {
  resolveNativeArtifact,
  verifyNativeArtifact,
} = require('../src/native-artifact.cjs');
const addon = require(artifact);
verifyNativeArtifact(addon, resolveNativeArtifact(profile));
writeFileSync(process.stdout.fd, JSON.stringify({
  abi: addon.nativeAbiVersion(),
  environment: typeof addon.NativeEnvironment,
  profile,
  traceDirEnabled: typeof addon.playwrightTraceDirEnabled === 'function'
    && addon.playwrightTraceDirEnabled(),
  burst20Enabled: typeof addon.frameNavigationBurst20Enabled === 'function'
    && addon.frameNavigationBurst20Enabled(),
  fillNavigationCommitEnabled: typeof addon.fillNavigationCommitEnabled === 'function'
    && addon.fillNavigationCommitEnabled(),
  routeRequestHistoryEnabled: typeof addon.routeRequestHistoryEnabled === 'function'
    && addon.routeRequestHistoryEnabled(),
  contextCookiesEnabled: typeof addon.contextCookiesEnabled === 'function'
    && addon.contextCookiesEnabled(),
  largeEvaluationInputsEnabled: typeof addon.largeEvaluationInputsEnabled === 'function'
    && addon.largeEvaluationInputsEnabled(),
  largeRoutePostDataEnabled: typeof addon.largeRoutePostDataEnabled === 'function'
    && addon.largeRoutePostDataEnabled(),
  elementHandleScrollIntoViewEnabled: typeof addon.elementHandleScrollIntoViewEnabled === 'function'
    && addon.elementHandleScrollIntoViewEnabled(),
  elementHandleWaitForSelectorEnabled: typeof addon.elementHandleWaitForSelectorEnabled === 'function'
    && addon.elementHandleWaitForSelectorEnabled(),
  elementHandleSelectOptionValueEnabled: typeof addon.elementHandleSelectOptionValueEnabled === 'function'
    && addon.elementHandleSelectOptionValueEnabled(),
  lightSelectorsEnabled: typeof addon.lightSelectorsEnabled === 'function'
    && addon.lightSelectorsEnabled(),
  frameEventsEnabled: typeof addon.frameEventsEnabled === 'function'
    && addon.frameEventsEnabled(),
  cdpSessionLease: typeof addon.NativeCdpSessionLease,
  cdpEventBatch: typeof addon.NativeCdpEventBatch,
  methods: {
    cdpSessionCreate: typeof addon.NativeSession.prototype.cdpSessionCreate,
    cdpBrowserSessionCreate: typeof addon.NativeSession.prototype.cdpBrowserSessionCreate,
    cdpSessionSend: typeof addon.NativeSession.prototype.cdpSessionSend,
    cdpSessionClaim: typeof addon.NativeSession.prototype.cdpSessionClaim,
    cdpSessionDetach: typeof addon.NativeSession.prototype.cdpSessionDetach,
    nextCdpSessionEventGeneration: typeof addon.NativeSession.prototype.nextCdpSessionEventGeneration,
    cdpReportCallbackException: typeof addon.NativeSession.prototype.reportCallbackException,
    cdpLeaseToken: typeof addon.NativeCdpSessionLease?.prototype?.token,
    cdpLeaseClaim: typeof addon.NativeCdpSessionLease?.prototype?.claim,
    cdpLeaseAbort: typeof addon.NativeCdpSessionLease?.prototype?.abort,
    cdpBatchPeek: typeof addon.NativeCdpEventBatch?.prototype?.peek,
    cdpBatchAckNext: typeof addon.NativeCdpEventBatch?.prototype?.ackNext,
    cdpBatchRollback: typeof addon.NativeCdpEventBatch?.prototype?.rollback,
    launchChromiumWithTracesDir: typeof addon.NativeEnvironment.prototype.launchChromiumWithTracesDir,
    contextRequest: typeof addon.NativeSession.prototype.contextRequest,
    contextAddCookies: typeof addon.NativeSession.prototype.contextAddCookies,
    pageGoBack: typeof addon.NativeSession.prototype.pageGoBack,
    pageGoForward: typeof addon.NativeSession.prototype.pageGoForward,
    newContextWithHasTouch: typeof addon.NativeSession.prototype.newContextWithHasTouch,
    touchscreenTap: typeof addon.NativeSession.prototype.touchscreenTap,
    elementHandleTap: typeof addon.NativeSession.prototype.elementHandleTap,
    elementHandleScrollIntoViewIfNeeded: typeof addon.NativeSession.prototype.elementHandleScrollIntoViewIfNeeded,
    elementHandleWaitForSelector: typeof addon.NativeSession.prototype.elementHandleWaitForSelector,
    elementHandleSelectOptionValue: typeof addon.NativeSession.prototype.elementHandleSelectOptionValue,
    frameElement: typeof addon.NativeSession.prototype.frameElement,
    locatorWaitForElement: typeof addon.NativeLocator.prototype.waitForElement,
    locatorWaitForSelectorElement: typeof addon.NativeLocator.prototype.waitForSelectorElement,
    ...(['official-conformance-page-cdp-session', 'official-conformance-page-dispatch-event', 'official-conformance-page-goto-burst-20', 'official-conformance-page-goto-frame-detach', 'official-conformance-page-route-header-url', 'official-conformance-page-route-header-url-unroute', 'official-conformance-page-fill-navigation-commit'].includes(profile)
      ? { elementHandleDispatchEvent: typeof addon.NativeSession.prototype.elementHandleDispatchEvent }
      : {}),
    pageRouteContinueWithHeaders: typeof addon.NativeSession.prototype.pageRouteContinueWithHeaders,
    pageRouteFallbackWithUrl: typeof addon.NativeSession.prototype.pageRouteFallbackWithUrl,
    pageUnroute: typeof addon.NativeSession.prototype.pageUnroute,
    responseRawHeaders: typeof addon.NativeSession.prototype.responseRawHeaders,
    newContextWithProxy: typeof addon.NativeSession.prototype.newContextWithProxy,
  },
}));
process.exit(0);
