'use strict';

const { inspect } = require('node:util');

class ConsoleArgument {
  constructor(value) { this._value = value; }
  async jsonValue() { return this._value; }
}

class ConsoleMessage {
  constructor(page, type, text, argumentsJSON = '[]', location = {}) {
    this._page = page;
    this._type = type;
    this._text = text;
    const values = JSON.parse(argumentsJSON);
    if (!Array.isArray(values)) throw new TypeError('Native console arguments are not an array');
    this._args = values.map(value => new ConsoleArgument(value));
    this._location = {
      url: location.url ?? '',
      lineNumber: location.lineNumber ?? 0,
      columnNumber: location.columnNumber ?? 0,
    };
  }

  page() { return this._page; }
  worker() { return null; }
  type() { return this._type; }
  text() { return this._text; }
  args() { return this._args.slice(); }
  location() { return this._location; }
  [inspect.custom]() { return this._text; }
}

module.exports = ConsoleMessage;
