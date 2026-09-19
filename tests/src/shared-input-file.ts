/** Resolve only the historical corpus alias, never read an arbitrary /tmp file. */
export function sharedInputResolver(ownedFile: string): (path: string) => string {
  return path => {
    if (path !== "/tmp/test-file.txt") throw new Error(`Unregistered shared upload fixture: ${path}`);
    return ownedFile;
  };
}
