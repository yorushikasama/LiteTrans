import { fileURLToPath, URL } from "node:url";

import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// 主窗口与翻译弹卡两个入口
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "chrome105",
    rollupOptions: {
      input: {
        main: "index.html",
        popup: "popup.html",
      },
    },
  },
});
