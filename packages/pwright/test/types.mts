import { chromium, firefox, webkit, selectors, errors, compatibility, type Browser, type BrowserContext, type Page, type Response, type Locator, type FrameLocator, type Route } from '@shuozeli/pwright';

const firefoxName: 'firefox' = firefox.name();
const webkitName: 'webkit' = webkit.name();
const expectedChromiumPath: string = chromium.executablePath();
const expectedFirefoxPath: string = firefox.executablePath();
const expectedWebkitPath: string = webkit.executablePath();
const firefoxCdp: Promise<Browser> = firefox.connectOverCDP({ endpointURL: 'ws://foo' });
void firefoxName; void webkitName; void firefoxCdp;
void expectedChromiumPath; void expectedFirefoxPath; void expectedWebkitPath;
const launchServerUnsupported: Promise<never> = chromium.launchServer({ channel: 'chrome' });
const ownedLaunch: Promise<Browser> = chromium.launch({ executablePath: '/absolute/chromium', headless: true, timeout: 5000 });
const noSigintHandler: Promise<Browser> = chromium.launch({ executablePath: '/absolute/chromium', handleSIGINT: false });
const traceDirectoryLaunch: Promise<Browser> = chromium.launch({ executablePath: '/absolute/chromium', tracesDir: 'relative/traces' });
const persistentPreflightOnly: Promise<never> = webkit.launchPersistentContext('foo', {
  port: 1234,
  args: ['http://example.test/page'],
  ignoreDefaultArgs: [],
  clientCertificates: [{ origin: 'test' }],
});
void launchServerUnsupported; void ownedLaunch; void noSigintHandler; void traceDirectoryLaunch; void persistentPreflightOnly;
// @ts-expect-error fresh launch requires an explicit executable and excludes persistent profile options
chromium.launch({ userDataDir: 'random-path', port: 1234 });
// @ts-expect-error Firefox launch has no implementation
firefox.launch({ executablePath: '/absolute/firefox' });
// @ts-expect-error launch arguments are outside the narrow owned capability
chromium.launch({ executablePath: '/absolute/chromium', args: ['--no-sandbox'] });
// @ts-expect-error installing a SIGINT handler is outside the owned profile
chromium.launch({ executablePath: '/absolute/chromium', handleSIGINT: true });
// @ts-expect-error trace directory must be a string
chromium.launch({ executablePath: '/absolute/chromium', tracesDir: 42 });
// @ts-expect-error launchServer channel must be a string
chromium.launchServer({ channel: null });

const dispatchEventProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-dispatch-event';
const frameElementProfile: import('@shuozeli/pwright').NativeProfile = 'frame-element';
const frameElementConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-frame-element';
const browserRegistryProfile: import('@shuozeli/pwright').NativeProfile = 'browser-registry';
const browserCdpSessionProfile: import('@shuozeli/pwright').NativeProfile = 'browser-cdp-session';
const cdpHandshakeHeadersProfile: import('@shuozeli/pwright').NativeProfile = 'cdp-handshake-headers';
const cdpHandshakeHeadersConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-cdp-handshake-headers';
const cdpDefaultUserAgentProfile: import('@shuozeli/pwright').NativeProfile = 'cdp-default-user-agent';
const cdpDefaultUserAgentConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-cdp-default-user-agent';
const playwrightTraceProfile: import('@shuozeli/pwright').NativeProfile = 'playwright-trace';
const frameNavigationBurstProfile: import('@shuozeli/pwright').NativeProfile = 'frame-navigation-burst-20';
const pageGotoBurstProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-goto-burst-20';
const routeOverrideProfile: import('@shuozeli/pwright').NativeProfile = 'route-header-url-overrides';
const routeConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url';
const unrouteProfile: import('@shuozeli/pwright').NativeProfile = 'page-unroute';
const routeUnrouteConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url-unroute';
const routeRegExpProfile: import('@shuozeli/pwright').NativeProfile = 'route-regexp';
const routeRegExpConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url-unroute-cors-regexp';
const routeHistoryProfile: import('@shuozeli/pwright').NativeProfile = 'route-request-history';
const routeHistoryConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url-unroute-cors-regexp-history';
const cookiesProfile: import('@shuozeli/pwright').NativeProfile = 'context-cookies';
const cookiesConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies';
const largeRoutePostDataProfile: import('@shuozeli/pwright').NativeProfile = 'route-large-post-data';
const largePostDataConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data';
const fillNavigationCommitProfile: import('@shuozeli/pwright').NativeProfile = 'fill-navigation-commit';
const fillNavigationCommitConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-fill-navigation-commit';
const selectOptionTimeoutProfile: import('@shuozeli/pwright').NativeProfile = 'select-option-timeout-progress';
const selectOptionTimeoutConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-select-option-timeout-progress';
const selectOptionNavigationCommitProfile: import('@shuozeli/pwright').NativeProfile = 'select-option-navigation-commit';
const selectOptionNavigationCommitConformanceProfile: import('@shuozeli/pwright').NativeProfile = 'official-conformance-page-select-option-navigation-commit';
void dispatchEventProfile; void frameElementProfile; void frameElementConformanceProfile; void browserRegistryProfile; void browserCdpSessionProfile; void cdpHandshakeHeadersProfile; void cdpHandshakeHeadersConformanceProfile; void playwrightTraceProfile; void frameNavigationBurstProfile; void pageGotoBurstProfile; void routeOverrideProfile; void routeConformanceProfile; void unrouteProfile; void routeUnrouteConformanceProfile; void routeRegExpProfile; void routeRegExpConformanceProfile; void routeHistoryProfile; void routeHistoryConformanceProfile; void largePostDataConformanceProfile; void fillNavigationCommitProfile; void fillNavigationCommitConformanceProfile;
void selectOptionNavigationCommitProfile; void selectOptionNavigationCommitConformanceProfile;

const selectorRegistration: Promise<void> = selectors.register('tag', () => ({
  query(root: Document | Element | ShadowRoot, body: string) { return root.querySelector(body); },
  queryAll(root: Document | Element | ShadowRoot, body: string) { return [...root.querySelectorAll(body)]; },
}), { contentScript: true });
selectors.setTestIdAttribute('data-pwright-id');
void selectorRegistration;
// @ts-expect-error contentScript is a boolean
selectors.register('tag2', '({ query() {}, queryAll() { return []; } })', { contentScript: 'yes' });

const byUrl: Promise<Browser> = chromium.connectOverCDP('http://127.0.0.1:9222', { timeout: 5000 });
const byOptions: Promise<Browser> = chromium.connectOverCDP({ endpointURL: 'http://127.0.0.1:9222' });
const byWsAlias: Promise<Browser> = chromium.connectOverCDP({ wsEndpoint: 'ws://example.test/ws', headers: { foo: 'bar' } });
const byHeaders: Promise<Browser> = chromium.connectOverCDP('http://127.0.0.1:9222', { headers: { 'User-Agent': 'Playwright' } });
void byUrl; void byOptions; void byWsAlias; void byHeaders;
declare const browser: Browser;
const browserCdpSession: Promise<import('@shuozeli/pwright').CDPSession> = browser.newBrowserCDPSession();
void browserCdpSession;
const configuredContext: Promise<BrowserContext> = browser.newContext({ baseURL: 'https://example.com/path/' });
const touchContext: Promise<BrowserContext> = browser.newContext({ hasTouch: true });
const implicitPage: Promise<Page> = browser.newPage({ baseURL: 'https://example.com/path/' });
const downloadRejectingImplicitPage: Promise<Page> = browser.newPage({ acceptDownloads: false });
void implicitPage; void downloadRejectingImplicitPage;
// @ts-expect-error Browser.newPage currently exposes only the Rust-owned baseURL/acceptDownloads=false slice
browser.newPage({ locale: 'fr-FR' });
// @ts-expect-error download acceptance requires the independent download-events native profile
browser.newPage({ acceptDownloads: true });
const configuredHeaders: Promise<BrowserContext> = browser.newContext({ extraHTTPHeaders: { 'X-Layer': 'creation' } });
const defaultFixtureContext: Promise<BrowserContext> = browser.newContext({
  acceptDownloads: true, bypassCSP: false, colorScheme: 'light', hasTouch: false,
  ignoreHTTPSErrors: false, isMobile: false, javaScriptEnabled: true, locale: 'en-US',
  offline: false, serviceWorkers: 'allow', viewport: { width: 1280, height: 720 },
});
const bypassedContext: Promise<BrowserContext> = browser.newContext({ bypassCSP: true });
void defaultFixtureContext;
void configuredHeaders;
// @ts-expect-error creation header values must be strings
browser.newContext({ extraHTTPHeaders: { 'X-Layer': 1 } });
const contextMetadataBytes: number = browser.pwrightContextMetadataMetrics().bytes;
void configuredContext; void touchContext; void contextMetadataBytes;
// @ts-expect-error baseURL must be a string
browser.newContext({ baseURL: 42 });
// @ts-expect-error nondefault viewport requires an additional capability
browser.newContext({ viewport: { width: 800, height: 600 } });
const page: Page = browser.contexts()[0].pages()[0];
const frameOwnerElement: Promise<import('@shuozeli/pwright').ElementHandle<HTMLIFrameElement | HTMLFrameElement>> = page.mainFrame().frameElement();
void frameOwnerElement;
const cdpPage: Promise<import('@shuozeli/pwright').CDPSession> = page.context().newCDPSession(page);
const cdpMainFrame: Promise<import('@shuozeli/pwright').CDPSession> = page.context().newCDPSession(page.mainFrame());
void cdpPage; void cdpMainFrame;
// @ts-expect-error CDPSession targets must be a Page or Frame
page.context().newCDPSession(page.context());
declare const cdpSession: import('@shuozeli/pwright').CDPSession;
const cdpResult: Promise<Record<string, unknown>> = cdpSession.send('Runtime.evaluate', { expression: '1 + 2' });
const cdpDetached: Promise<void> = cdpSession.detach();
const cdpListener = (_params: Record<string, unknown>) => {};
cdpSession.on('Runtime.consoleAPICalled', cdpListener);
cdpSession.once('Runtime.consoleAPICalled', cdpListener);
cdpSession.removeListener('Runtime.consoleAPICalled', cdpListener);
cdpSession.on('event', event => { const method: string = event.method; void method; });
cdpSession.on('close', session => { const detached: Promise<void> = session.detach(); void detached; });
void cdpResult; void cdpDetached;
// @ts-expect-error CDP params must be an object
cdpSession.send('Runtime.evaluate', 'not an object');
const pageSelectorWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement> | null> =
  page.waitForSelector('button', { state: 'visible', strict: true, timeout: 500 });
const pageButtonWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLButtonElement>> =
  page.waitForSelector('button');
const frameVisibleWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement>> =
  page.mainFrame().waitForSelector('.ready', { state: 'attached' });
const frameSelectorWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement> | null> =
  page.mainFrame().waitForSelector('button', { state: 'hidden', timeout: 0 });
void pageSelectorWait; void pageButtonWait; void frameVisibleWait; void frameSelectorWait;
declare const rootHandle: import('@shuozeli/pwright').ElementHandle;
const scopedSpanWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLSpanElement>> =
  rootHandle.waitForSelector('span', { state: 'attached' });
const scopedHiddenWait: Promise<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement> | null> =
  rootHandle.waitForSelector('.gone', { state: 'hidden', strict: true, timeout: 0 });
void scopedSpanWait; void scopedHiddenWait;
// @ts-expect-error scoped wait state must be one of Playwright's four selector states
rootHandle.waitForSelector('span', { state: 'ready' });
// @ts-expect-error wait state must be one of Playwright's four selector states
page.waitForSelector('button', { state: 'ready' });
// @ts-expect-error strict selector mode must be boolean
page.mainFrame().waitForSelector('button', { strict: 'yes' });
const pageRouteHandler = (_route: Route) => {};
const pageUnrouteAll: Promise<void> = page.unroute('**/*.css');
const pageUnrouteHandler: Promise<void> = page.unroute('**/*.css', pageRouteHandler);
const pageRegExpRoute: Promise<void> = page.route(/\.css$/, pageRouteHandler);
const pageRegExpUnroute: Promise<void> = page.unroute(/\.css$/, pageRouteHandler);
void pageUnrouteAll; void pageUnrouteHandler; void pageRegExpRoute; void pageRegExpUnroute;
// @ts-expect-error unroute handler must be callable
page.unroute('**/*.css', 42);
const pageSelectorClick: Promise<void> = page.click('button', { strict: true, clickCount: 3 });
const frameSelectorClick: Promise<void> = page.mainFrame().click('button', { strict: false, trial: true });
const pageSelectorDblclick: Promise<void> = page.dblclick('button', { strict: true, trial: true });
const frameSelectorDblclick: Promise<void> = page.mainFrame().dblclick('button', { strict: false });
void pageSelectorClick; void frameSelectorClick; void pageSelectorDblclick; void frameSelectorDblclick;
// @ts-expect-error selector-based strict mode requires a Boolean
page.click('button', { strict: 'yes' });
// @ts-expect-error dblclick does not accept clickCount
page.dblclick('button', { clickCount: 3 });
declare const route: Route;
const routeFulfill: Promise<void> = route.fulfill({
  status: 201,
  headers: { 'x-test': 'yes' },
  contentType: 'application/json',
  json: { answer: 42 },
});
void routeFulfill;
const routeContinueHeaders: Promise<void> = route.continue({ headers: { 'x-test': 'yes' } });
const routeFallbackUrl: Promise<void> = route.fallback({ url: 'https://example.test/bar' });
void routeContinueHeaders; void routeFallbackUrl;
// @ts-expect-error Route.continue header values must be strings
route.continue({ headers: { 'x-test': 42 } });
// @ts-expect-error Route.continue URL overrides are not in this selected capability
route.continue({ url: 'https://example.test/bar' });
// @ts-expect-error Route.fallback URLs must be strings
route.fallback({ url: 42 });
// @ts-expect-error Route.fulfill header values must be strings
route.fulfill({ headers: { 'x-test': 42 } });
const pageRequest: import('@shuozeli/pwright').APIRequestContext = page.request;
const contextRequest: import('@shuozeli/pwright').APIRequestContext = page.context().request;
const contextCookies: Promise<void> = page.context().addCookies([{ url: 'https://example.test/', name: 'session', value: 'value', sameSite: 'Lax' }]);
void pageRequest; void contextRequest;
const contextClose: Promise<void> = page.context().close({ reason: 'Test ended.' });
void contextClose;
const objectHandle: Promise<import('@shuozeli/pwright').JSHandle<{ answer: number }>> = page.evaluateHandle(() => ({ answer: 42 }));
const elementHandle: Promise<import('@shuozeli/pwright').ElementHandle<HTMLBodyElement>> = page.mainFrame().evaluateHandle(() => document.body as HTMLBodyElement);
const pageElements: Promise<Array<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement>>> = page.$$('div');
const frameElements: Promise<Array<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement>>> = page.mainFrame().$$('div');
void objectHandle; void elementHandle; void pageElements; void frameElements;
declare const retainedObject: import('@shuozeli/pwright').JSHandle<{ answer: number }>;
declare const retainedNode: import('@shuozeli/pwright').ElementHandle<HTMLDivElement>;
const retainedOwnerFrame: Promise<import('@shuozeli/pwright').Frame | null> = retainedNode.ownerFrame();
const retainedAttribute: Promise<string | null> = retainedNode.getAttribute('data-name');
const retainedHtml: Promise<string> = retainedNode.innerHTML();
const retainedInputValue: Promise<string> = retainedNode.inputValue();
const retainedVisible: Promise<boolean> = retainedNode.isVisible();
const retainedHidden: Promise<boolean> = retainedNode.isHidden();
const retainedEnabled: Promise<boolean> = retainedNode.isEnabled();
const retainedDisabled: Promise<boolean> = retainedNode.isDisabled();
const retainedEditable: Promise<boolean> = retainedNode.isEditable();
const retainedChecked: Promise<boolean> = retainedNode.isChecked();
const retainedBox: Promise<import('@shuozeli/pwright').BoundingBox | null> = retainedNode.boundingBox();
const retainedFocus: Promise<void> = retainedNode.focus();
const retainedScroll: Promise<void> = retainedNode.scrollIntoViewIfNeeded({ timeout: 500 });
const retainedEvent: Promise<void> = retainedNode.dispatchEvent('click', { detail: 2 });
const retainedTap: Promise<void> = retainedNode.tap({ trial: true, timeout: 500 });
void retainedOwnerFrame;
void retainedAttribute; void retainedHtml; void retainedInputValue;
void retainedVisible; void retainedHidden; void retainedEnabled; void retainedDisabled; void retainedEditable; void retainedChecked; void retainedBox; void retainedFocus; void retainedScroll; void retainedEvent; void retainedTap;
// @ts-expect-error primitive/object JSHandle has no ElementHandle owner-frame method
retainedObject.ownerFrame();
// @ts-expect-error primitive/object JSHandle has no DOM event target
retainedObject.dispatchEvent('click');
const unboxedAnswer: Promise<number> = page.evaluate(value => value.nested.answer + value.list[0].answer, { nested: retainedObject, list: [retainedObject] });
const unboxedNode: Promise<string | null> = page.evaluate(node => node.textContent, retainedNode);
const unboxedOther: Promise<number> = retainedObject.evaluate((value, arg) => value.answer + arg.other.answer, { other: retainedObject });
void unboxedAnswer; void unboxedNode; void unboxedOther;
const retainedExpression: Promise<number> = retainedObject.evaluate<number>('42', { retainedObject });
const pageExpression: Promise<number> = page.evaluate<number, typeof retainedObject>('42', retainedObject);
void retainedExpression; void pageExpression;
const producedObject: Promise<import('@shuozeli/pwright').JSHandle<{ answer: number }>> = retainedObject.evaluateHandle(value => value);
const producedElement: Promise<import('@shuozeli/pwright').ElementHandle<HTMLDivElement>> = retainedObject.evaluateHandle((_value, node) => node, retainedNode);
const retainedProperty: Promise<import('@shuozeli/pwright').JSHandle<number>> = retainedObject.getProperty('answer');
void producedObject; void producedElement; void retainedProperty;
declare const promisePropertySource: import('@shuozeli/pwright').JSHandle<{ value: Promise<number> }>;
const promiseProperty: Promise<import('@shuozeli/pwright').JSHandle<Promise<number>>> = promisePropertySource.getProperty('value');
void promiseProperty;
const retainedProperties: Promise<Map<string, import('@shuozeli/pwright').JSHandle<unknown>>> = retainedObject.getProperties();
const elementProperties: Promise<Map<string, import('@shuozeli/pwright').JSHandle<unknown>>> = retainedNode.getProperties();
void retainedProperties; void elementProperties;
// @ts-expect-error property enumeration has no selector or options argument
retainedObject.getProperties('answer');
const pageClosed: boolean = page.isClosed();
void pageClosed;
const pageHeaders: Promise<void> = page.setExtraHTTPHeaders({ 'X-Layer': 'page' });
const contextHeaders: Promise<void> = page.context().setExtraHTTPHeaders({ 'X-Layer': 'context' });
void pageHeaders; void contextHeaders;
// @ts-expect-error header values must be strings
page.setExtraHTTPHeaders({ 'X-Layer': 1 });
// @ts-expect-error header values must be strings
page.context().setExtraHTTPHeaders({ 'X-Layer': false });
const identical: Page = page.mainFrame().page();
const frameParent: import('@shuozeli/pwright').Frame | null = page.frames()[0].parentFrame();
const frameChildren: import('@shuozeli/pwright').Frame[] = page.mainFrame().childFrames();
const frameDetached: boolean = page.mainFrame().isDetached();
const frameName: string = page.mainFrame().name();
const namedFrame: import('@shuozeli/pwright').Frame | null = page.frame('child');
const urlFrame = page.frame({ url: '**/child.html' });
const regexFrame = page.frame({ url: /child/ });
const predicateFrame = page.frame({ url: url => url.pathname === '/child.html' });
void namedFrame; void urlFrame; void regexFrame; void predicateFrame;
// @ts-expect-error Frame URL matcher must have a declared supported shape
page.frame({ url: 3 });
void frameName;
void frameParent; void frameChildren; void frameDetached;
const connected: boolean = browser.isConnected();
const queuedNativeResults: number = browser.pwrightFfiMetrics().queued;
const reservedNativeBytes: number = browser.pwrightFfiMetrics().reservedBytes;
const unpublishedUrls: number = browser.pwrightMetadataMetrics().pendingPublications;
const unpublishedFrame: boolean = page.mainFrame().pwrightMetadataMetrics().unpublishedUrl;
void queuedNativeResults; void reservedNativeBytes; void unpublishedUrls; void unpublishedFrame;
const pageUrl: string = page.url();
const pageInnerText: Promise<string> = page.innerText('#content', { timeout: 1000 });
const frameInnerText: Promise<string> = page.mainFrame().innerText('#content');
const pageTextContent: Promise<string | null> = page.textContent('#content', { timeout: 1000 });
const frameTextContent: Promise<string | null> = page.mainFrame().textContent('#content');
const pageAttribute: Promise<string | null> = page.getAttribute('#content', 'data-name');
const frameAttribute: Promise<string | null> = page.mainFrame().getAttribute('#content', 'data-name');
const pageHtml: Promise<string> = page.innerHTML('#content');
const frameInputValue: Promise<string> = page.mainFrame().inputValue('#input');
const pageEnabled: Promise<boolean> = page.isEnabled('#input');
const frameChecked: Promise<boolean> = page.mainFrame().isChecked('#check');
const pageFill: Promise<void> = page.fill('#input', 'value', { force: true });
const frameSelection: Promise<string[]> = page.mainFrame().selectOption('#select', { value: 'a' });
declare const retainedOption: import('@shuozeli/pwright').ElementHandle<HTMLOptionElement>;
declare const optionalOption: import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement> | null;
const pageSelectionByHandle: Promise<string[]> = page.selectOption('#select', retainedOption);
const pageSelectionByOptionalHandle: Promise<string[]> = page.selectOption('#select', optionalOption);
const frameSelectionByHandle: Promise<string[]> = page.mainFrame().selectOption('#select', retainedOption);
const retainedSelectionByValue: Promise<string[]> = retainedOption.selectOption('blue', { timeout: 1000 });
// @ts-expect-error Retained ElementHandle option descriptors are not yet selected.
retainedOption.selectOption({ value: 'blue' });
void retainedSelectionByValue;
const screenshot: Promise<Buffer> = page.screenshot({ type: 'jpeg', quality: 80, fullPage: true, timeout: 5000 });
void pageInnerText; void frameInnerText; void pageTextContent; void frameTextContent; void screenshot;
void pageAttribute; void frameAttribute; void pageHtml; void frameInputValue;
void pageEnabled; void frameChecked; void pageFill; void frameSelection;
void pageSelectionByHandle; void pageSelectionByOptionalHandle; void frameSelectionByHandle;
// @ts-expect-error Playwright public screenshots do not expose CDP WebP
page.screenshot({ type: 'webp' });
const frameUrl: string = page.mainFrame().url();
const pageUrlWait: Promise<void> = page.waitForURL('**/ready.html', { waitUntil: 'domcontentloaded', timeout: 1000 });
const frameUrlWait: Promise<void> = page.mainFrame().waitForURL(/ready\.html/);
const predicateUrlWait: Promise<void> = page.waitForURL(url => url.pathname === '/ready.html');
const requestWait: Promise<import('@shuozeli/pwright').Request> = page.waitForRequest(
  request => request.method() === 'POST',
  { timeout: 1000 },
);
const responseWait: Promise<Response> = page.waitForResponse(
  response => response.status() === 200,
  { timeout: 1000 },
);
const responseFinished: Promise<null> = (await responseWait).finished();
void responseFinished;
void pageUrlWait; void frameUrlWait; void predicateUrlWait; void requestWait; void responseWait;
// @ts-expect-error URL matcher must be a string, RegExp, or predicate
page.waitForURL(42);
const keyboardDown: Promise<void> = page.keyboard.down('Shift');
const keyboardUp: Promise<void> = page.keyboard.up('Shift');
const keyboardPress: Promise<void> = page.keyboard.press('KeyA', { delay: 0.5 });
const keyboardInsert: Promise<void> = page.keyboard.insertText('你好');
const keyboardType: Promise<void> = page.keyboard.type('hello', { delay: 10 });
void keyboardDown; void keyboardUp; void keyboardPress; void keyboardInsert; void keyboardType;
// @ts-expect-error Keyboard has no Locator timeout option
page.keyboard.press('A', { timeout: 100 });
const locator: Locator = page.locator('button', { hasText: /^save$/i }).filter({ hasNotText: /Cancel/ }).first();
const locatorNativeElement: Promise<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement>> = locator.elementHandle({ timeout: 5000 });
const locatorNativeElements: Promise<Array<import('@shuozeli/pwright').ElementHandle<HTMLElement | SVGElement>>> = locator.elementHandles();
const locatorEvaluatedHandle: Promise<import('@shuozeli/pwright').JSHandle<Text>> = locator.evaluateHandle(element => element.firstChild as Text);
const describedLocator: Locator = locator.describe('Primary action');
const locatorDescription: string | null = describedLocator.description();
const locatorDisplay: string = describedLocator.toString();
void locatorNativeElement; void locatorNativeElements; void locatorEvaluatedHandle;
void describedLocator; void locatorDescription; void locatorDisplay;
const locatorCount: Promise<number> = locator.count();
const locatorText: Promise<string | null> = locator.textContent({ timeout: 1000 });
const locatorTexts: Promise<string[]> = locator.allTextContents();
const locatorAttribute: Promise<string | null> = locator.getAttribute('missing');
const locatorState: Promise<boolean> = locator.isVisible();
const click: Promise<void> = locator.click({ modifiers: ['ControlOrMeta'], position: { x: 2, y: 3 }, trial: true });
const clickWithSteps: Promise<void> = locator.click({ steps: 5 });
void clickWithSteps;
// @ts-expect-error movement steps are currently opt-in only for Locator.click
locator.dblclick({ steps: 5 });
// @ts-expect-error Page selector clicks do not yet expose movement steps
page.click('button', { steps: 5 });
const locatorTap: Promise<void> = locator.tap({ modifiers: ['Alt'], position: { x: 2, y: 3 }, trial: true });
const pageTap: Promise<void> = page.tap('button', { force: true });
const frameTap: Promise<void> = page.mainFrame().tap('button', { timeout: 500 });
const fill: Promise<void> = locator.fill('value', { force: true });
const press: Promise<void> = locator.press('Enter', { delay: 10 });
const selected: Promise<string[]> = locator.selectOption([{ value: 'a' }, { index: 1 }]);
const locatorSelectionByHandle: Promise<string[]> = locator.selectOption(retainedOption);
void locatorSelectionByHandle;
const checked: Promise<void> = locator.setChecked(true);
const dispatched: Promise<void> = locator.dispatchEvent('click', { detail: 2 }, { timeout: 73 });
const selectedText: Promise<void> = locator.selectText({ force: true, timeout: 73 });
const locatorScreenshot: Promise<Buffer> = locator.screenshot({ type: 'png', scale: 'css', timeout: 73 });
const resized: Promise<void> = page.setViewportSize({ width: 500, height: 500 });
const uploadPath: Promise<void> = locator.setInputFiles('fixture.txt', { noWaitAfter: true });
const uploadPayload: Promise<void> = locator.setInputFiles({ name: 'binary.bin', mimeType: 'application/octet-stream', buffer: Buffer.from([0, 255]) });
const clearFiles: Promise<void> = locator.setInputFiles([]);
const pageUpload: Promise<void> = page.setInputFiles('input', 'fixture.txt', { strict: false });
const frameUpload: Promise<void> = page.mainFrame().setInputFiles('input', [], { strict: true, noWaitAfter: true });
void pageUpload; void frameUpload;
// @ts-expect-error strict is boolean on selector-taking upload APIs
page.setInputFiles('input', [], { strict: 'true' });
// @ts-expect-error locator uploads are strict and have no strict option
locator.setInputFiles([], { strict: false });
void uploadPath; void uploadPayload; void clearFiles;
// @ts-expect-error file arrays must be homogeneous
locator.setInputFiles(['a.txt', { name: 'b.txt', mimeType: 'text/plain', buffer: Buffer.alloc(0) }]);
// @ts-expect-error payload bytes require a Buffer
locator.setInputFiles({ name: 'a.txt', mimeType: 'text/plain', buffer: 'text' });
void click; void locatorTap; void pageTap; void frameTap; void fill; void press; void selected; void checked; void dispatched; void selectedText; void locatorScreenshot; void resized;
// @ts-expect-error Playwright selection arrays are homogeneous
locator.selectOption(['a', { index: 1 }]);
const nested: Locator = page.mainFrame().locator('form').locator(locator);
const contentFrame: FrameLocator = page.locator('iframe').contentFrame();
const nestedFrame: FrameLocator = page.locator('body').frameLocator('iframe').frameLocator('iframe').nth(0);
const frameSemantic: Locator = contentFrame.getByRole('button', { name: /save/i });
const frameOwner: Locator = nestedFrame.owner();
const frameLocatorArgument: Locator = contentFrame.locator(page.getByText('Save'));
void contentFrame; void nestedFrame; void frameSemantic; void frameOwner; void frameLocatorArgument;
// @ts-expect-error FrameLocator indexing requires a number
contentFrame.nth('0');
// @ts-expect-error frameLocator requires a selector string
locator.frameLocator(page.locator('iframe'));
const semanticQuery: Locator = page.getByRole('button', { name: /save/i, pressed: false }).getByText('Save', { exact: true });
const labelQuery: Locator = page.mainFrame().getByLabel('Email').or(page.getByPlaceholder(/email/));
const attributeQuery: Locator = locator.getByTitle(/details/).or(page.getByAltText('cat')).or(page.getByTestId('id'));
void semanticQuery; void labelQuery; void attributeQuery;
// @ts-expect-error role is a declared ARIA role
page.getByRole('custom-role');
// @ts-expect-error exact is a boolean
page.getByText('text', { exact: 'yes' });
const queryBytes: number = browser.pwrightLocatorMetrics().bytes;
void locatorCount; void locatorText; void locatorTexts; void locatorAttribute; void locatorState; void nested; void queryBytes;
// @ts-expect-error Locator composition requires a Locator, not a selector string
locator.and('button');
// @ts-expect-error selector is required
page.locator();
void pageUrl; void frameUrl;
const error: Error = new errors.TimeoutError('test');
const ownership: 'external' | 'external-and-owned' = compatibility.browserOwnership;
const title: Promise<string> = page.title();
const mouseMove: Promise<void> = page.mouse.move(1.5, 2, { steps: 3 });
const mouseDown: Promise<void> = page.mouse.down({ button: 'right', clickCount: 2 });
const mouseUp: Promise<void> = page.mouse.up();
const mouseClick: Promise<void> = page.mouse.click(1, 2, { delay: 0.5, clickCount: 3 });
const mouseDouble: Promise<void> = page.mouse.dblclick(1, 2, { button: 'middle' });
const mouseWheel: Promise<void> = page.mouse.wheel(-1.5, 2.5);
const touchscreenTap: Promise<void> = page.touchscreen.tap(40.25, 60);
void touchscreenTap;
void mouseMove; void mouseDown; void mouseUp; void mouseClick; void mouseDouble; void mouseWheel;
// @ts-expect-error double-click count is always two
page.mouse.dblclick(1, 2, { clickCount: 3 });
// @ts-expect-error raw Mouse does not perform actionability checks
page.mouse.click(1, 2, { force: true });
// @ts-expect-error Mouse button is a closed union
page.mouse.down({ button: 'back' });
const content: Promise<string> = page.mainFrame().content();
const media: Promise<void> = page.emulateMedia({ media: 'print', colorScheme: null, reducedMotion: 'reduce', forcedColors: 'active', contrast: 'more' });
const requestedGc: Promise<void> = page.requestGC();
void media; void requestedGc;
// @ts-expect-error media is a closed union
page.emulateMedia({ media: 'speech' });
const evaluatedNumber: Promise<number> = page.evaluate(value => value + 1, 41);
const evaluatedAsync: Promise<string> = page.mainFrame().evaluate(async () => 'value');
const evaluatedExpression: Promise<number> = page.evaluate<number>('40 + 2');
void evaluatedNumber; void evaluatedAsync; void evaluatedExpression;
// @ts-expect-error the supplied argument must match the function parameter
page.evaluate((value: number) => value, 'wrong');
const newPage: Promise<Page> = page.context().newPage();
const sequential: Promise<void> = page.locator('input').pressSequentially('中文', { delay: 5, noWaitAfter: true, timeout: 500 });
const deprecatedTyping: Promise<void> = page.locator('input').type('abc');
void sequential; void deprecatedTyping;
// @ts-expect-error sequential input requires a string
page.locator('input').pressSequentially(12);
const locatorEvaluation: Promise<string> = page.locator('#content').evaluate((element, arg) => element.id + arg.suffix, { suffix: '!' }, { timeout: 500 });
const locatorElement: Promise<string> = page.locator('input').evaluate<string, HTMLInputElement>(element => element.value);
void locatorEvaluation; void locatorElement;
const newContext: Promise<BrowserContext> = browser.newContext({});
const closedContext: Promise<void> = page.context().close();
void newContext; void closedContext;
const closedPage: Promise<void> = page.close({ runBeforeUnload: false });
void newPage; void closedPage;
const navigation: Promise<Response | null> = page.goto('about:blank', {
  waitUntil: 'commit', timeout: 5000, referer: 'https://example.com/',
});
const frameNavigation: Promise<Response | null> = page.mainFrame().goto('about:blank', {
  waitUntil: 'networkidle', timeout: 5000, referer: 'https://example.com/',
});
void frameNavigation;
// @ts-expect-error Frame navigation accepts the same lifecycle modes as Page
page.mainFrame().goto('about:blank', { waitUntil: 'ready' });
declare const response: Response;
const reloaded: Promise<Response | null> = page.reload({ waitUntil: 'load', timeout: 5000 });
const historyResponse: Promise<Response | null> = page.goBack({ waitUntil: 'domcontentloaded', timeout: 5000 });
const forwardResponse: Promise<Response | null> = page.goForward({ waitUntil: 'commit', timeout: 5000 });
void reloaded; void historyResponse; void forwardResponse;
// @ts-expect-error reload does not accept a replacement URL or referer
page.reload({ referer: 'https://example.com/' });
// @ts-expect-error goBack does not accept a replacement URL or referer
page.goBack({ referer: 'https://example.com/' });
// @ts-expect-error goForward does not accept a replacement URL or referer
page.goForward({ referer: 'https://example.com/' });
const headers: Record<string, string> = response.headers();
const responseRequest: import('@shuozeli/pwright').Request = response.request();
const allRequestHeaders: Promise<Record<string, string>> = responseRequest.allHeaders();
const allResponseHeaders: Promise<Record<string, string>> = response.allHeaders();
const responseHeaderValue: Promise<string | null> = response.headerValue('Content-Type');
// @ts-expect-error response.headerValue requires a header name string
response.headerValue(1);
const eventualResponse: Promise<Response | null> = responseRequest.response();
const requestFailure: { errorText: string } | null = responseRequest.failure();
const redirectPredecessor: import('@shuozeli/pwright').Request | null = responseRequest.redirectedFrom();
const redirectSuccessor: import('@shuozeli/pwright').Request | null = responseRequest.redirectedTo();
const responseRequestPostData: unknown = responseRequest.postDataJSON();
const status: number = response.status();
const ok: boolean = response.ok();
const responsePage: Page = response.frame().page();
void identical; void connected; void error; void ownership;
void title; void content; void navigation; void headers; void responseRequest; void allRequestHeaders; void allResponseHeaders; void responseHeaderValue; void eventualResponse; void requestFailure; void redirectPredecessor; void redirectSuccessor; void responseRequestPostData; void status; void ok; void responsePage;
// @ts-expect-error CDP handshake header values must be strings
chromium.connectOverCDP('http://127.0.0.1:9222', { headers: { foo: 42 } });
// @ts-expect-error exactly one object-overload endpoint key is required
chromium.connectOverCDP({ endpointURL: 'http://127.0.0.1:9222', wsEndpoint: 'ws://example.test/ws' });
// @ts-expect-error waitUntil accepts only the four implemented lifecycle modes
page.goto('about:blank', { waitUntil: 'ready' });
