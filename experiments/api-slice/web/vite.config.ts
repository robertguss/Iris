import { defineConfig } from "vite";

export default defineConfig({
  server: {
    proxy: { "/api": process.env.IRIS_API_TARGET ?? "http://127.0.0.1:3002" },
  },
});
