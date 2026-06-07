
<script>
  import { onMount } from 'svelte';
  import * as Sentry from '@sentry/astro';

  export let currentRegion = "";

  let isOpen = false;
  let content = "";
  let loading = false;
  let statusMessage = "";
  let statusType = "";

  async function handleSubmit() {
    if (!content.trim()) return;

    loading = true;
    statusMessage = "";

    try {
      Sentry.captureFeedback({
        message: content,
        url: window.location.href,
        tags: {
          region: currentRegion,
          feedback_type: 'user_feedback',
          page: window.location.pathname
        },
      });

      statusType = "success";
      statusMessage = "Cảm ơn bạn đã đóng góp ý kiến! 🙏";

      // Reset form
      content = "";

      setTimeout(() => {
        isOpen = false;
        statusMessage = "";
      }, 2000);

    } catch (e) {
      console.error(e);
      statusType = "error";
      statusMessage = "Gửi thất bại. Vui lòng thử lại.";
    } finally {
      loading = false;
    }
  }
</script>

<div class="fixed bottom-6 right-6 z-50 font-sans">
  {#if !isOpen}
    <button
      on:click={() => isOpen = true}
      class="flex items-center gap-2 bg-slate-900 hover:bg-slate-800 text-white font-medium text-sm px-4 py-3 rounded-full shadow-xl transition-all duration-200 transform hover:scale-105 active:scale-95 border border-slate-700/50"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-amber-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
      </svg>
      <span>Góp ý</span>
    </button>
  {:else}
    <div class="w-80 bg-white border border-slate-200 shadow-2xl rounded-2xl overflow-hidden transition-all duration-300">
      <div class="px-4 py-3 bg-slate-900 text-white flex justify-between items-center">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-amber-500 animate-pulse"></div>
          <span class="text-xs font-bold uppercase tracking-wider text-slate-300">Feedback Hệ Thống</span>
        </div>
        <button on:click={() => isOpen = false} class="text-slate-400 hover:text-white transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="p-4 flex flex-col gap-3">
        <p class="text-xs text-slate-500 leading-relaxed">
          Phát hiện lỗi hiển thị, sai giá hoặc có ý tưởng cải tiến? Hãy để lại lời nhắn giúp admin nhé!
        </p>

        <textarea
          bind:value={content}
          disabled={loading}
          placeholder="Nhập nội dung feedback tại đây..."
          rows="4"
          class="w-full text-sm p-2.5 border border-slate-200 rounded-xl focus:outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500/20 resize-none transition-all disabled:opacity-50"
        ></textarea>

        {#if statusMessage}
          <div class="text-xs p-2 rounded-lg font-medium {statusType === 'success' ? 'bg-emerald-50 text-emerald-700 border border-emerald-100' : 'bg-rose-50 text-rose-700 border border-rose-100'}">
            {statusMessage}
          </div>
        {/if}

        <button
          on:click={handleSubmit}
          disabled={loading || !content.trim()}
          class="w-full bg-amber-500 hover:bg-amber-600 disabled:bg-slate-100 text-slate-900 disabled:text-slate-400 font-semibold text-sm py-2.5 rounded-xl transition-all shadow-sm active:scale-[0.98]"
        >
          {#if loading}
            <span class="inline-block animate-spin mr-1">⌛</span> Đang gửi...
          {:else}
            Gửi phản hồi
          {/if}
        </button>
      </div>
    </div>
  {/if}
</div>
