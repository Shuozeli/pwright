import { EventEmitter } from 'node:events';

export interface ConnectOverCDPOptions {
  timeout?: number;
  /** Requires the opt-in cdp-handshake-headers native profile. */
  headers?: Record<string, string>;
}
/** Opt-in Playwright Server connection options. Legacy diagnostic profiles always reject. */
export interface ConnectServerErrorOptions {
  /** The successful-session profile currently caps timeout 0 at 120000 ms. */
  timeout?: number;
  headers?: Record<string, string>;
}
/** Fresh Chromium only. The executable must already exist; no browser is downloaded. */
export interface ChromiumLaunchOptions {
  executablePath: string;
  headless?: boolean;
  timeout?: number;
  /** This profile installs no SIGINT handler; opting out matches that policy. */
  handleSIGINT?: false;
  /** Requires the opt-in playwright-trace native profile; recording is not installed yet. */
  tracesDir?: string;
}
export interface DeviceDescriptor {
  userAgent: string;
  viewport: { width: number; height: number };
  screen?: { width: number; height: number };
  deviceScaleFactor: number;
  isMobile: boolean;
  hasTouch: boolean;
  defaultBrowserType: 'chromium' | 'firefox' | 'webkit';
}
export interface BrowserNewContextOptions {
  baseURL?: string;
  extraHTTPHeaders?: { [key: string]: string };
  proxy?: { server: string; bypass?: string; username?: string; password?: string };
  acceptDownloads?: true;
  bypassCSP?: boolean;
  colorScheme?: 'light';
  hasTouch?: boolean;
  ignoreHTTPSErrors?: false;
  isMobile?: false;
  javaScriptEnabled?: true;
  locale?: 'en-US';
  offline?: boolean;
  serviceWorkers?: 'allow';
  viewport?: { width: 1280; height: 720 };
}
export interface EmulateMediaOptions {
  media?: 'screen' | 'print' | null;
  colorScheme?: 'dark' | 'light' | 'no-preference' | null;
  reducedMotion?: 'reduce' | 'no-preference' | null;
  forcedColors?: 'active' | 'none' | null;
  contrast?: 'more' | 'no-preference' | null;
}
export type URLMatch = string | RegExp | ((url: URL) => boolean);
export type RequestMatch = string | RegExp | ((request: Request) => boolean | Promise<boolean>);
export type ResponseMatch = string | RegExp | ((response: Response) => boolean | Promise<boolean>);
export interface WaitForURLOptions {
  waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle';
  timeout?: number;
}
type WithoutHandles<T> = T extends JSHandle ? never : T extends object ? { [K in keyof T]: WithoutHandles<T[K]> } : T;
export type Unboxed<T> = T extends ElementHandle<infer E> ? E : T extends JSHandle<infer V> ? V : T extends WithoutHandles<T> ? T : T extends object ? { [K in keyof T]: Unboxed<T[K]> } : T;
export type PageFunction<Arg, Result> = string | ((arg: Unboxed<Arg>) => Result | Promise<Result>);
export type SmartHandle<T> = T extends Node ? ElementHandle<T> : JSHandle<T>;
/** Explicit handles native profile; most ElementHandle actions remain unsupported. */
export interface JSHandle<T = unknown> {
  evaluate<Result = unknown>(expression: string, arg?: unknown): Promise<Awaited<Result>>;
  asElement(): ElementHandle<Node> | null;
  jsonValue(): Promise<T>;
  evaluate<Result, Arg>(pageFunction: (value: T, arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  evaluate<Result>(pageFunction: (value: T) => Result | Promise<Result>): Promise<Awaited<Result>>;
  evaluateHandle<Result = unknown>(expression: string, arg?: unknown): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result, Arg>(pageFunction: (value: T, arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result>(pageFunction: (value: T) => Result | Promise<Result>): Promise<SmartHandle<Awaited<Result>>>;
  getProperty<Key extends keyof T>(name: Key): Promise<JSHandle<T[Key]>>;
  getProperty(name: string): Promise<JSHandle<unknown>>;
  getProperties(): Promise<Map<string, JSHandle<unknown>>>;
  dispose(): Promise<void>;
}
export interface ElementHandle<T extends Node = HTMLElement | SVGElement> extends JSHandle<T> {
  asElement(): this;
  $(selector: string): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  $$(selector: string): Promise<ElementHandle<HTMLElement | SVGElement>[]>;
  waitForSelector<K extends keyof HTMLElementTagNameMap>(selector: K, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElementTagNameMap[K]>>;
  waitForSelector(selector: string, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElement | SVGElement>>;
  waitForSelector(selector: string, options: WaitForSelectorOptions): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  $eval<Result, Arg>(selector: string, pageFunction: (element: Element, arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $eval<Result>(selector: string, pageFunction: string | ((element: Element) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  $$eval<Result, Arg>(selector: string, pageFunction: (elements: Element[], arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $$eval<Result>(selector: string, pageFunction: string | ((elements: Element[]) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  getAttribute(name: string): Promise<string | null>;
  innerHTML(): Promise<string>;
  innerText(): Promise<string>;
  inputValue(): Promise<string>;
  focus(): Promise<void>;
  /** Opt-in retained-Handle subset: string value or label only; no descriptor or option-Handle input. */
  selectOption(value: string, options?: { timeout?: number }): Promise<string[]>;
  scrollIntoViewIfNeeded(options?: LocatorReadOptions): Promise<void>;
  isChecked(): Promise<boolean>;
  isDisabled(): Promise<boolean>;
  isEditable(): Promise<boolean>;
  isEnabled(): Promise<boolean>;
  isHidden(): Promise<boolean>;
  isVisible(): Promise<boolean>;
  boundingBox(): Promise<BoundingBox | null>;
  tap(options?: LocatorTapOptions): Promise<void>;
  /** Live JSHandle values in eventInit require the optional Handle-enabled native profile. */
  dispatchEvent(type: string, eventInit?: unknown): Promise<void>;
  textContent(): Promise<string | null>;
  ownerFrame(): Promise<Frame | null>;
  contentFrame(): Promise<Frame | null>;
}
export interface SessionMetrics {
  connected: boolean;
  liveObjects: number;
  pendingCalls: number;
  topologyDrivers: number;
  activeLeases: number;
  liveScopes: number;
  retainedEventBytes: number;
  activeOperations: number;
  cleanupInFlight: number;
}
export interface BrowserType<Name extends 'chromium' | 'firefox' | 'webkit' = 'chromium' | 'firefox' | 'webkit'> {
  name(): Name;
  /** Expected pinned browser path; requires the opt-in browser-registry native profile and does not prove installation. */
  executablePath(): string;
  connectOverCDP(endpointURL: string, options?: ConnectOverCDPOptions): Promise<Browser>;
  connectOverCDP(options: ConnectOverCDPOptions & ({ endpointURL: string; wsEndpoint?: never } | { wsEndpoint: string; endpointURL?: never })): Promise<Browser>;
  /** Requires the opt-in owned-launch native profile and an explicit Chromium executable. */
  launch(options: Name extends 'chromium' ? ChromiumLaunchOptions : never): Promise<Browser>;
  /** Only pre-launch channel validation is available; valid launches reject as unsupported. */
  launchServer(options?: { channel?: string }): Promise<never>;
  /** Only invalid-option preflight is available; valid launches reject as unsupported. */
  launchPersistentContext(userDataDir: string, options?: {
    port?: unknown;
    args?: string[];
    ignoreDefaultArgs?: boolean | string[];
    channel?: string;
    clientCertificates?: APIRequestClientCertificate[];
  }): Promise<never>;
  /** Requires the opt-in successful-session profile. Supports connection lifecycle, empty-option newContext, Context.newPage, and explicit empty-option Context.close with a current addon; most Context/Page methods remain unsupported. */
  connect(wsEndpoint: string, options?: ConnectServerErrorOptions): Promise<Browser>;
  connect(options: ConnectServerErrorOptions & { wsEndpoint: string }): Promise<Browser>;
}
export interface Selectors {
  register(
    name: string,
    script: Function | string | { path?: string; content?: string },
    options?: { contentScript?: boolean },
  ): Promise<void>;
  setTestIdAttribute(attributeName: string): void;
}
export interface Browser extends EventEmitter {
  browserType(): BrowserType;
  isConnected(): boolean;
  version(): string;
  contexts(): BrowserContext[];
  /** Requires the opt-in browser-cdp-session native profile. */
  newBrowserCDPSession(): Promise<CDPSession>;
  close(): Promise<void>;
  pwrightMetrics(): SessionMetrics;
  pwrightMetadataMetrics(): { entries: number; urlBytes: number; nameBytes?: number; activeFrames: number; pendingPublications: number };
  pwrightFrameTreeMetrics(): { states: number; revisions: number; bytes: number };
  pwrightContextMetadataMetrics(): { states: number; conversions: number; bytes: number; activeContexts: number; pendingCreations: number };
  pwrightFfiMetrics(): { pending: number; queued: number; queuedCommands: number; running: number; reservedBytes: number; connects: number; commands: number; closes: number; watches: number };
  pwrightConversionMetrics(): { pending: number; binaryReferences: number; backingBytes: number };
  pwrightLocatorConversionMetrics(): { pending: number; rawReferences: number; binaryReferences: number; backingBytes: number; exceptions: number };
  pwrightLocatorMetrics(): { recipes: number; bytes: number };
  newContext(options?: BrowserNewContextOptions): Promise<BrowserContext>;
  /** The opt-in Playwright Server session supports only empty options and closes its implicit Context with this Page. */
  newPage(options?: Pick<BrowserNewContextOptions, 'baseURL'> & { acceptDownloads?: false }): Promise<Page>;
}
export interface BrowserContext extends EventEmitter {
  readonly request: APIRequestContext;
  addCookies(cookies: ReadonlyArray<BrowserContextCookieInput>): Promise<void>;
  setDefaultTimeout(timeout: number | undefined): void;
  setDefaultNavigationTimeout(timeout: number | undefined): void;
  setExtraHTTPHeaders(headers: { [key: string]: string }): Promise<void>;
  setOffline(offline: boolean): Promise<void>;
  addInitScript(script: string | (() => unknown) | { path?: string; content?: string }): Promise<void>;
  browser(): Browser;
  on(event: 'dialog', listener: (dialog: Dialog) => void): this;
  waitForEvent(event: 'dialog', optionsOrPredicate?: { predicate?: (dialog: Dialog) => boolean | Promise<boolean>; timeout?: number } | ((dialog: Dialog) => boolean | Promise<boolean>)): Promise<Dialog>;
  waitForEvent(event: string, optionsOrPredicate?: { predicate?: (value: unknown) => boolean | Promise<boolean>; timeout?: number } | ((value: unknown) => boolean | Promise<boolean>)): Promise<unknown>;
  pages(): Page[];
  newPage(): Promise<Page>;
  /** Opt-in Chromium CDP for Page/main Frame, including idle events; OOPIF child sessions are unsupported. */
  newCDPSession(page: Page | Frame): Promise<CDPSession>;
  close(options?: { reason?: string }): Promise<void>;
}
export interface CDPSession extends EventEmitter {
  send(method: string, params?: Record<string, unknown>): Promise<Record<string, unknown>>;
  detach(): Promise<void>;
  on(event: 'event', listener: (event: { method: string; params: Record<string, unknown> }) => void): this;
  on(event: 'close', listener: (session: CDPSession) => void): this;
  on(event: string, listener: (params: Record<string, unknown>) => void): this;
  once(event: 'event', listener: (event: { method: string; params: Record<string, unknown> }) => void): this;
  once(event: 'close', listener: (session: CDPSession) => void): this;
  once(event: string, listener: (params: Record<string, unknown>) => void): this;
  removeListener(event: 'event', listener: (event: { method: string; params: Record<string, unknown> }) => void): this;
  removeListener(event: 'close', listener: (session: CDPSession) => void): this;
  removeListener(event: string, listener: (params: Record<string, unknown>) => void): this;
}
export interface BrowserContextCookieInput {
  name: string;
  value: string;
  url?: string;
  domain?: string;
  path?: string;
  expires?: number;
  httpOnly?: boolean;
  secure?: boolean;
  sameSite?: 'Strict' | 'Lax' | 'None';
  partitionKey?: string;
}
export interface Page extends EventEmitter, SemanticLocators {
  readonly request: APIRequestContext;
  setDefaultTimeout(timeout: number): void;
  setDefaultNavigationTimeout(timeout: number): void;
  on(event: 'console', listener: (message: ConsoleMessage) => void): this;
  on(event: 'dialog', listener: (dialog: Dialog) => void): this;
  on(event: 'request' | 'requestfinished' | 'requestfailed', listener: (request: Request) => void): this;
  evaluateHandle<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<SmartHandle<Awaited<Result>>>;
  setInputFiles(selector: string, files: InputFiles, options?: SelectorSetInputFilesOptions): Promise<void>;
  setExtraHTTPHeaders(headers: { [key: string]: string }): Promise<void>;
  readonly keyboard: Keyboard;
  readonly mouse: Mouse;
  readonly touchscreen: Touchscreen;
  context(): BrowserContext;
  opener(): Promise<Page | null>;
  waitForEvent(event: 'dialog', optionsOrPredicate?: { predicate?: (dialog: Dialog) => boolean | Promise<boolean>; timeout?: number } | ((dialog: Dialog) => boolean | Promise<boolean>)): Promise<Dialog>;
  waitForEvent(event: string, optionsOrPredicate?: { predicate?: (value: unknown) => boolean | Promise<boolean>; timeout?: number } | ((value: unknown) => boolean | Promise<boolean>)): Promise<unknown>;
  waitForRequest(urlOrPredicate: RequestMatch, options?: { timeout?: number }): Promise<Request>;
  waitForResponse(urlOrPredicate: ResponseMatch, options?: { timeout?: number }): Promise<Response>;
  waitForLoadState(state?: 'load' | 'domcontentloaded' | 'networkidle' | 'commit', options?: { timeout?: number }): Promise<void>;
  waitForNavigation(options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<Response | null>;
  consoleMessages(): Promise<ConsoleMessage[]>;
  exposeBinding(name: string, callback: (source: BindingSource, ...args: unknown[]) => unknown | Promise<unknown>, options?: { handle?: boolean }): Promise<void>;
  exposeFunction(name: string, callback: (...args: unknown[]) => unknown | Promise<unknown>): Promise<void>;
  /** RegExp matchers require the opt-in route-regexp native profile. */
  route(url: string | RegExp, handler: (route: Route, request: Request) => unknown | Promise<unknown>, options?: { times?: number }): Promise<void>;
  /** Requires the opt-in page-unroute native profile; RegExp matchers also require route-regexp. */
  unroute(url: string | RegExp, handler?: (route: Route, request: Request) => unknown | Promise<unknown>): Promise<void>;
  addInitScript(script: string | (() => unknown) | { path?: string; content?: string }): Promise<void>;
  addScriptTag(options?: { url?: string; path?: string; content?: string; type?: string }): Promise<ElementHandle<HTMLScriptElement>>;
  $(selector: string): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  $$(selector: string): Promise<Array<ElementHandle<HTMLElement | SVGElement>>>;
  waitForSelector<K extends keyof HTMLElementTagNameMap>(selector: K, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElementTagNameMap[K]>>;
  waitForSelector(selector: string, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElement | SVGElement>>;
  waitForSelector(selector: string, options: WaitForSelectorOptions): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  getAttribute(selector: string, name: string, options?: LocatorReadOptions): Promise<string | null>;
  innerHTML(selector: string, options?: LocatorReadOptions): Promise<string>;
  inputValue(selector: string, options?: LocatorReadOptions): Promise<string>;
  isChecked(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isDisabled(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isEditable(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isEnabled(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isVisible(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isHidden(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  $eval<Result, Arg>(selector: string, pageFunction: (element: Element, arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $eval<Result>(selector: string, pageFunction: string | ((element: Element) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  $$eval<Result, Arg>(selector: string, pageFunction: (elements: Element[], arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $$eval<Result>(selector: string, pageFunction: string | ((elements: Element[]) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  waitForTimeout(timeout: number): Promise<void>;
  waitForFunction<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg, options?: WaitForFunctionOptions): Promise<JSHandle<Awaited<Result>>>;
  waitForFunction<Result>(pageFunction: string | (() => Result | Promise<Result>), arg?: unknown, options?: WaitForFunctionOptions): Promise<JSHandle<Awaited<Result>>>;
  waitForURL(url: URLMatch, options?: WaitForURLOptions): Promise<void>;
  mainFrame(): Frame;
  frame(name: string): Frame | null;
  frame(options: { name?: string; url?: string | RegExp | ((url: URL) => boolean) }): Frame | null;
  frames(): Frame[];
  screenshot(options?: ScreenshotOptions): Promise<Buffer>;
  setViewportSize(viewportSize: { width: number; height: number }): Promise<void>;
  goto(url: string, options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number; referer?: string }): Promise<Response | null>;
  reload(options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<Response | null>;
  goBack(options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<Response | null>;
  goForward(options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<Response | null>;
  title(): Promise<string>;
  content(): Promise<string>;
  setContent(html: string, options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<void>;
  emulateMedia(options?: EmulateMediaOptions): Promise<void>;
  requestGC(): Promise<void>;
  press(selector: string, key: string, options?: LocatorPressOptions): Promise<void>;
  /** @deprecated Use locator(selector).pressSequentially(text) instead. */
  type(selector: string, text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  focus(selector: string, options?: LocatorReadOptions): Promise<void>;
  click(selector: string, options?: PageClickOptions): Promise<void>;
  dblclick(selector: string, options?: PageDblclickOptions): Promise<void>;
  tap(selector: string, options?: LocatorTapOptions): Promise<void>;
  check(selector: string, options?: PageCheckedOptions): Promise<void>;
  uncheck(selector: string, options?: PageCheckedOptions): Promise<void>;
  setChecked(selector: string, checked: boolean, options?: PageCheckedOptions): Promise<void>;
  /** Live JSHandle values in eventInit require the optional Handle-enabled native profile. */
  dispatchEvent(selector: string, type: string, eventInit?: unknown, options?: PageDispatchEventOptions): Promise<void>;
  fill(selector: string, value: string, options?: LocatorFillOptions): Promise<void>;
  /** ElementHandle input requires a handles-capable native profile and the same session and select. */
  selectOption(selector: string, values: string | SelectOptionDescriptor | string[] | SelectOptionDescriptor[] | ElementHandle | null, options?: LocatorFillOptions): Promise<string[]>;
  innerText(selector: string, options?: LocatorReadOptions): Promise<string>;
  textContent(selector: string, options?: LocatorReadOptions): Promise<string | null>;
  url(): string;
  evaluate<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<Awaited<Result>>;
  evaluate<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<Awaited<Result>>;
  close(options?: { runBeforeUnload?: false }): Promise<void>;
  isClosed(): boolean;
  locator(selector: string, options?: LocatorFilterOptions): Locator;
  frameLocator(selector: string): FrameLocator;
}
export interface BindingSource { context: BrowserContext; page: Page; frame: Frame; }
export interface Request {
  url(): string;
  method(): string;
  resourceType(): string;
  headers(): Record<string, string>;
  allHeaders(): Promise<Record<string, string>>;
  headersArray(): Array<{ name: string; value: string }>;
  postData(): string | null;
  postDataBuffer(): Buffer | null;
  postDataJSON(): unknown;
  isNavigationRequest(): boolean;
  frame(): Frame | null;
  response(): Promise<Response | null>;
  failure(): { errorText: string } | null;
  redirectedFrom(): Request | null;
  redirectedTo(): Request | null;
}
export interface Route {
  request(): Request;
  continue(options?: RouteContinueOptions): Promise<void>;
  fallback(options?: RouteFallbackOptions): Promise<void>;
  abort(errorCode?: string): Promise<void>;
  fulfill(options?: RouteFulfillOptions): Promise<void>;
  fetch(): Promise<never>;
}
/** Requires the opt-in route-header-url-overrides native profile. */
export interface RouteContinueOptions { headers?: Record<string, string>; }
/** Requires the opt-in route-header-url-overrides native profile. */
export interface RouteFallbackOptions { url?: string; }
export interface RouteFulfillOptions {
  status?: number;
  headers?: Record<string, string>;
  body?: string | Buffer;
  json?: unknown;
  contentType?: string;
}
export interface Dialog {
  page(): Page;
  type(): 'alert' | 'beforeunload' | 'confirm' | 'prompt';
  message(): string;
  defaultValue(): string;
  accept(promptText?: string): Promise<void>;
  dismiss(): Promise<void>;
}
export interface ConsoleMessage {
  page(): Page | null;
  worker(): null;
  type(): string;
  text(): string;
  args(): JSHandle[];
  location(): { url: string; lineNumber: number; columnNumber: number };
}
export interface ScreenshotOptions {
  type?: 'png' | 'jpeg';
  quality?: number;
  fullPage?: boolean;
  clip?: { x: number; y: number; width: number; height: number };
  scale?: 'device' | 'css';
  timeout?: number;
}
export interface WaitForFunctionOptions {
  polling?: 'raf' | number;
  timeout?: number;
}
export interface Frame extends SemanticLocators {
  evaluateHandle<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<SmartHandle<Awaited<Result>>>;
  setInputFiles(selector: string, files: InputFiles, options?: SelectorSetInputFilesOptions): Promise<void>;
  page(): Page;
  goto(url: string, options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number; referer?: string }): Promise<Response | null>;
  name(): string;
  parentFrame(): Frame | null;
  childFrames(): Frame[];
  isDetached(): boolean;
  /** Requires the opt-in frame-element native profile. */
  frameElement(): Promise<ElementHandle<HTMLIFrameElement | HTMLFrameElement>>;
  locator(selector: string, options?: LocatorFilterOptions): Locator;
  frameLocator(selector: string): FrameLocator;
  url(): string;
  title(): Promise<string>;
  content(): Promise<string>;
  addScriptTag(options?: { url?: string; path?: string; content?: string; type?: string }): Promise<ElementHandle<HTMLScriptElement>>;
  $(selector: string): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  $$(selector: string): Promise<Array<ElementHandle<HTMLElement | SVGElement>>>;
  waitForSelector<K extends keyof HTMLElementTagNameMap>(selector: K, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElementTagNameMap[K]>>;
  waitForSelector(selector: string, options?: WaitForSelectorOptionsNotHidden): Promise<ElementHandle<HTMLElement | SVGElement>>;
  waitForSelector(selector: string, options: WaitForSelectorOptions): Promise<ElementHandle<HTMLElement | SVGElement> | null>;
  getAttribute(selector: string, name: string, options?: LocatorReadOptions): Promise<string | null>;
  innerHTML(selector: string, options?: LocatorReadOptions): Promise<string>;
  innerText(selector: string, options?: LocatorReadOptions): Promise<string>;
  inputValue(selector: string, options?: LocatorReadOptions): Promise<string>;
  textContent(selector: string, options?: LocatorReadOptions): Promise<string | null>;
  isChecked(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isDisabled(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isEditable(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isEnabled(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isVisible(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  isHidden(selector: string, options?: LocatorReadOptions): Promise<boolean>;
  $eval<Result, Arg>(selector: string, pageFunction: (element: Element, arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $eval<Result>(selector: string, pageFunction: string | ((element: Element) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  $$eval<Result, Arg>(selector: string, pageFunction: (elements: Element[], arg: Unboxed<Arg>) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  $$eval<Result>(selector: string, pageFunction: string | ((elements: Element[]) => Result | Promise<Result>)): Promise<Awaited<Result>>;
  waitForTimeout(timeout: number): Promise<void>;
  waitForFunction<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg, options?: WaitForFunctionOptions): Promise<JSHandle<Awaited<Result>>>;
  waitForFunction<Result>(pageFunction: string | (() => Result | Promise<Result>), arg?: unknown, options?: WaitForFunctionOptions): Promise<JSHandle<Awaited<Result>>>;
  waitForURL(url: URLMatch, options?: WaitForURLOptions): Promise<void>;
  waitForLoadState(state?: 'load' | 'domcontentloaded' | 'networkidle' | 'commit', options?: { timeout?: number }): Promise<void>;
  press(selector: string, key: string, options?: LocatorPressOptions): Promise<void>;
  /** @deprecated Use locator(selector).pressSequentially(text) instead. */
  type(selector: string, text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  focus(selector: string, options?: LocatorReadOptions): Promise<void>;
  click(selector: string, options?: PageClickOptions): Promise<void>;
  dblclick(selector: string, options?: PageDblclickOptions): Promise<void>;
  tap(selector: string, options?: LocatorTapOptions): Promise<void>;
  fill(selector: string, value: string, options?: LocatorFillOptions): Promise<void>;
  /** ElementHandle input requires a handles-capable native profile and the same session and select. */
  selectOption(selector: string, values: string | SelectOptionDescriptor | string[] | SelectOptionDescriptor[] | ElementHandle | null, options?: LocatorFillOptions): Promise<string[]>;
  pwrightMetadataMetrics(): { unpublishedUrl: boolean };
  evaluate<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<Awaited<Result>>;
  evaluate<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<Awaited<Result>>;
}
export type AriaRole = 'alert' | 'alertdialog' | 'application' | 'article' | 'banner' | 'blockquote' | 'button' | 'caption' | 'cell' | 'checkbox' | 'code' | 'columnheader' | 'combobox' | 'complementary' | 'contentinfo' | 'definition' | 'deletion' | 'dialog' | 'directory' | 'document' | 'emphasis' | 'feed' | 'figure' | 'form' | 'generic' | 'grid' | 'gridcell' | 'group' | 'heading' | 'img' | 'insertion' | 'link' | 'list' | 'listbox' | 'listitem' | 'log' | 'main' | 'marquee' | 'math' | 'meter' | 'menu' | 'menubar' | 'menuitem' | 'menuitemcheckbox' | 'menuitemradio' | 'navigation' | 'none' | 'note' | 'option' | 'paragraph' | 'presentation' | 'progressbar' | 'radio' | 'radiogroup' | 'region' | 'row' | 'rowgroup' | 'rowheader' | 'scrollbar' | 'search' | 'searchbox' | 'separator' | 'slider' | 'spinbutton' | 'status' | 'strong' | 'subscript' | 'superscript' | 'switch' | 'tab' | 'table' | 'tablist' | 'tabpanel' | 'term' | 'textbox' | 'time' | 'timer' | 'toolbar' | 'tooltip' | 'tree' | 'treegrid' | 'treeitem';
export interface TextQueryOptions { exact?: boolean; }
export interface RoleQueryOptions extends TextQueryOptions {
  name?: string | RegExp;
  checked?: boolean;
  disabled?: boolean;
  expanded?: boolean;
  includeHidden?: boolean;
  level?: number;
  pressed?: boolean;
  selected?: boolean;
}
export interface SemanticLocators {
  getByText(text: string | RegExp, options?: TextQueryOptions): Locator;
  getByLabel(text: string | RegExp, options?: TextQueryOptions): Locator;
  getByPlaceholder(text: string | RegExp, options?: TextQueryOptions): Locator;
  getByAltText(text: string | RegExp, options?: TextQueryOptions): Locator;
  getByTitle(text: string | RegExp, options?: TextQueryOptions): Locator;
  getByTestId(testId: string | RegExp): Locator;
  getByRole(role: AriaRole, options?: RoleQueryOptions): Locator;
}
export interface LocatorFilterOptions {
  hasText?: string | RegExp;
  hasNotText?: string | RegExp;
  has?: Locator;
  hasNot?: Locator;
  visible?: boolean;
}
export interface FrameLocator extends SemanticLocators {
  locator(selector: string | Locator, options?: LocatorFilterOptions): Locator;
  owner(): Locator;
  frameLocator(selector: string): FrameLocator;
  first(): FrameLocator;
  last(): FrameLocator;
  nth(index: number): FrameLocator;
}
export interface LocatorReadOptions { timeout?: number; }
export interface BoundingBox { x: number; y: number; width: number; height: number; }
export interface LocatorWaitForOptions extends LocatorReadOptions {
  state?: 'attached' | 'detached' | 'visible' | 'hidden';
}
export interface WaitForSelectorOptions extends LocatorWaitForOptions { strict?: boolean; }
export interface WaitForSelectorOptionsNotHidden extends Omit<WaitForSelectorOptions, 'state'> {
  state?: 'attached' | 'visible';
}
export type KeyboardModifier = 'Alt' | 'Control' | 'ControlOrMeta' | 'Meta' | 'Shift';
export interface LocatorPointerOptions extends LocatorReadOptions {
  force?: boolean; modifiers?: KeyboardModifier[]; position?: { x: number; y: number }; trial?: boolean; noWaitAfter?: boolean;
}
export interface LocatorClickOptions extends LocatorPointerOptions {
  button?: 'left' | 'right' | 'middle'; clickCount?: number; delay?: number; steps?: number;
}
export interface PageClickOptions extends Omit<LocatorClickOptions, 'steps'> { strict?: boolean; }
export interface PageDblclickOptions extends Omit<LocatorClickOptions, 'clickCount' | 'steps'> { strict?: boolean; }
export interface LocatorTapOptions extends LocatorPointerOptions {}
export interface LocatorFillOptions extends LocatorReadOptions { force?: boolean; noWaitAfter?: boolean; }
export interface LocatorPressOptions extends LocatorReadOptions { delay?: number; noWaitAfter?: boolean; }
export interface LocatorPressSequentiallyOptions extends LocatorReadOptions { delay?: number; noWaitAfter?: boolean; }
export interface LocatorCheckedOptions extends LocatorFillOptions { position?: { x: number; y: number }; trial?: boolean; }
export interface PageCheckedOptions extends LocatorCheckedOptions { strict?: boolean; }
export interface PageDispatchEventOptions extends LocatorReadOptions { strict?: boolean; }
export interface LocatorSelectTextOptions extends LocatorReadOptions { force?: boolean; }
export type LocatorScreenshotOptions = Pick<ScreenshotOptions, 'type' | 'quality' | 'scale' | 'timeout'>;
export interface SelectOptionDescriptor { value?: string; label?: string; index?: number; }
export interface FilePayload { name: string; mimeType: string; buffer: Buffer; }
export type InputFiles = string | ReadonlyArray<string> | FilePayload | ReadonlyArray<FilePayload>;
export interface SetInputFilesOptions extends LocatorReadOptions {
  /** @deprecated This option has no effect. */
  noWaitAfter?: boolean;
}
export interface SelectorSetInputFilesOptions extends SetInputFilesOptions { strict?: boolean; }
export interface Locator extends SemanticLocators {
  elementHandle(options?: LocatorReadOptions): Promise<ElementHandle<HTMLElement | SVGElement>>;
  elementHandles(): Promise<Array<ElementHandle<HTMLElement | SVGElement>>>;
  evaluate<Result, Arg, E extends Element = HTMLElement | SVGElement>(pageFunction: (element: E, arg: Arg) => Result | Promise<Result>, arg: Arg, options?: LocatorReadOptions): Promise<Awaited<Result>>;
  evaluate<Result, E extends Element = HTMLElement | SVGElement>(pageFunction: string | ((element: E) => Result | Promise<Result>), arg?: undefined, options?: LocatorReadOptions): Promise<Awaited<Result>>;
  evaluateHandle<Result, Arg, E extends Element = HTMLElement | SVGElement>(pageFunction: (element: E, arg: Arg) => Result | Promise<Result>, arg: Arg, options?: LocatorReadOptions): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result, E extends Element = HTMLElement | SVGElement>(pageFunction: string | ((element: E) => Result | Promise<Result>), arg?: undefined, options?: LocatorReadOptions): Promise<SmartHandle<Awaited<Result>>>;
  evaluateAll<Result, Arg, E extends Element = HTMLElement | SVGElement>(pageFunction: (elements: E[], arg: Arg) => Result | Promise<Result>, arg: Arg): Promise<Awaited<Result>>;
  evaluateAll<Result, E extends Element = HTMLElement | SVGElement>(pageFunction: (elements: E[]) => Result | Promise<Result>, arg?: undefined): Promise<Awaited<Result>>;
  page(): Page;
  first(): Locator;
  last(): Locator;
  nth(index: number): Locator;
  contentFrame(): FrameLocator;
  frameLocator(selector: string): FrameLocator;
  and(other: Locator): Locator;
  or(other: Locator): Locator;
  locator(selector: string | Locator, options?: LocatorFilterOptions): Locator;
  filter(options?: LocatorFilterOptions): Locator;
  describe(description: string): Locator;
  description(): string | null;
  toString(): string;
  count(): Promise<number>;
  boundingBox(options?: LocatorReadOptions): Promise<BoundingBox | null>;
  highlight(): Promise<void>;
  dispatchEvent(type: string, eventInit?: unknown, options?: LocatorReadOptions): Promise<void>;
  selectText(options?: LocatorSelectTextOptions): Promise<void>;
  screenshot(options?: LocatorScreenshotOptions): Promise<Buffer>;
  all(): Promise<Locator[]>;
  allTextContents(): Promise<string[]>;
  allInnerTexts(): Promise<string[]>;
  textContent(options?: LocatorReadOptions): Promise<string | null>;
  innerText(options?: LocatorReadOptions): Promise<string>;
  innerHTML(options?: LocatorReadOptions): Promise<string>;
  inputValue(options?: LocatorReadOptions): Promise<string>;
  getAttribute(name: string, options?: LocatorReadOptions): Promise<string | null>;
  isVisible(options?: LocatorReadOptions): Promise<boolean>;
  isHidden(options?: LocatorReadOptions): Promise<boolean>;
  isEnabled(options?: LocatorReadOptions): Promise<boolean>;
  isDisabled(options?: LocatorReadOptions): Promise<boolean>;
  isEditable(options?: LocatorReadOptions): Promise<boolean>;
  isChecked(options?: LocatorReadOptions): Promise<boolean>;
  waitFor(options?: LocatorWaitForOptions): Promise<void>;
  click(options?: LocatorClickOptions): Promise<void>;
  dblclick(options?: Omit<LocatorClickOptions, 'clickCount' | 'steps'>): Promise<void>;
  tap(options?: LocatorTapOptions): Promise<void>;
  fill(value: string, options?: LocatorFillOptions): Promise<void>;
  clear(options?: LocatorFillOptions): Promise<void>;
  press(key: string, options?: LocatorPressOptions): Promise<void>;
  pressSequentially(text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  /** @deprecated Use pressSequentially instead. */
  type(text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  check(options?: LocatorCheckedOptions): Promise<void>;
  uncheck(options?: LocatorCheckedOptions): Promise<void>;
  setChecked(checked: boolean, options?: LocatorCheckedOptions): Promise<void>;
  /** ElementHandle input requires a handles-capable native profile and the same session and select. */
  selectOption(values: string | SelectOptionDescriptor | string[] | SelectOptionDescriptor[] | ElementHandle | null, options?: LocatorFillOptions): Promise<string[]>;
  setInputFiles(files: InputFiles, options?: SetInputFilesOptions): Promise<void>;
  hover(options?: LocatorPointerOptions): Promise<void>;
  focus(options?: LocatorReadOptions): Promise<void>;
  blur(options?: LocatorReadOptions): Promise<void>;
  scrollIntoViewIfNeeded(options?: LocatorReadOptions): Promise<void>;
}
export type MouseButton = 'left' | 'right' | 'middle';
export interface Mouse {
  move(x: number, y: number, options?: { steps?: number }): Promise<void>;
  down(options?: { button?: MouseButton; clickCount?: number }): Promise<void>;
  up(options?: { button?: MouseButton; clickCount?: number }): Promise<void>;
  click(x: number, y: number, options?: { button?: MouseButton; clickCount?: number; delay?: number }): Promise<void>;
  dblclick(x: number, y: number, options?: { button?: MouseButton; delay?: number }): Promise<void>;
  wheel(deltaX: number, deltaY: number): Promise<void>;
}
export interface Touchscreen {
  tap(x: number, y: number): Promise<void>;
}
export interface Keyboard {
  down(key: string): Promise<void>;
  up(key: string): Promise<void>;
  press(key: string, options?: { delay?: number }): Promise<void>;
  insertText(text: string): Promise<void>;
  type(text: string, options?: { delay?: number }): Promise<void>;
}
export interface Response {
  url(): string;
  status(): number;
  statusText(): string;
  ok(): boolean;
  headers(): Record<string, string>;
  /** Ordered raw headers require the optional route-request-history native capability. */
  allHeaders(): Promise<Record<string, string>>;
  /** Looks up a response header from the optional route-request-history native capability. */
  headerValue(name: string): Promise<string | null>;
  request(): Request;
  frame(): Frame;
  body(): Promise<Buffer>;
  text(): Promise<string>;
  json(): Promise<unknown>;
  finished(): Promise<null>;
}
export interface APIRequestNewContextOptions {
  baseURL?: string;
  userAgent?: string;
  extraHTTPHeaders?: Record<string, string>;
  timeout?: number;
  failOnStatusCode?: boolean;
  ignoreHTTPSErrors?: boolean;
  maxRedirects?: number;
  httpCredentials?: {
    username: string;
    password: string;
    origin?: string;
    send?: 'unauthorized' | 'always';
  };
  storageState?: string | APIRequestStorageState;
  clientCertificates?: APIRequestClientCertificate[];
  proxy?: {
    server: string;
    bypass?: string;
    username?: string;
    password?: string;
  };
}
export interface APIRequestClientCertificate {
  origin: string;
  cert?: Buffer;
  certPath?: string;
  key?: Buffer;
  keyPath?: string;
  pfx?: Buffer;
  pfxPath?: string;
  passphrase?: string;
}
export interface APIRequestStorageCookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  expires: number;
  httpOnly: boolean;
  secure: boolean;
  sameSite: 'Strict' | 'Lax' | 'None';
}
export interface APIRequestStorageOrigin {
  origin: string;
  localStorage: Array<{ name: string; value: string }>;
  indexedDB?: unknown[];
}
export interface APIRequestStorageState {
  cookies: APIRequestStorageCookie[];
  origins: APIRequestStorageOrigin[];
}
export interface APIRequestOptions {
  headers?: Record<string, string> | ReadonlyArray<{ name: string; value: string }>;
  data?: unknown;
  timeout?: number;
  failOnStatusCode?: boolean;
  ignoreHTTPSErrors?: boolean;
  maxRedirects?: number;
  maxRetries?: number;
  method?: string;
}
export interface APIResponse {
  url(): string;
  status(): number;
  statusText(): string;
  ok(): boolean;
  headers(): Record<string, string>;
  headersArray(): Array<{ name: string; value: string }>;
  body(): Promise<Buffer>;
  text(): Promise<string>;
  json(): Promise<unknown>;
  dispose(): Promise<void>;
  log(): Promise<string[]>;
}
export interface APIRequestContext {
  fetch(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  delete(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  get(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  head(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  patch(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  post(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  put(url: string, options?: APIRequestOptions): Promise<APIResponse>;
  storageState(options?: { indexedDB?: boolean; path?: string }): Promise<APIRequestStorageState>;
  dispose(options?: { reason?: string }): Promise<void>;
  pwrightMetrics(): { disposed: boolean; inFlight: number; liveResponses: number; retainedBodyBytes: number; retainedLogBytes: number };
}
export interface APIRequest {
  readonly _contexts: ReadonlySet<APIRequestContext>;
  newContext(options?: APIRequestNewContextOptions): Promise<APIRequestContext>;
}
declare class TimeoutError extends Error {}
declare class UnsupportedOperationError extends Error { code: 'PWRIGHT_UNSUPPORTED'; }
export const errors: { readonly TimeoutError: typeof TimeoutError; readonly UnsupportedOperationError: typeof UnsupportedOperationError };
export const chromium: BrowserType<'chromium'>;
export const firefox: BrowserType<'firefox'>;
export const webkit: BrowserType<'webkit'>;
export const request: APIRequest;
export const selectors: Selectors;
export const devices: { [name: string]: DeviceDescriptor };
export type NativeProfile = 'frame-events' | 'official-conformance-page-frame-events' | 'official-conformance-page-frame-hierarchy' | 'frame-element' | 'official-conformance-page-frame-element' | 'playwright-server-connect-session' | 'playwright-server-connect-close-error' | 'playwright-server-connect-ws-error' | 'playwright-server-connect-headers' | 'default' | 'cdp-session' | 'browser-cdp-session' | 'official-conformance-page-cdp-session' | 'browser-registry' | 'owned-launch' | 'playwright-trace' | 'handles' | 'element-handle-dom' | 'element-handle-select-option-value' | 'element-handle-scroll-into-view' | 'retained-file-input' | 'retained-file-input-combined' | 'retained-target-marking' | 'retained-target-marking-combined' | 'retained-target-prevalidation' | 'retained-target-prevalidation-combined' | 'retained-input-instrumentation' | 'retained-input-instrumentation-combined' | 'retained-call-argument-serialization' | 'retained-call-argument-serialization-combined' | 'retained-serialized-payload-import' | 'retained-serialized-payload-import-combined' | 'retained-serialized-payload-mutation' | 'retained-serialized-payload-mutation-combined' | 'file-input' | 'file-input-concurrent-close-drag-frame-extra-headers' | 'drag-runtime' | 'frame-navigation' | 'frame-navigation-burst-20' | 'response-request' | 'route-header-url-overrides' | 'route-fulfill-cors' | 'route-regexp' | 'route-request-history' | 'context-cookies' | 'route-large-post-data' | 'page-unroute' | 'request-events' | 'request-post-data-json' | 'history-navigation' | 'drag-frame-navigation' | 'extra-headers' | 'drag-extra-headers' | 'frame-extra-headers' | 'concurrent-close' | 'concurrent-close-drag-frame-extra-headers' | 'set-content' | 'page-events' | 'set-content-page-events' | 'set-content-page-events-console' | 'set-content-page-events-console-bindings' | 'device-emulation' | 'touchscreen-input' | 'selector-registration' | 'bypass-csp' | 'api-request' | 'api-request-http-proxy' | 'api-request-socks5-proxy' | 'api-request-socks5-proxy-bypass' | 'api-request-client-certificates' | 'api-request-client-certificates-http-proxy' | 'api-request-client-certificates-socks5-proxy' | 'context-api-request-proxy' | 'init-scripts' | 'official-conformance-page-bindings' | 'official-conformance-page-check' | 'official-conformance-page-check-light-v1' | 'official-conformance-page-dispatch-event' | 'official-conformance-page-goto-burst-20' | 'official-conformance-page-goto-frame-detach' | 'official-conformance-page-route-header-url' | 'official-conformance-page-route-header-url-unroute' | 'official-conformance-page-route-header-url-unroute-cors' | 'official-conformance-page-route-header-url-unroute-cors-regexp' | 'official-conformance-page-route-header-url-unroute-cors-regexp-history' | 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies' | 'official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data' | 'fill-navigation-commit' | 'official-conformance-page-fill-navigation-commit' | 'select-option-navigation-commit' | 'select-option-timeout-progress' | 'official-conformance-page-select-option-timeout-progress' | 'official-conformance-page-select-option-navigation-commit' | 'react-selectors' | 'official-conformance-page-react-selectors' | 'official-conformance-page-retained-scroll' | 'official-conformance-page-retained-wait' | 'official-conformance-page-retained-select-option-value' | 'cdp-handshake-headers' | 'cdp-default-user-agent' | 'official-conformance-page-cdp-handshake-headers' | 'official-conformance-page-cdp-default-user-agent';
export const compatibility: Readonly<{ oracle: string; profile: string; nativeProfile: NativeProfile; browserOwnership: 'external' | 'external-and-owned'; apiCoverage: 'experimental-subset' }>;
