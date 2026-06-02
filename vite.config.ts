import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import obfuscator from "vite-plugin-javascript-obfuscator";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [
    react(),
    tailwindcss(),
    obfuscator({
      include: ["src/**/*.ts", "src/**/*.tsx"],
      apply: "build",
      options: {
        compact: true,
        sourceMap: false,
        deadCodeInjection: false,
        controlFlowFlattening: false,
        stringArray: true,
        stringArrayRotate: true,
        splitStrings: true,
        splitStringsChunkLength: 8,
      },
    }),
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  optimizeDeps: {
    entries: ["index.html"],
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
