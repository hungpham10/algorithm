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
      // 1. Thêm Gzip song song với Brotli (Brotli tốt cho trình duyệt mới, Gzip làm fallback tránh lãng phí byte)
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
      // 2. Chuyển Obfuscator xuống chạy SAU cùng để không làm hỏng logic chunking
      {
        ...obfuscator({
          options: {
            compact: true,
            controlFlowFlattening: false,
            deadCodeInjection: false,
            identifierNamesGenerator: 'mangled',
            stringArray: true,
            stringArrayThreshold: 0.8, // Tăng nhẹ để gộp chuỗi tốt hơn
            stringArrayEncoding: ['base64'], // Mã hóa base64 để nén Gzip/Brotli bắt được pattern trùng tốt hơn
            unicodeEscapeSequence: false,
            exclude: [
              'node_modules/**/*',
              '**/@sentry/**',
              '**/*.css'
            ],
          },
        }),
        apply: 'build',
        enforce: 'post', // ÉP BUỘC: Chạy sau khi Vite đã tối ưu xong xuôi
      },
    ],

    build: {
      cssMinify: 'lightningcss',
      minify: 'terser',
      terserOptions: {
        compress: {
          drop_console: true,
          drop_debugger: true,
          pure_funcs: ['console.info', 'console.debug', 'console.warn'], // Xóa thêm cả console.warn
          passes: 3,
          unsafe: true, // BẬT: Cho phép Terser dùng các hàm tối ưu sâu (khai thác tính chất ES6+)
          unsafe_arrows: true, // Chuyển function() thường thành arrow function () => để giảm byte
          unsafe_comps: true,
          unsafe_math: true, // Tối ưu các biểu thức toán học tĩnh
        },
        mangle: {
          toplevel: true,
          properties: false, // TUYỆT ĐỐI ĐỂ FALSE: Mangle property sẽ làm hỏng Svelte 5 / Sentry
        },
        format: {
          comments: false,
        },
      },

      assetsInlineLimit: 4096,
      rollupOptions: {
        output: {
          // 3. Tối ưu lại phân mảnh ( manualChunks )
          manualChunks(id) {
            // Tách Sentry ra chunk riêng, vì Obfuscate đè vào Sentry sẽ lỗi crash log telemetry
            if (id.includes('node_modules/@sentry')) {
              return 'sentry';
            }
            // Gom tất cả node_modules còn lại vào 'v' như cũ
            if (id.includes('node_modules')) {
              return 'v';
            }
            // Code core API/Dispatch của bạn
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
