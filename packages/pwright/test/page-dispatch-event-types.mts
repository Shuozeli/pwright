import type { ElementHandle, Page, PageDispatchEventOptions } from '@shuozeli/pwright';

declare const page: Page;
declare const element: ElementHandle<HTMLButtonElement>;

const options: PageDispatchEventOptions = { strict: true, timeout: 73 };
const pageDefault: Promise<void> = page.dispatchEvent('button', 'click');
const pageStrict: Promise<void> = page.dispatchEvent('button', 'click', { detail: 2 }, options);
const handle: Promise<void> = element.dispatchEvent('focus', null);
void pageDefault; void pageStrict; void handle;

// @ts-expect-error the selector-based strict option must be Boolean
page.dispatchEvent('button', 'click', {}, { strict: 'yes' });
// @ts-expect-error the event type must be a string
page.dispatchEvent('button', 42);
// @ts-expect-error ElementHandle.dispatchEvent has no selector argument
element.dispatchEvent('button', 'click', {});
