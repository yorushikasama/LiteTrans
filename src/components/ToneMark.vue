<script setup lang="ts">
// 五音立标:五根竖柱(宫商角徵羽),当前配置所属的「调」拔高并着色。
// 既是品牌标记,也是实时信息:一眼看出当前在用哪条配置。
import { computed } from "vue";
import { TONES, toneIndex } from "@/tones";

const props = withDefaults(defineProps<{ id?: string; size?: "sm" | "md" }>(), { size: "md" });

const active = computed(() => (props.id ? toneIndex(props.id) : -1));
const BASE = [7, 10, 13, 10, 7];
const scale = computed(() => (props.size === "sm" ? 0.8 : 1));
</script>

<template>
  <span
    class="inline-flex shrink-0 items-end"
    :style="{ gap: `${2.5 * scale}px`, height: `${18 * scale}px` }"
    :title="active >= 0 ? `当前调式:${TONES[active].name}调` : '五音'"
    aria-hidden="true"
  >
    <span
      v-for="(t, i) in TONES"
      :key="t.key"
      class="w-[3px] rounded-full transition-[height,background-color] duration-200 ease-out"
      :style="{
        height: `${(i === active ? BASE[i] + 5 : BASE[i]) * scale}px`,
        background:
          i === active ? t.color : 'color-mix(in oklab, var(--muted-foreground) 38%, transparent)',
      }"
    />
  </span>
</template>
