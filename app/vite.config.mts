import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Builds into ../ui (or VAAL_UI, see build.ps1): vaal.html + assets/ + fonts/ + images/.
const outDir = process.env.VAAL_UI ?? path.resolve(__dirname, '../ui')

export default defineConfig({
  base: '/',
  build: {
    target: 'esnext',
    assetsInlineLimit: 0,
    outDir,
    emptyOutDir: true,
    assetsDir: 'assets',
    rollupOptions: { input: path.resolve(__dirname, 'vaal.html') }
  },
  plugins: [vue()]
})