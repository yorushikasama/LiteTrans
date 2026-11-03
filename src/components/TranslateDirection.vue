<script setup lang="ts">
import { ArrowRight } from "@lucide/vue";
import type { Settings } from "@/types";
import LanguageSelect from "./LanguageSelect.vue";

const props = defineProps<{ settings: Settings }>();
const emit = defineEmits<{ (e: "change"): void }>();

function setMode(m: string) {
  if (props.settings.transMode === m) return;
  props.settings.transMode = m;
  emit("change");
}
</script>

<template>
  <!-- 宽屏不换行(折行会把工具行撑到 99px);窄屏允许换行,否则 640px
       最小宽度下两个语言下拉会横向溢出。 -->
  <div class="flex min-w-0 flex-wrap items-center gap-2.5 lg:flex-nowrap">
    <div class="inline-flex shrink-0 items-center gap-0.5 rounded-md border p-0.5">
      <button
        class="rounded-[4px] px-2.5 py-1 text-[12px] leading-5 transition-colors"
        :class="
          settings.transMode === 'auto'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="setMode('auto')"
      >
        智能
      </button>
      <button
        class="rounded-[4px] px-2.5 py-1 text-[12px] leading-5 transition-colors"
        :class="
          settings.transMode === 'custom'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="setMode('custom')"
      >
        自定义
      </button>
    </div>

    <!-- 自定义方向:源 → 目标。宽度固定一个合理值并允许收缩:
         `w-full` 在 flex 行里会强制撑到父宽,配合 min-w-28 两个下拉就超出
         容器(实测 900px 窗口下 499 > 371 横向溢出)。
         这一行每加宽一档都会把工具行挤折(折行后工具行 52px→102px,
         正文区直接少 50px;150% 缩放下正文本来只有 245px)。
         实测 533px 宽时可用区 353px:智能/自定义分段 108 + 间距 10 +
         两枚下拉 2x96 + 箭头 14 ≈ 324,才放得下。 -->
    <div v-if="settings.transMode === 'custom'" class="flex min-w-0 items-center gap-2">
      <LanguageSelect
        v-model="settings.sourceLang"
        with-auto
        class="w-24 min-w-0 md:w-32 lg:w-36"
        @update:model-value="emit('change')"
      />
      <ArrowRight class="size-3.5 shrink-0 text-muted-foreground" />
      <LanguageSelect
        v-model="settings.targetLang"
        class="w-24 min-w-0 md:w-32 lg:w-36"
        @update:model-value="emit('change')"
      />
    </div>
    <span v-else class="truncate text-[12px] text-muted-foreground">自动识别源语言 → 中文</span>
  </div>
</template>
