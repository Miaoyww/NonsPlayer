import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import pkg from "./package.json" with { type: "json" };
import svg from "@poppanator/sveltekit-svg";

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  plugins: [
    tailwindcss(),
    sveltekit(),
    svg({
      includePaths: ["./src/lib/assets/"],
      svgoOptions: {
        multipass: true,
        plugins: [
          {
            name: "preset-default",
          },
          { name: "removeAttrs", params: { attrs: "(fill|stroke)" } },
        ],
      },
    }),
  ].filter(Boolean),
});
