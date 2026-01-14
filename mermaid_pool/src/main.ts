// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

/**
 * Mermaid renderer pool implementation.
 */

import './styles.css';
import { Pool } from './pool';
import mermaid from 'mermaid';

/**
 * Initialize mermaid and the renderer pool and expose a global API
 * `window.__mermaidPool.submitTask(id, diagram)` which returns a Promise
 * that resolves to the SVG string. This is intended to be used by the
 * Rust test harness via `headless_chrome`.
 */

interface MermaidPoolWindow extends Window {
  __mermaidPool?: {
    submitTask: (id: string, diagram: string) => Promise<string>;
    status: () => unknown;
  };
}

declare const window: MermaidPoolWindow;

// Configure mermaid once at startup
mermaid.initialize({ startOnLoad: false, securityLevel: 'loose' });

// pool size configurable by query parameter ?pool=3
const url = new URL(window.location.href);
const poolSizeStr = url.searchParams.get('pool');
const poolSize = poolSizeStr ? Math.max(1, Math.min(8, Number(poolSizeStr))) : 2;

const pool = new Pool(poolSize);

// Expose the global API used by Rust via evaluate(...) and DOM signaling
window.__mermaidPool = {
  submitTask: (id: string, diagram: string) => {
    // create DOM task placeholder
    const el = document.createElement('div');
    el.id = `task-${id}`;
    el.dataset.state = 'pending';
    el.style.display = 'none'; // keep tasks invisible
    document.body.appendChild(el);

    return pool.submit(id, diagram);
  },
  status: () => ({ poolSize, queueLen: (pool as any).queue.length }),
};

// minimal metrics element
const metrics = document.createElement('pre');
metrics.id = 'mermaid-metrics';
metrics.style.display = 'none';
metrics.textContent = JSON.stringify({ poolSize }, null, 2);
document.body.appendChild(metrics);

// keep console messages visible for debugging in CI logs
console.log('Mermaid pool initialized', { poolSize });
