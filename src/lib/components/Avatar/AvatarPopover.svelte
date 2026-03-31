<script>
  import { formatBubbleMessage } from './bubbleMessage.js';

  export let bubble = null;
  export let onClose = () => {};

  $: bubbleMessage = formatBubbleMessage(bubble?.message);
  $: toneClasses =
    bubble?.tone === 'success'
      ? 'border-emerald-400/28 bg-[linear-gradient(180deg,rgba(6,78,59,0.97),rgba(6,95,70,0.94))] text-emerald-50 ring-1 ring-emerald-200/8'
      : 'border-slate-300/20 bg-[linear-gradient(180deg,rgba(15,23,42,0.97),rgba(30,41,59,0.93))] text-slate-100 ring-1 ring-white/8';
</script>

{#if bubble}
  <div class="absolute inset-0 z-20 overflow-visible pointer-events-none">
    <div class="absolute" style="right: 18%; top: 4%;">
      <div class="relative overflow-visible">
        <div
          class="pointer-events-auto relative rounded-[22px] border backdrop-blur-xl shadow-[0_10px_18px_rgba(15,23,42,0.18),0_24px_44px_rgba(15,23,42,0.26)] {toneClasses}"
          style="width: fit-content; max-width: min(58vw, 188px); min-width: 0; padding: 10px 12px;"
        >
          {#if bubble?.persistent}
            <button
              type="button"
              class="absolute inset-0 rounded-[22px]"
              aria-label="关闭提醒"
              on:click={onClose}
            ></button>
          {/if}
          <div class="pointer-events-none absolute inset-[1px] rounded-[21px] border border-white/10"></div>
          {#if bubble?.persistent}
            <button
              type="button"
              class="absolute right-2 top-2 z-10 inline-flex h-5 w-5 items-center justify-center rounded-full text-slate-300 transition hover:bg-white/10 hover:text-white"
              aria-label="关闭提醒"
              on:click={onClose}
            >
              ×
            </button>
          {/if}
          <div
            class="pointer-events-none relative text-sm font-semibold leading-[1.35] tracking-[0.01em] pr-6"
            style="display: inline-block; word-break: break-word;"
          >
            {bubbleMessage}
          </div>
        </div>
        <div class="absolute left-[18px] top-[calc(100%-4px)] h-[7px] w-[7px] rounded-full bg-slate-200/45 shadow-[0_2px_6px_rgba(15,23,42,0.14)]"></div>
        <div class="absolute left-[6px] top-[calc(100%+8px)] h-[11px] w-[11px] rounded-full bg-slate-100/62 shadow-[0_4px_10px_rgba(15,23,42,0.18)]"></div>
      </div>
    </div>
  </div>
{/if}
