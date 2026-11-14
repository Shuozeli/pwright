//! Opt-in React component query for the pinned Playwright selector corpus.
//! The plan and installation are Rust-owned; the browser helper reads the live
//! React tree in the main world because Fiber expandos are world-local.

pub(crate) const SYMBOL: &str = "pwright.react-selector/1";
pub(crate) const VERSION: u32 = 3;

pub(crate) fn install_script() -> String {
    format!(
        r#"(() => {{
  const key = Symbol.for({symbol:?});
  if (!globalThis[key] || globalThis[key].version !== {version})
    Object.defineProperty(globalThis, key, {{ value: {RUNTIME}, configurable: true }});
  return true;
}})()"#,
        symbol = SYMBOL,
        version = VERSION,
    )
}

// This is separate from the default web-platform selector runtime. Inactive
// profiles neither install it nor retain React-specific browser code.
const RUNTIME: &str = r#"(() => {
  const maxNodes = 20000;
  const parse = source => {
    const first = source.indexOf('[');
    const name = (first < 0 ? source : source.slice(0, first)).trim();
    const attributes = [];
    let offset = first < 0 ? source.length : first;
    while (offset < source.length) {
      while (/\s/.test(source[offset] || '') && offset < source.length) ++offset;
      if (offset >= source.length) break;
      if (source[offset] !== '[') throw new Error('Invalid React attribute selector');
      let end = offset + 1;
      let quote = '';
      let regex = false;
      let escaped = false;
      for (; end < source.length; ++end) {
        const char = source[end];
        if (escaped) { escaped = false; continue; }
        if (char === '\\') { escaped = true; continue; }
        if (quote) { if (char === quote) quote = ''; continue; }
        if (regex) { if (char === '/') regex = false; continue; }
        if (char === '"' || char === "'") { quote = char; continue; }
        if (char === '/') { regex = true; continue; }
        if (char === ']') break;
      }
      if (end >= source.length) throw new Error('Unclosed React attribute selector');
      const body = source.slice(offset + 1, end).trim();
      const match = /^([\w.]+)\s*(?:(\*=|\^=|\$=|\|=|~=|=)\s*("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\/(?:\\.|[^\/])*\/[gimsuy]*|true|false|-?\d+(?:\.\d+)?)\s*([iIsS])?)?$/.exec(body);
      if (!match) throw new Error('Invalid React attribute selector');
      if (attributes.length >= 256) throw new Error('React attribute selector limit exceeded');
      let value;
      if (match[3] !== undefined) {
        const raw = match[3];
        if (raw[0] === '"') value = JSON.parse(raw);
        else if (raw[0] === "'") value = raw.slice(1, -1).replace(/\\(['\\])/g, '$1');
        else if (raw[0] === '/') {
          const last = raw.lastIndexOf('/');
          value = new RegExp(raw.slice(1, last), raw.slice(last + 1));
        } else if (raw === 'true' || raw === 'false') value = raw === 'true';
        else value = Number(raw);
      }
      attributes.push({ path: match[1].split('.'), op: match[2] || 'truthy', value, sensitive: !/[iI]/.test(match[4] || '') });
      offset = end + 1;
    }
    const invalidName = /[^\w\u0080-\uFFFF-]/.exec(name);
    if (invalidName) {
      const position = source.indexOf(name) + invalidName.index;
      throw new Error(`Error while parsing selector \`${source}\` - unexpected symbol "${invalidName[0]}" at position ${position}`);
    }
    if (!name && !attributes.length)
      throw new Error(`Error while parsing selector \`${source}\` - selector cannot be empty`);
    return { name, attributes };
  };
  const matchesAttribute = (props, attr) => {
    let actual = props;
    for (const token of attr.path) actual = actual == null ? undefined : actual[token];
    if (attr.op === 'truthy') return !!actual;
    if (attr.value instanceof RegExp)
      return attr.op === '=' && typeof actual === 'string' && !!actual.match(attr.value);
    let expected = attr.value;
    if (!attr.sensitive && typeof actual === 'string' && typeof expected === 'string') {
      actual = actual.toUpperCase();
      expected = expected.toUpperCase();
    }
    if (attr.op === '=') return actual === expected;
    if (typeof actual !== 'string' || typeof expected !== 'string') return false;
    if (attr.op === '*=') return actual.includes(expected);
    if (attr.op === '^=') return actual.startsWith(expected);
    if (attr.op === '$=') return actual.endsWith(expected);
    if (attr.op === '|=') return actual === expected || actual.startsWith(expected + '-');
    if (attr.op === '~=') return actual.split(' ').includes(expected);
    return false;
  };
  const inside = (scope, element) => {
    for (let current = element; current;) {
      if (scope.contains(current)) return true;
      const root = current.getRootNode();
      current = root instanceof ShadowRoot ? root.host : null;
    }
    return false;
  };
  const roots = document => {
    const result = [];
    const pending = [{ walker: document.createTreeWalker(document, NodeFilter.SHOW_ELEMENT), started: false }];
    let visited = 0;
    while (pending.length) {
      const frame = pending[pending.length - 1];
      if (frame.started) {
        if (!frame.walker.nextNode()) {
          pending.pop();
          continue;
        }
      } else frame.started = true;
      if (++visited > maxNodes) throw new Error('React selector DOM traversal limit exceeded');
      const node = frame.walker.currentNode;
      const key = Object.keys(node).find(key => key.startsWith('__reactContainer') && node[key] !== null);
      if (key) result.push(node[key].stateNode.current);
      else if (Object.prototype.hasOwnProperty.call(node, '_reactRootContainer') && node._reactRootContainer !== null)
        result.push(node._reactRootContainer._internalRoot.current);
      if (node instanceof Element && node.hasAttribute('data-reactroot')) {
        for (const key of Object.keys(node)) {
          if (key.startsWith('__reactInternalInstance') || key.startsWith('__reactFiber')) result.push(node[key]);
        }
      }
      if (node instanceof Element && node.shadowRoot)
        pending.push({ walker: document.createTreeWalker(node.shadowRoot, NodeFilter.SHOW_ELEMENT), started: false });
    }
    return result;
  };
  const componentName = node => {
    const type = node.type || node._currentElement?.type;
    if (typeof type === 'string') return type;
    if (typeof type === 'function') return type.displayName || type.name || 'Anonymous';
    if (type && typeof type === 'object')
      return type.displayName || (type.render && (type.render.displayName || type.render.name)) || '';
    return '';
  };
  const children = node => {
    if (node.child) {
      const result = [];
      for (let child = node.child; child; child = child.sibling) {
        if (result.length >= maxNodes) throw new Error('React child list limit exceeded');
        result.push(child);
      }
      return result;
    }
    if (!node._currentElement) return [];
    const known = child => child &&
      (typeof child._currentElement?.type === 'function' || typeof child._currentElement?.type === 'string');
    if (node._renderedComponent) return known(node._renderedComponent) ? [node._renderedComponent] : [];
    if (!node._renderedChildren) return [];
    const keys = Object.keys(node._renderedChildren);
    if (keys.length > maxNodes) throw new Error('React child list limit exceeded');
    return keys.map(key => node._renderedChildren[key]).filter(known);
  };
  const query = (scope, selector) => {
    const { name, attributes } = parse(selector);
    const matches = [];
    let visited = 0;
    const walk = (node, depth = 0) => {
      if (++visited > maxNodes) throw new Error('React selector component traversal limit exceeded');
      if (depth > 256) throw new Error('React selector depth limit exceeded');
      const slot = matches.length;
      matches.push(null);
      const descendants = children(node).map(child => walk(child, depth + 1));
      const element = node.stateNode || node._hostNode || node._renderedComponent?._hostNode;
      const elements = element instanceof Element ? [element] : descendants.flat();
      if ((!name || componentName(node) === name) && elements.every(element => inside(scope, element))) {
        const raw = node.memoizedProps || node._currentElement?.props;
        const props = raw && typeof raw === 'object' ? { ...raw } : {};
        delete props.children;
        const key = node.key ?? node._currentElement?.key;
        if (key !== undefined && key !== null) props.key = key;
        if (attributes.every(attribute => matchesAttribute(props, attribute)))
          matches[slot] = elements;
      }
      return elements;
    };
    for (const root of roots(scope.ownerDocument || scope)) walk(root);
    const result = new Set();
    for (const elements of matches)
      if (elements) for (const element of elements) result.add(element);
    return [...result];
  };
  return Object.freeze({ version: 3, query });
})()"#;

#[cfg(test)]
mod tests {
    #[test]
    fn installation_is_explicitly_versioned_and_bounded() {
        // Arrange
        let script = super::install_script();
        // Act
        let symbol = script.contains(super::SYMBOL);
        // Assert
        assert!(symbol);
        assert!(script.contains("version: 3"));
        assert!(script.contains("maxNodes = 20000"));
        assert!(script.len() < 16 * 1024);
    }
}
