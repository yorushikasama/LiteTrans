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
  <div class="flex flex-wrap items-center gap-2.5">
    <!-- 模式切换 -->
    <div class="inline-flex rounded-lg border p-0.5 text-xs">
      <button
        class="rounded-md px-3 py-1.5 transition-colors"
        :class="
          settings.transMode === 'auto'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="setMode('auto')"
      >
        智能(自动→中文)
      </button>
      <button
        class="rounded-md px-3 py-1.5 transition-colors"
        :class="
          settings.transMode === 'custom'
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'
        "
        @click="setMode('custom')"
      >
        自定义方向
      </button>
    </div>

    <!-- 自定义方向:源 → 目标 -->
    <div v-if="settings.transMode === 'custom'" class="flex items-center gap-2">
      <LanguageSelect
        v-model="settings.sourceLang"
        with-auto
        @update:model-value="emit('change')"
      />
      <ArrowRight class="size-4 text-muted-foreground" />
      <LanguageSelect v-model="settings.targetLang" @update:model-value="emit('change')" />
    </div>
    <span v-else class="text-xs text-muted-foreground">自动识别源语言,译文为中文</span>
  </div>
</template>
