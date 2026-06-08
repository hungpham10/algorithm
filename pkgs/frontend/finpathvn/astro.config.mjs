import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import viteCompression from 'vite-plugin-compression';
import obfuscator from 'vite-plugin-javascript-obfuscator';
import sentry from '@sentry/astro';


export default defineConfig({
  integrations: [
    svelte(),
    sentry({
      dsn: "https://91b76614979afa5046ac89fd9f7a0e10@o306117.ingest.us.sentry.io/4509647814262784",
      tracesSampleRate: 0.1,
    }),
  ],

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
        algorithm: 'gzip',
        ext: '.gz',
        threshold: 1024,
      }),
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
            stringArrayThreshold: 0.8,
            stringArrayEncoding: ['base64'],
            unicodeEscapeSequence: false,
            exclude: [
              'node_modules/**/*',
              '**/@sentry/**/*',
              '**/*.css'
            ],
          },
        }),
        apply: 'build',
        enforce: 'post',
      },
    ],

    build: {
      cssMinify: 'lightningcss',
      minify: 'terser',
      terserOptions: {
        compress: {
          drop_console: true,
          drop_debugger: true,
          pure_funcs: ['console.info', 'console.debug', 'console.warn'],
          passes: 3,
          unsafe: true,
          unsafe_arrows: true,
          unsafe_comps: true,
          unsafe_math: true,
        },
        mangle: {
          toplevel: true,
          properties: false,
        },
        format: {
          comments: false,
        },
      },

      assetsInlineLimit: 4096,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes('node_modules/@sentry')) {
              return 'sentry';
            }
            if (id.includes('node_modules')) {
              return 'v';
            }
            if (id.includes('src/lib/api') || id.includes('dispatch.js')) {
              return 'api';
            }
          },

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
