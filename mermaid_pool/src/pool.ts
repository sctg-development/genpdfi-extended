// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

/**
 * Mermaid renderer pool implementation.
 *
 * The pool creates N renderer slots. Each renderer keeps a hidden DIV
 * container and renders diagrams into that DIV. After rendering, the
 * renderer extracts the produced SVG and resolves the task promise.
 *
 * This implementation is defensive: mermaid's exact API may differ across
 * versions, so we attempt to obtain the SVG using multiple strategies.
 */

import mermaid from 'mermaid';

/** Simple unique id helper */
function uid(prefix = '') {
  return `${prefix}${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * Renderer: holds a hidden container and exposes render(diagram) -> Promise<svg>
 */
export class Renderer {
  id: string;
  container: HTMLDivElement;
  busy: boolean;

  constructor(id?: string) {
    this.id = id ?? uid('r-');
    this.container = document.createElement('div');
    this.container.style.position = 'absolute';
    this.container.style.left = '-10000px';
    this.container.style.top = '-10000px';
    this.container.style.width = '0px';
    this.container.style.height = '0px';
    this.container.dataset.rendererId = this.id;
    document.body.appendChild(this.container);
    this.busy = false;
  }

  /**
   * Render the mermaid diagram and return the SVG string. JSDoc above
   * provides guidance for usage. We attempt multiple extraction strategies
   * to maximize compatibility across mermaid versions.
   *
   * @param diagram - mermaid source string
   */
  async render(diagram: string): Promise<string> {
    this.busy = true;
    this.container.innerHTML = ''; // clear previous content

    // strategy 1: try mermaid.render which may return svg synchronously
    try {
      // mermaid.render may be sync or async depending on version.
      // We attempt to call it and use whatever it returns.
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore -- conservative call; types vary by mermaid version
      const maybe = mermaid.render(uid('m-'), diagram);
      if (typeof maybe === 'string') {
        // got svg string
        this.busy = false;
        return maybe;
      }
      if (maybe && typeof (maybe as any).then === 'function') {
        const resolved = await (maybe as any);
        if (typeof resolved === 'string') {
          this.busy = false;
          return resolved;
        }
      }
    } catch (err) {
      // fall through to other extraction strategies
      // console.warn('mermaid.render failed (try fallback)', err);
    }

    // strategy 2: insert a '.mermaid' element into the container and run mermaid.init
    try {
      const wrapper = document.createElement('div');
      wrapper.className = 'mermaid';
      wrapper.textContent = diagram;
      this.container.appendChild(wrapper);

      // Run mermaid initialization - target the wrapper element explicitly which
      // avoids surprises when multiple renderers share the same container.
      try {
        // mermaid.init optionally accepts configuration, some versions accept a parent
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        mermaid.init(undefined, wrapper);
      } catch (initErr) {
        // If init fails, fall through and attempt parse/render
      }

      // Give mermaid a slightly larger tick to perform async DOM updates
      await new Promise((r) => setTimeout(r, 50));

      const svgEl = wrapper.querySelector('svg');
      if (svgEl) {
        const svgOuter = svgEl.outerHTML;
        this.busy = false;
        return svgOuter;
      }
    } catch (err) {
      // continue to next strategy
    }

    // strategy 3: last resort - attempt mermaid.parse & mermaid.render callback style
    try {
      const id = uid('mr-');
      const mount = document.createElement('div');
      this.container.appendChild(mount);
      return await new Promise<string>((resolve, reject) => {
        let done = false;
        try {
          // @ts-ignore
          const res = mermaid.render(id, diagram, (svgCode: string) => {
            if (!done) {
              done = true;
              resolve(svgCode);
            }
          }, mount as any);

          if (!done && typeof res === 'string') {
            done = true;
            resolve(res);
          }

          // ensure we time out if render never completes
          setTimeout(() => {
            if (!done) {
              done = true;
              reject(new Error('mermaid render timed out'));
            }
          }, 5000);
        } catch (e) {
          if (!done) {
            done = true;
            reject(e);
          }
        }
      }).finally(() => {
        this.busy = false;
      });
    } catch (err) {
      this.busy = false;
      throw err;
    }
  }
}

/** Pool of renderers with a small queue and telemetry. */
export class Pool {
  renderers: Renderer[];
  queue: Array<{ id: string; diagram: string; resolve: (s: string) => void; reject: (e: any) => void }>;

  constructor(poolSize = 2) {
    this.renderers = new Array(poolSize).fill(null).map((_, i) => new Renderer(`r-${i}`));
    this.queue = [];
  }

  /**
   * Submit a diagram to the pool and return a Promise that resolves to the SVG string.
   */
  submit(taskId: string, diagram: string): Promise<string> {
    return new Promise<string>((resolve, reject) => {
      this.queue.push({ id: taskId, diagram, resolve, reject });
      this.processQueue();
    });
  }

  private async processQueue() {
    // find idle renderers
    while (this.queue.length > 0) {
      const renderer = this.renderers.find((r) => !r.busy);
      if (!renderer) break; // no idle, wait for them to finish

      const task = this.queue.shift()!;
      // run it
      renderer.busy = true;
      (async () => {
        try {
          const started = performance.now();
          const svg = await renderer.render(task.diagram);
          const dur = performance.now() - started;
          // write result into DOM task element
          const taskEl = document.getElementById(`task-${task.id}`);
          if (taskEl) {
            taskEl.dataset.state = 'done';
            // save the SVG as textContent to avoid attribute length limits
            taskEl.textContent = svg;
            taskEl.dataset.duration = dur.toFixed(3);
          }
          task.resolve(svg);
        } catch (err) {
          const taskEl = document.getElementById(`task-${task.id}`);
          if (taskEl) {
            taskEl.dataset.state = 'error';
            taskEl.textContent = String(err);
          }
          task.reject(err);
        } finally {
          renderer.busy = false;
        }
      })();
    }
  }
}
