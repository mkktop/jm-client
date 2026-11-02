import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: '127.0.0.1',
  },
  build: {
    target: 'esnext',
    sourcemap: false,
  },
})
