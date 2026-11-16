import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  // Everything ships in the bundle: no CDN, no remote fonts, no telemetry.
  build: { target: 'safari16' },
});
