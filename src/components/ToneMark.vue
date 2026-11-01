<script setup lang="ts">
// 五音立标:五根竖弦,按宫商角徵羽着色,当前配置所属的「调」拔高发光。
// 既是品牌标记,也是实时读数:一眼看出当前在用哪条配置。
import { computed } from "vue";
import { TONES, toneIndex } from "@/tones";

const props = withDefaults(defineProps<{ id?: string; size?: "sm" | "md" | "lg" }>(), {
  size: "md",
});

const active = computed(() => (props.id ? toneIndex(props.id) : -1));
/** 弦长:中间最高,两头最短,像一张琴的弦列 */
const BASE = [9, 12, 15, 12, 9];
const scale = computed(() => ({ sm: 0.72, md: 1, lg: 1.35 })[props.size]);
const barW = computed(() => Math.max(2, 3 * scale.value));
</script>

<template>
  <span
    class="inline-flex shrink-0 items-center"
    :style="{ gap: `${3 * scale}px`, height: `${18 * scale}px` }"
    :title="active >= 0 ? `当前调式:${TONES[active].name}调` : '宫商角徵羽'"
    aria-hidden="true"
  >
    <span
      v-for="(t, i) in TONES"
      :key="t.key"
      class="rounded-full transition-all duration-300 ease-out"
      :style="{
        width: `${barW}px`,
        height: `${(i === active ? BASE[i] + 3 : BASE[i]) * scale}px`,
        background: t.color,
        opacity: i === active ? 1 : 0.42,
        boxShadow: i === active ? `0 0 ${6 * scale.value}px ${t.color}` : 'none',
      }"
    />
  </span>
</template>
