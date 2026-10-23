import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  // GitHub Pages serves this repo at <user>.github.io/odbviewer/, not the domain root.
  base: '/odbviewer/',
})
