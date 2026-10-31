<script setup lang="ts">
// 调式标签:把一条配置的身份压成一个字(宫/商/角/徵/羽)。
// 字本身是标签,颜色只是辅助,色盲用户也能靠字形区分。
import { computed } from "vue";
import { toneIndex, toneOf } from "@/tones";

const props = withDefaults(
  defineProps<{ id?: string; label?: string; filled?: boolean; mono?: boolean }>(),
  { filled: false, mono: false },
);

const tone = computed(() => toneOf(props.id));
const index = computed(() => (props.id ? toneIndex(props.id) : 0));
</script>

<template>
  <span class="inline-flex items-center gap-1.5">
    <span
      class="grid size-[18px] shrink-0 place-items-center rounded-[4px] text-[11px] leading-none font-semibold transition-colors"
      :style="
        filled
          ? { background: tone.color, color: 'oklch(0.19 0.02 258)' }
          : {
              border: '1px solid color-mix(in oklab, ' + tone.color + ' 55%, transparent)',
              color: tone.ink,
            }
      "
      :title="`${tone.name}调`"
      :data-tone="index"
      >{{ tone.name }}</span
    >
    <span v-if="label" class="truncate" :class="mono && 'lt-num'">{{ label }}</span>
  </span>
</template>
