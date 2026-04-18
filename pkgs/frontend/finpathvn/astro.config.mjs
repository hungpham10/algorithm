// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import viteCompression from 'vite-plugin-compression';

// https://astro.build/config
export default defineConfig({
  integrations: [svelte()],

  vite: {
    plugins: [
      tailwindcss(),
      viteCompression({
        algorithm: 'gzip', // hoặc 'brotliCompress' nếu server bạn hỗ trợ
        ext: '.gz',
      })
    ],
    build: {
      // 1. Asset nhỏ hơn 5kb sẽ được inline thẳng vào HTML (giảm số lượng request file lẻ)
      assetsInlineLimit: 5120,
      rollupOptions: {
        output: {
          // 2. Gom các thư viện node_modules vào 1 file vendor duy nhất
          // Giúp trình duyệt tải 1 file lớn nhanh hơn là nhiều file 1-2kb
          manualChunks(id) {
            if (id.includes('node_modules')) {
              return 'vendor';
            }
            // Gom tất cả các component Svelte vào một chunk để phá vỡ móc xích
            if (id.includes('src/components/')) {
              return 'main-ui';
            }
          },
          // 3. Đặt tên file gọn gàng
          chunkFileNames: 'assets/js/[name]-[hash].js',
          entryFileNames: 'assets/js/[name]-[hash].js',
          assetFileNames: 'assets/[ext]/[name]-[hash].[ext]'
        }
      }
    }
  }
});
