import { EventEmitter } from 'node:events';

export interface ConnectOverCDPOptions { timeout?: number; }
export interface BrowserNewContextOptions { baseURL?: string; extraHTTPHeaders?: { [key: string]: string }; }
type WithoutHandles<T> = T extends JSHandle ? never : T extends object ? { [K in keyof T]: WithoutHandles<T[K]> } : T;
export type Unboxed<T> = T extends ElementHandle<infer E> ? E : T extends JSHandle<infer V> ? V : T extends WithoutHandles<T> ? T : T extends object ? { [K in keyof T]: Unboxed<T[K]> } : T;
export type PageFunction<Arg, Result> = string | ((arg: Unboxed<Arg>) => Result | Promise<Result>);
export type SmartHandle<T> = T extends Node ? ElementHandle<T> : JSHandle<T>;
/** Explicit handles native profile; ElementHandle actions remain unsupported. */
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
  ownerFrame(): Promise<Frame | null>;
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
export interface BrowserType {
  name(): 'chromium';
  connectOverCDP(endpointURL: string, options?: ConnectOverCDPOptions): Promise<Browser>;
  connectOverCDP(options: ConnectOverCDPOptions & { endpointURL: string }): Promise<Browser>;
  launch(): Promise<never>;
  launchPersistentContext(): Promise<never>;
  connect(): Promise<never>;
}
export interface Browser extends EventEmitter {
  browserType(): BrowserType;
  isConnected(): boolean;
  version(): string;
  contexts(): BrowserContext[];
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
  newPage(): Promise<never>;
}
export interface BrowserContext extends EventEmitter {
  setExtraHTTPHeaders(headers: { [key: string]: string }): Promise<void>;
  browser(): Browser;
  pages(): Page[];
  newPage(): Promise<Page>;
  close(): Promise<void>;
}
export interface Page extends EventEmitter, SemanticLocators {
  evaluateHandle<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<SmartHandle<Awaited<Result>>>;
  evaluateHandle<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<SmartHandle<Awaited<Result>>>;
  setInputFiles(selector: string, files: InputFiles, options?: SelectorSetInputFilesOptions): Promise<void>;
  setExtraHTTPHeaders(headers: { [key: string]: string }): Promise<void>;
  readonly keyboard: Keyboard;
  readonly mouse: Mouse;
  context(): BrowserContext;
  mainFrame(): Frame;
  frame(name: string): Frame | null;
  frame(options: { name?: string; url?: string | RegExp | ((url: URL) => boolean) }): Frame | null;
  frames(): Frame[];
  screenshot(options?: ScreenshotOptions): Promise<Buffer>;
  goto(url: string, options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number; referer?: string }): Promise<Response | null>;
  reload(options?: { waitUntil?: 'commit' | 'domcontentloaded' | 'load' | 'networkidle'; timeout?: number }): Promise<Response | null>;
  title(): Promise<string>;
  content(): Promise<string>;
  url(): string;
  evaluate<Result, Arg>(pageFunction: PageFunction<Arg, Result>, arg: Arg): Promise<Awaited<Result>>;
  evaluate<Result>(pageFunction: string | (() => Result | Promise<Result>)): Promise<Awaited<Result>>;
  close(options?: { runBeforeUnload?: false }): Promise<void>;
  isClosed(): boolean;
  locator(selector: string, options?: LocatorFilterOptions): Locator;
}
export interface ScreenshotOptions {
  type?: 'png' | 'jpeg';
  quality?: number;
  fullPage?: boolean;
  clip?: { x: number; y: number; width: number; height: number };
  scale?: 'device' | 'css';
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
  locator(selector: string, options?: LocatorFilterOptions): Locator;
  url(): string;
  title(): Promise<string>;
  content(): Promise<string>;
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
  hasText?: string;
  hasNotText?: string;
  has?: Locator;
  hasNot?: Locator;
  visible?: boolean;
}
export interface LocatorReadOptions { timeout?: number; }
export type KeyboardModifier = 'Alt' | 'Control' | 'ControlOrMeta' | 'Meta' | 'Shift';
export interface LocatorPointerOptions extends LocatorReadOptions {
  force?: boolean; modifiers?: KeyboardModifier[]; position?: { x: number; y: number }; trial?: boolean; noWaitAfter?: boolean;
}
export interface LocatorClickOptions extends LocatorPointerOptions {
  button?: 'left' | 'right' | 'middle'; clickCount?: number; delay?: number;
}
export interface LocatorFillOptions extends LocatorReadOptions { force?: boolean; noWaitAfter?: boolean; }
export interface LocatorPressOptions extends LocatorReadOptions { delay?: number; noWaitAfter?: boolean; }
export interface LocatorPressSequentiallyOptions extends LocatorReadOptions { delay?: number; noWaitAfter?: boolean; }
export interface LocatorCheckedOptions extends LocatorFillOptions { position?: { x: number; y: number }; trial?: boolean; }
export interface SelectOptionDescriptor { value?: string; label?: string; index?: number; }
export interface FilePayload { name: string; mimeType: string; buffer: Buffer; }
export type InputFiles = string | ReadonlyArray<string> | FilePayload | ReadonlyArray<FilePayload>;
export interface SetInputFilesOptions extends LocatorReadOptions {
  /** @deprecated This option has no effect. */
  noWaitAfter?: boolean;
}
export interface SelectorSetInputFilesOptions extends SetInputFilesOptions { strict?: boolean; }
export interface Locator extends SemanticLocators {
  evaluate<Result, Arg, E extends Element = HTMLElement | SVGElement>(pageFunction: (element: E, arg: Arg) => Result | Promise<Result>, arg: Arg, options?: LocatorReadOptions): Promise<Awaited<Result>>;
  evaluate<Result, E extends Element = HTMLElement | SVGElement>(pageFunction: string | ((element: E) => Result | Promise<Result>), arg?: undefined, options?: LocatorReadOptions): Promise<Awaited<Result>>;
  page(): Page;
  first(): Locator;
  last(): Locator;
  nth(index: number): Locator;
  and(other: Locator): Locator;
  or(other: Locator): Locator;
  locator(selector: string | Locator, options?: LocatorFilterOptions): Locator;
  filter(options?: LocatorFilterOptions): Locator;
  count(): Promise<number>;
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
  click(options?: LocatorClickOptions): Promise<void>;
  dblclick(options?: Omit<LocatorClickOptions, 'clickCount'>): Promise<void>;
  fill(value: string, options?: LocatorFillOptions): Promise<void>;
  clear(options?: LocatorFillOptions): Promise<void>;
  press(key: string, options?: LocatorPressOptions): Promise<void>;
  pressSequentially(text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  /** @deprecated Use pressSequentially instead. */
  type(text: string, options?: LocatorPressSequentiallyOptions): Promise<void>;
  check(options?: LocatorCheckedOptions): Promise<void>;
  uncheck(options?: LocatorCheckedOptions): Promise<void>;
  setChecked(checked: boolean, options?: LocatorCheckedOptions): Promise<void>;
  selectOption(values: string | SelectOptionDescriptor | string[] | SelectOptionDescriptor[] | null, options?: LocatorFillOptions): Promise<string[]>;
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
  frame(): Frame;
  body(): Promise<never>;
  text(): Promise<never>;
  json(): Promise<never>;
  finished(): Promise<never>;
}
declare class TimeoutError extends Error {}
declare class UnsupportedOperationError extends Error { code: 'PWRIGHT_UNSUPPORTED'; }
export const errors: { readonly TimeoutError: typeof TimeoutError; readonly UnsupportedOperationError: typeof UnsupportedOperationError };
export const chromium: BrowserType;
export type NativeProfile = 'default' | 'handles' | 'retained-file-input' | 'retained-file-input-combined' | 'retained-target-marking' | 'retained-target-marking-combined' | 'retained-target-prevalidation' | 'retained-target-prevalidation-combined' | 'retained-input-instrumentation' | 'retained-input-instrumentation-combined' | 'retained-call-argument-serialization' | 'retained-call-argument-serialization-combined' | 'retained-serialized-payload-import' | 'retained-serialized-payload-import-combined' | 'retained-serialized-payload-mutation' | 'retained-serialized-payload-mutation-combined' | 'file-input' | 'file-input-concurrent-close-drag-frame-extra-headers' | 'drag-runtime' | 'frame-navigation' | 'drag-frame-navigation' | 'extra-headers' | 'drag-extra-headers' | 'frame-extra-headers' | 'drag-frame-extra-headers' | 'concurrent-close' | 'concurrent-close-drag-frame-extra-headers';
export const compatibility: Readonly<{ oracle: string; profile: string; nativeProfile: NativeProfile; browserOwnership: 'external'; apiCoverage: 'experimental-subset' }>;
