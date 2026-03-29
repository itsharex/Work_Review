<script>
  export let bubble = null;

  $: bubbleMessage = bubble?.message ? bubble.message.replace(/\s+/g, '').trim() : '';
  $: toneClasses =
    bubble?.tone === 'success'
      ? 'border-emerald-400/28 bg-[linear-gradient(180deg,rgba(6,78,59,0.97),rgba(6,95,70,0.94))] text-emerald-50 ring-1 ring-emerald-200/8'
      : 'border-slate-300/20 bg-[linear-gradient(180deg,rgba(15,23,42,0.97),rgba(30,41,59,0.93))] text-slate-100 ring-1 ring-white/8';
</script>

{#if bubble}
  <div class="pointer-events-none absolute inset-0 z-20 overflow-visible">
    <div class="absolute" style="right: 18%; top: 4%;">
      <div class="relative overflow-visible">
      <div
        class="relative rounded-[22px] border backdrop-blur-xl shadow-[0_10px_18px_rgba(15,23,42,0.18),0_24px_44px_rgba(15,23,42,0.26)] {toneClasses}"
        style="width: fit-content; max-width: min(58vw, 188px); min-width: 0; padding: 10px 12px;"
      >
        <div class="pointer-events-none absolute inset-[1px] rounded-[21px] border border-white/10"></div>
        <div
          class="relative text-sm font-semibold leading-[1.35] tracking-[0.01em]"
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
