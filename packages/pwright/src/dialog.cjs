'use strict';

// The native owner is an opaque, single-use Rust DialogAction. This facade
// only presents Playwright's language API and maps the typed boundary error.
module.exports = ({ mapError }) => class Dialog {
  constructor(page, native, type, message, defaultValue) {
    this._page = page;
    this._native = native;
    this._type = type;
    this._message = message;
    this._defaultValue = defaultValue;
  }

  page() { return this._page; }
  type() { return this._type; }
  message() { return this._message; }
  defaultValue() { return this._defaultValue; }

  async accept(promptText) {
    if (promptText !== undefined && typeof promptText !== 'string')
      throw new TypeError('dialog.accept: promptText must be a string');
    try {
      await this._page._context._browser._session.handleDialog(
        this._native, true, promptText,
      );
    } catch (error) { throw mapError(error, 'dialog.accept'); }
  }

  async dismiss() {
    try {
      await this._page._context._browser._session.handleDialog(
        this._native, false, undefined,
      );
    } catch (error) { throw mapError(error, 'dialog.dismiss'); }
  }

  async _autoHandle() {
    try {
      if (this._type === 'beforeunload') await this.accept();
      else await this.dismiss();
    } catch {
      // Page/browser teardown can retire the native owner first.
    }
  }
};
