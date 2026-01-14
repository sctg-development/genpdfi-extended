import { defineConfig } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';

// Vite config tuned to produce a single file bundle that can be embedded into the
// Rust tests via `include_str!` (plugin inlines JS/CSS into a single HTML).
export default defineConfig({
  build: {
    target: 'es2022',
    outDir: 'dist',
    minify: true,
    rollupOptions: {
      output: {
        manualChunks: undefined,
      },
    },
  },
  plugins: [viteSingleFile()],
});
