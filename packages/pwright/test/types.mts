import { chromium, errors, compatibility, type Browser, type BrowserContext, type Page, type Response, type Locator } from '@shuozeli/pwright';

const byUrl: Promise<Browser> = chromium.connectOverCDP('http://127.0.0.1:9222', { timeout: 5000 });
const byOptions: Promise<Browser> = chromium.connectOverCDP({ endpointURL: 'http://127.0.0.1:9222' });
void byUrl; void byOptions;
declare const browser: Browser;
const configuredContext: Promise<BrowserContext> = browser.newContext({ baseURL: 'https://example.com/path/' });
const configuredHeaders: Promise<BrowserContext> = browser.newContext({ extraHTTPHeaders: { 'X-Layer': 'creation' } });
void configuredHeaders;
// @ts-expect-error creation header values must be strings
browser.newContext({ extraHTTPHeaders: { 'X-Layer': 1 } });
const contextMetadataBytes: number = browser.pwrightContextMetadataMetrics().bytes;
void configuredContext; void contextMetadataBytes;
// @ts-expect-error baseURL must be a string
browser.newContext({ baseURL: 42 });
const page: Page = browser.contexts()[0].pages()[0];
const objectHandle: Promise<import('@shuozeli/pwright').JSHandle<{ answer: number }>> = page.evaluateHandle(() => ({ answer: 42 }));
const elementHandle: Promise<import('@shuozeli/pwright').ElementHandle<HTMLBodyElement>> = page.mainFrame().evaluateHandle(() => document.body as HTMLBodyElement);
void objectHandle; void elementHandle;
declare const retainedObject: import('@shuozeli/pwright').JSHandle<{ answer: number }>;
declare const retainedNode: import('@shuozeli/pwright').ElementHandle<HTMLDivElement>;
const retainedOwnerFrame: Promise<import('@shuozeli/pwright').Frame | null> = retainedNode.ownerFrame();
void retainedOwnerFrame;
// @ts-expect-error primitive/object JSHandle has no ElementHandle owner-frame method
retainedObject.ownerFrame();
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
const screenshot: Promise<Buffer> = page.screenshot({ type: 'jpeg', quality: 80, fullPage: true, timeout: 5000 });
void screenshot;
// @ts-expect-error Playwright public screenshots do not expose CDP WebP
page.screenshot({ type: 'webp' });
const frameUrl: string = page.mainFrame().url();
const keyboardDown: Promise<void> = page.keyboard.down('Shift');
const keyboardUp: Promise<void> = page.keyboard.up('Shift');
const keyboardPress: Promise<void> = page.keyboard.press('KeyA', { delay: 0.5 });
const keyboardInsert: Promise<void> = page.keyboard.insertText('你好');
const keyboardType: Promise<void> = page.keyboard.type('hello', { delay: 10 });
void keyboardDown; void keyboardUp; void keyboardPress; void keyboardInsert; void keyboardType;
// @ts-expect-error Keyboard has no Locator timeout option
page.keyboard.press('A', { timeout: 100 });
const locator: Locator = page.locator('button').filter({ hasNotText: 'Cancel' }).first();
const locatorCount: Promise<number> = locator.count();
const locatorText: Promise<string | null> = locator.textContent({ timeout: 1000 });
const locatorTexts: Promise<string[]> = locator.allTextContents();
const locatorAttribute: Promise<string | null> = locator.getAttribute('missing');
const locatorState: Promise<boolean> = locator.isVisible();
const click: Promise<void> = locator.click({ modifiers: ['ControlOrMeta'], position: { x: 2, y: 3 }, trial: true });
const fill: Promise<void> = locator.fill('value', { force: true });
const press: Promise<void> = locator.press('Enter', { delay: 10 });
const selected: Promise<string[]> = locator.selectOption([{ value: 'a' }, { index: 1 }]);
const checked: Promise<void> = locator.setChecked(true);
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
void click; void fill; void press; void selected; void checked;
// @ts-expect-error Playwright selection arrays are homogeneous
locator.selectOption(['a', { index: 1 }]);
const nested: Locator = page.mainFrame().locator('form').locator(locator);
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
const ownership: 'external' = compatibility.browserOwnership;
const title: Promise<string> = page.title();
const mouseMove: Promise<void> = page.mouse.move(1.5, 2, { steps: 3 });
const mouseDown: Promise<void> = page.mouse.down({ button: 'right', clickCount: 2 });
const mouseUp: Promise<void> = page.mouse.up();
const mouseClick: Promise<void> = page.mouse.click(1, 2, { delay: 0.5, clickCount: 3 });
const mouseDouble: Promise<void> = page.mouse.dblclick(1, 2, { button: 'middle' });
const mouseWheel: Promise<void> = page.mouse.wheel(-1.5, 2.5);
void mouseMove; void mouseDown; void mouseUp; void mouseClick; void mouseDouble; void mouseWheel;
// @ts-expect-error double-click count is always two
page.mouse.dblclick(1, 2, { clickCount: 3 });
// @ts-expect-error raw Mouse does not perform actionability checks
page.mouse.click(1, 2, { force: true });
// @ts-expect-error Mouse button is a closed union
page.mouse.down({ button: 'back' });
const content: Promise<string> = page.mainFrame().content();
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
void reloaded;
// @ts-expect-error reload does not accept a replacement URL or referer
page.reload({ referer: 'https://example.com/' });
const headers: Record<string, string> = response.headers();
const status: number = response.status();
const ok: boolean = response.ok();
const responsePage: Page = response.frame().page();
void identical; void connected; void error; void ownership;
void title; void content; void navigation; void headers; void status; void ok; void responsePage;
// @ts-expect-error headers are not part of the initial profile
chromium.connectOverCDP('http://127.0.0.1:9222', { headers: {} });
// @ts-expect-error waitUntil accepts only the four implemented lifecycle modes
page.goto('about:blank', { waitUntil: 'ready' });
