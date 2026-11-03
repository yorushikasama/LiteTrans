<script setup lang="ts">
/** 历史页。只存本地,最多 50 条;点一条把原文带回翻译页。 */
import { History } from "@lucide/vue";
import type { HistoryItem } from "@/types";
import { Button } from "@/components/ui/button";
import ToneChip from "@/components/ToneChip.vue";

defineProps<{ items: HistoryItem[] }>();

const emit = defineEmits<{
  (e: "clear"): void;
  /** 选中一条:宿主需要把原文填进翻译页并切过去 */
  (e: "pick", source: string): void;
}>();

/** 当天的只显示时刻,更早的带上月/日 */
function fmtTime(ts: number): string {
  const d = new Date(ts * 1000);
  const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  return d.toDateString() === new Date().toDateString()
    ? hm
    : `${String(d.getMonth() + 1).padStart(2, "0")}/${String(d.getDate()).padStart(2, "0")} ${hm}`;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex h-14 shrink-0 items-center gap-3 px-5">
      <h1 class="text-[15px] font-medium">历史记录</h1>
      <span class="lt-num text-[11px] text-muted-foreground">
        {{ items.length }} / 50 · 仅存本地
      </span>
      <span class="flex-1"></span>
      <Button
        v-if="items.length"
        variant="ghost"
        size="sm"
        class="h-7 text-muted-foreground"
        @click="emit('clear')"
        >清空</Button
      >
    </div>

    <div
      v-if="!items.length"
      class="flex flex-1 flex-col items-center justify-center gap-2 border-t text-center"
    >
      <History class="size-6 text-shang-ink/40" />
      <span class="text-[13px] text-muted-foreground">还没有翻译记录</span>
      <span class="text-[11px] text-muted-foreground/70">
        翻译过的内容会自动存成历史,点击可重新编辑
      </span>
    </div>

    <div v-else class="min-h-0 flex-1 overflow-y-auto border-t">
      <button
        v-for="(h, i) in items"
        :key="`${h.ts}-${i}`"
        class="flex w-full items-start gap-3 border-b border-border/50 px-5 py-2.5 text-left transition-colors hover:bg-accent/50"
        @click="emit('pick', h.source)"
      >
        <span class="lt-num w-12 shrink-0 pt-0.5 text-[11px] text-muted-foreground">
          {{ fmtTime(h.ts) }}
        </span>
        <span class="flex min-w-0 flex-1 flex-col gap-0.5">
          <span class="flex items-center gap-2">
            <ToneChip v-if="h.profileId" :id="h.profileId" />
            <span class="lt-num truncate text-[11px] text-muted-foreground">
              {{ h.direction }}<template v-if="h.dict"> · 词典</template>
              <template v-if="h.profile"> · {{ h.profile }}</template>
            </span>
          </span>
          <span class="line-clamp-1 text-[11.5px] text-muted-foreground/80">
            {{ h.source }}
          </span>
          <span class="line-clamp-2 text-[13px] leading-5">{{ h.result }}</span>
        </span>
      </button>
    </div>
  </div>
</template>
