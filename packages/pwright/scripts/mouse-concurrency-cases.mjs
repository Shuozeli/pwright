import assert from 'node:assert/strict';

// Submit in one JS turn. Awaiting between calls would hide start-order races.
export function mouseConcurrencyCases() {
  const scenarios = [
    ['move-down-move-up', page => [
      page.mouse.move(20, 30), page.mouse.down(), page.mouse.move(100, 120), page.mouse.up(),
    ]],
    ['keyboard-click-keyboard', page => [
      page.keyboard.down('Shift'), page.mouse.click(100, 120), page.keyboard.up('Shift'),
    ]],
    ['two-button-state', page => [
      page.mouse.down(), page.mouse.down({ button: 'right' }),
      page.mouse.up({ button: 'right' }), page.mouse.move(100, 120), page.mouse.up(),
    ]],
  ];
  return scenarios.flatMap(([name, submit]) => Array.from({ length: 5 }, (_, index) => ({
    id: `same-turn-${name}-${index + 1}`,
    async run(page) {
      const outcomes = await Promise.allSettled(submit(page));
      for (const outcome of outcomes) assert.equal(outcome.status, 'fulfilled',
        outcome.status === 'rejected' ? String(outcome.reason) : undefined);
      return { outcomes, snapshot: await page.evaluate(() => window.mouseSnapshot()) };
    },
  })));
}
