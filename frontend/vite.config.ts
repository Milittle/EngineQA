import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const backendPort = process.env.APP_PORT ?? '8080';
const backendTarget =
  process.env.VITE_BACKEND_PROXY_TARGET ?? `http://127.0.0.1:${backendPort}`;

export default defineConfig({
  plugins: [react()],
  server: {
    host: '0.0.0.0',
    port: Number(process.env.FRONTEND_PORT ?? 5173),
    proxy: {
      '/api': {
        target: backendTarget,
        changeOrigin: true,
      },
      '/health': {
        target: backendTarget,
        changeOrigin: true,
      },
    },
  },
});
