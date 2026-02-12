import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

export default defineConfig({
  plugins: [
    react(),
    nodePolyfills({
      include: ['buffer', 'process'],
      globals: {
        Buffer: true,
        process: true,
      },
    }),
  ],
  optimizeDeps: {
    // exclude local hyperstack packages from pre-bundling
    exclude: ['hyperstack-typescript', 'hyperstack-react', 'hyperstack-stacks'],
  },
})
