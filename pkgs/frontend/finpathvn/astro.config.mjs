import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import viteCompression from 'vite-plugin-compression';
import obfuscator from 'vite-plugin-javascript-obfuscator';
import sentry from '@sentry/astro';

/**
 * Cấu hình Astro tối ưu cho hiệu suất và bảo mật
 * Phân tách rõ ràng luồng Client (Fetch/Dispatch) và SSG (Render)
 */
export default defineConfig({
  integrations: [
    svelte(),
    sentry({
      dsn: "https://91b76614979afa5046ac89fd9f7a0e10@o306117.ingest.us.sentry.io/4509647814262784",
      tracesSampleRate: 0.1,
    }),
  ],

  // Inlining CSS để giảm số lượng request cho trang SSG
  build: {
    inlineStylesheets: 'always',
  },

  vite: {
    css: {
      transformer: 'lightningcss',
      lightningcss: {
        targets: {
          safari: (13 << 16),
        },
      }
    },

    plugins: [
      tailwindcss(),
      viteCompression({
        algorithm: 'brotliCompress',
        ext: '.br',
        threshold: 1024,
      }),
      {
        ...obfuscator({
          options: {
            compact: true,
            controlFlowFlattening: false,
            deadCodeInjection: false,
            identifierNamesGenerator: 'mangled',
            stringArray: true,
            stringArrayThreshold: 0.75,
            unicodeEscapeSequence: false,
            exclude: [
              'node_modules/**/*',
              '**/@sentry/**'
            ],
          },
        }),
        apply: 'build',
      },
    ],

    build: {
      cssMinify: 'lightningcss',
      minify: 'terser',
      terserOptions: {
        compress: {
          drop_console: true,
          drop_debugger: true,
          pure_funcs: ['console.info', 'console.debug'],
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
          manualChunks(id) {
            if (id.includes('node_modules')) {
              return 'v';
            }

            if (id.includes('src/lib/api') || id.includes('dispatch.js')) {
              return 'api';
            }
          },

          // Tên file thu gọn để tối ưu metadata
          chunkFileNames: 'a/[hash].js',
          entryFileNames: 'a/[hash].js',
          assetFileNames: (assetInfo) => {
            if (assetInfo.name?.endsWith('.css')) {
              return 'c/[hash].css';
            }
            return 'assets/[hash][extname]';
          },
        },
      },
    },
  },
});
