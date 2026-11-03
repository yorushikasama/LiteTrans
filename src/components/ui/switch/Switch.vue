<script setup lang="ts">
import type { SwitchRootEmits, SwitchRootProps } from "reka-ui";
import type { HTMLAttributes } from "vue";
import { reactiveOmit } from "@vueuse/core";
import { cn } from "cn";
import { SwitchRoot, SwitchThumb, useForwardPropsEmits } from "reka-ui";

const props = defineProps<SwitchRootProps & { class?: HTMLAttributes["class"] }>();

const emits = defineEmits<SwitchRootEmits>();

const delegatedProps = reactiveOmit(props, "class");

const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <SwitchRoot
    v-slot="slotProps"
    data-slot="switch"
    v-bind="forwarded"
    :class="
      cn(
        'peer relative data-[state=checked]:bg-primary data-[state=unchecked]:bg-input focus-visible:border-ring focus-visible:ring-ring/50 dark:data-[state=unchecked]:bg-input/80 inline-flex h-[1.15rem] w-8 shrink-0 items-center rounded-full border border-transparent shadow-xs transition-all outline-none focus-visible:ring-3 disabled:cursor-not-allowed disabled:opacity-50 [container-type:size]',
        props.class,
      )
    "
  >
    <SwitchThumb
      data-slot="switch-thumb"
      :class="
        cn(
          // 滑块与位移都必须跟着轨道走,不能写死 size-4 + 固定位移。
          // 关键是位移:translate 的百分比按**滑块自身宽度**解析,而滑块宽度
          // 由轨道高度决定 —— 轨道一旦不是滑块的两倍宽(w-11 时是 2.4 倍),
          // 开启态就会停在离右边缘 9px 的地方(实测 44x24 轨道 gapRight=9)。
          // 所以用容器查询单位直接按轨道内容盒算:左右各留 2px,滑块宽 =
          // 内容高 - 4px,于是位移正好是 内容宽 - 内容高。
          // 注意 cqw/cqh 解析的是容器的**内容盒**,不要再补内缩量(补了会溢 1px)。
          'bg-background dark:data-[state=unchecked]:bg-foreground dark:data-[state=checked]:bg-primary-foreground pointer-events-none absolute left-[2px] top-1/2 block aspect-square h-[calc(100cqh-4px)] -translate-y-1/2 rounded-full ring-0 transition-transform data-[state=checked]:translate-x-[calc(100cqw-100cqh)] data-[state=unchecked]:translate-x-0',
        )
      "
    >
      <slot name="thumb" v-bind="slotProps" />
    </SwitchThumb>
  </SwitchRoot>
</template>
