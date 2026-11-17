import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: { target: "esnext", outDir: "dist" },
  lint: {
    ignorePatterns: ["node_modules/**", "dist/**", "data/**"],
    options: { typeAware: true, typeCheck: true },
  },
  fmt: { ignorePatterns: ["node_modules/**", "dist/**", "data/**"], printWidth: 110 },
});
