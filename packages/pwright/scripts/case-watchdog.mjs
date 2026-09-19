// Test-supervisor policy, not a Playwright API timeout or production scheduler.
export function createCaseWatchdog({ schedule, cancel, fail, timeoutMs = 60_000 }) {
  let timer;
  let current;
  let count = 0;
  const arm = label => {
    if (timer !== undefined) cancel(timer);
    timer = schedule(() => fail(`case watchdog expired: ${label}`), timeoutMs);
  };
  arm('startup');
  return {
    message(value) {
      if (!value || typeof value !== 'object' || value.event !== 'pwright-case'
        || typeof value.name !== 'string' || value.name.length > 512
        || !['start', 'end'].includes(value.phase)) {
        fail('invalid case watchdog message'); return;
      }
      if (value.phase === 'start') {
        if (current !== undefined || ++count > 128) { fail('invalid case start order or capacity'); return; }
        current = value.name;
        arm(current);
      } else {
        if (current !== value.name) { fail('case end does not match active case'); return; }
        current = undefined;
        arm('between cases or process exit');
      }
    },
    close() { if (timer !== undefined) cancel(timer); timer = undefined; },
  };
}
