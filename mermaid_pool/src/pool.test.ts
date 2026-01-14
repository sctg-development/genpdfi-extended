// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';

// polyfill missing SVG getBBox in jsdom which mermaid expects
if (typeof (SVGElement as any) !== 'undefined' && !('getBBox' in SVGElement.prototype)) {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  SVGElement.prototype.getBBox = function () {
    return { x: 0, y: 0, width: 100, height: 20, y: 0, toJSON: () => ({}) };
  };
}

// Suppress non-actionable unhandled rejections from the mermaid runtime about
// unknown diagram types (these appear when we intentionally send invalid input
// and mermaid emits errors asynchronously). We make sure to only swallow the
// specific mermaid UnknownDiagramError messages so other rejections still fail tests.
const _onUnhandledRejection = (err: any) => {
  const msg = err && err.message ? err.message : String(err);
  if (typeof msg === 'string' && msg.includes('No diagram type detected')) {
    // ignore this one
    return;
  }
  // rethrow so Vitest reports unexpected unhandled rejections
  throw err;
};
process.on('unhandledRejection', _onUnhandledRejection);

// Additionally, swallow window-level error/unhandledrejection events that mermaid
// sometimes raises asynchronously for invalid diagrams so Vitest doesn't mark the
// test run as containing unhandled errors. We only suppress the specific mermaid
// message to avoid hiding unrelated issues.
if (typeof window !== 'undefined' && window) {
  window.addEventListener('error', (ev: any) => {
    const msg = ev && ev.message ? ev.message : '';
    if (typeof msg === 'string' && msg.includes('No diagram type detected')) {
      ev.preventDefault();
    }
  });
  window.addEventListener('unhandledrejection', (ev: any) => {
    const reason = ev && ev.reason ? ev.reason : undefined;
    const msg = reason && reason.message ? reason.message : String(reason);
    if (typeof msg === 'string' && msg.includes('No diagram type detected')) {
      ev.preventDefault();
    }
  });
}

// Note: we intentionally do not remove the listener here; Vitest runs tests in a
// short-lived process and the handlers are useful for the duration of the suite.

import { Pool, Renderer } from './pool';
import mermaid from 'mermaid';

describe('mermaid pool', () => {
  beforeEach(() => {
    // reset DOM between tests
    document.body.innerHTML = '';
  });

  it('renders a valid diagram and marks task done', async () => {
    const pool = new Pool(1);
    const id = 'valid';
    const taskEl = document.createElement('div');
    taskEl.id = `task-${id}`;
    document.body.appendChild(taskEl);

    const svg = await pool.submit(id, 'graph TB\na-->b');

    expect(svg).toEqual(expect.stringContaining('<svg'));

    const task = document.getElementById(`task-${id}`)!;
    expect(task.dataset.state).toBe('done');
    expect((task.textContent || '')).toContain('<svg');
  });

  it('invalid syntax returns an error and marks task error', async () => {
    const pool = new Pool(1);
    const id = 'invalid';
    const taskEl = document.createElement('div');
    taskEl.id = `task-${id}`;
    document.body.appendChild(taskEl);

    // To avoid mermaid emitting asynchronous rejections that appear as
    // unhandled to Vitest we temporarily stub `mermaid.parse` so the
    // parse error is thrown synchronously and handled by our code under test.
    const origParse = (mermaid as any).parse;
    try {
      (mermaid as any).parse = (s: string) => {
        throw new Error('No diagram type detected matching given configuration for text: ' + s);
      };

      await expect(pool.submit(id, 'grph TB\na-->b')).rejects.toThrow(/Mermaid failed to compile/);

      const task = document.getElementById(`task-${id}`)!;
      expect(task.dataset.state).toBe('error');
      expect((task.textContent || '')).toMatch(/^Mermaid failed to compile/);
    } finally {
      (mermaid as any).parse = origParse;
    }
  });

  it('Renderer.render resolves for good diagrams and rejects on invalid syntax', async () => {
    const r = new Renderer('tst');
    const svg = await r.render('graph TB\na-->b');
    expect(svg).toEqual(expect.stringContaining('<svg'));

    const origParse = (mermaid as any).parse;
    try {
      (mermaid as any).parse = (s: string) => {
        throw new Error('No diagram type detected matching given configuration for text: ' + s);
      };
      await expect(r.render('grph TB\na-->b')).rejects.toThrow(/Mermaid failed to compile/);
    } finally {
      (mermaid as any).parse = origParse;
    }
  });
});
