<script setup lang="ts">
/** 侧栏:品牌 + 主导航 + 底部当前调式。 */
import { History, Languages, Settings2 } from "@lucide/vue";
import type { Profile } from "@/types";
import ToneChip from "@/components/ToneChip.vue";
import ToneMark from "@/components/ToneMark.vue";
import { TabsList, TabsTrigger } from "@/components/ui/tabs";

defineProps<{ activeProfile: Profile | null; toneLabel: string }>();

const emit = defineEmits<{ (e: "open-settings"): void }>();

const NAV = [
  { value: "translate", label: "翻译", icon: Languages },
  { value: "history", label: "历史", icon: History },
  { value: "settings", label: "设置", icon: Settings2 },
] as const;
</script>

<template>
  <!-- 150% 缩放下 800x600 的窗口只有 533px 宽,188px 的侧栏会吃掉 35%,
       内容区只剩 345px。窄屏收窄到 148px 并隐藏品牌字,把宽度还给正文。 -->
  <aside class="flex w-[148px] shrink-0 flex-col border-r bg-card/60 select-none sm:w-[188px]">
    <!-- 品牌 -->
    <div class="flex h-14 items-center gap-2.5 px-4">
      <ToneMark :id="activeProfile?.id" class="shrink-0" />
      <span class="lt-wordmark hidden truncate sm:inline">LiteTrans</span>
    </div>

    <!-- 导航 -->
    <!-- TabsList 自带 w-fit + inline-flex,在纵向布局里会收缩到内容宽度;
         TabsTrigger 自带 flex-1(flex-basis:0%),在 flex-col 里会覆盖 h-9。
         两个都要显式顶掉,否则实测得到 72x27.5 的窄条,而不是整栏 36px 高。 -->
    <TabsList
      class="mx-2 !flex !h-auto !w-auto flex-col gap-0.5 bg-transparent p-0"
      aria-label="主导航"
    >
      <TabsTrigger
        v-for="n in NAV"
        :key="n.value"
        :value="n.value"
        class="group !h-9 !w-full items-center !flex-none justify-start gap-2.5 rounded-md border-0 px-2.5 text-[13px] font-normal text-muted-foreground data-[state=active]:bg-secondary data-[state=active]:font-medium data-[state=active]:text-foreground data-[state=active]:shadow-none"
      >
        <component :is="n.icon" class="size-4 shrink-0" />
        {{ n.label }}
      </TabsTrigger>
    </TabsList>

    <!-- 当前调式 -->
    <div class="mt-auto px-2 pb-3">
      <button
        v-if="activeProfile"
        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-accent"
        :title="`当前配置:${activeProfile.name} · ${toneLabel}`"
        @click="emit('open-settings')"
      >
        <ToneChip :id="activeProfile.id" />
        <span class="min-w-0 flex-1">
          <span class="block truncate text-[12px] leading-4">{{ activeProfile.name }}</span>
          <span class="lt-num block truncate text-[10px] leading-4 text-muted-foreground">
            {{ activeProfile.model || "未指定模型" }}
          </span>
        </span>
      </button>
      <button
        v-else
        class="w-full rounded-md px-2 py-1.5 text-left text-[12px] text-muted-foreground hover:bg-accent"
        @click="emit('open-settings')"
      >
        尚未配置模型
      </button>
    </div>
  </aside>
</template>
