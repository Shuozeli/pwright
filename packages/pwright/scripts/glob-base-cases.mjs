// Shared deterministic corpus for the Rust parser diagnostic and actual N-API port.
export function globBaseCases() {
  const bases = [undefined, '', 'not a URL', 'https://EXAMPLE.test/root/', 'http://a.test:80/path', 'https://user:pass@a.test:444/root?q=1#hash', 'file:///tmp/root/', 'about:blank', 'data:text/plain,x', 'https://例子.test/目录/'];
  const globs = ['', '*', '**', '**/a', 'a', './a', '../a', '/a', '?q=1', '#hash', 'a?q=1#hash', 'a/**/b', 'a/{x,y}', '{{a,b},c}', 'https://EXAMPLE.test/Path', 'http*://Example.test/**', '{http,https}://EXAMPLE.test/a', '//EXAMPLE.test/a', 'https://EXAMPLE.test', 'about:blank', 'data:text/plain,a?b', 'chrome://version', 'edge://version', 'file:///tmp/a', 'ABOUT:blank', 'mailto:abc@example.test', 'a\\?', 'a\\\\?', '**/a\\\\?', 'a/./b/../c', 'a//b', 'a/%2E%2E/b', 'a b', '你好/🦀', '$$', '$&', '$`', "$'", '$1', 'a/$&/b', 'a/$`/b', "a/$'/b", '$_2_$/$_3_$', 'https://ΟΣ.test/ΟΣ', 'https://İ.test/İ'];
  const cases = bases.flatMap(baseURL => globs.map(glob => ({ baseURL, glob })));
  const alphabet = ['a', '?', '/', '$', '&', "'", '`'];
  let words = [''];
  for (let length = 1; length <= 3; length++) {
    words = words.flatMap(prefix => alphabet.map(character => prefix + character));
    for (const baseURL of [undefined, 'https://EXAMPLE.test/root/', 'not a URL'])
      cases.push(...words.map(glob => ({ baseURL, glob })));
  }
  return cases;
}
