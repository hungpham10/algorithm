import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import viteCompression from 'vite-plugin-compression';
import obfuscator from 'vite-plugin-javascript-obfuscator';

export default defineConfig({
  integrations: [svelte()],

  // Tự động nhúng CSS vào HTML nếu file nhỏ (giảm số lượng request)
  build: {
    inlineStylesheets: 'always',
  },

  vite: {
    // 1. Cấu hình Lightning CSS để tối ưu dung lượng style
    css: {
      transformer: 'lightningcss',
      lightningcss: {
        targets: {
          safari: (13 << 16), // Hỗ trợ tương đối rộng để tối ưu syntax
        },
      }
    },

    plugins: [
      tailwindcss(),
      viteCompression({
        algorithm: 'brotliCompress', // Brotli nén CSS/JS tốt hơn Gzip rất nhiều
        ext: '.br',
      }),
      {
        ...obfuscator({
          options: {
            compact: true,
            log: false, // Tắt cái quảng cáo "Obfuscator Pro" phiền phức
            controlFlowFlattening: false, // Để false để file không bị phình to
            deadCodeInjection: false,
            identifierNamesGenerator: 'mangled', // Đổi tên biến a, b, c
            stringArray: true,
            stringArrayThreshold: 0.75,
          },
        }),
        apply: 'build',
      },
    ],

    build: {
      // 2. Nén CSS bằng Lightning CSS
      cssMinify: 'lightningcss',

      // 3. Tối ưu JS sâu với Terser
      minify: 'terser',
      terserOptions: {
        compress: {
          drop_console: true,
          drop_debugger: true,
          pure_funcs: ['console.info', 'console.debug', 'console.warn'],
          passes: 3,
        },
        mangle: {
          toplevel: true,
        },
        format: {
          comments: false,
        },
      },

      assetsInlineLimit: 4096,
      rollupOptions: {
        output: {
          // Tối ưu tên file và chunk để giảm metadata
          manualChunks(id) {
            if (id.includes('node_modules')) {
              return 'v'; // vendor -> v
            }
          },
          chunkFileNames: 'a/[hash].js',
          entryFileNames: 'a/[hash].js',
          assetFileNames: (assetInfo) => {
            if (assetInfo.name?.endsWith('.css')) {
              return 'c/[hash].css'; // css -> c
            }
            return 'assets/[hash][extname]';
          },
        },
      },
    },
  },
});
