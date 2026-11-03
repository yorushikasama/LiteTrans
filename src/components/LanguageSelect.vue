<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { computed, ref } from "vue";
import { Check, ChevronsUpDown } from "@lucide/vue";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { LANGUAGES } from "@/types";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    withAuto?: boolean;
    /** 触发器宽度类:自定义方向行里要能收缩,不能写死 240px */
    class?: string;
  }>(),
  { placeholder: "", withAuto: false, class: "w-60" },
);
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();

const open = ref(false);
// LANGUAGES 里本来就没有 "auto"(自动检测是上面单独插的),不必再过滤一遍
const items = [
  ...(props.withAuto ? [{ value: "auto", label: "自动检测" }] : []),
  ...LANGUAGES.map((l) => ({ value: l.value, label: l.label })),
];
const display = computed(
  () =>
    items.find((i) => i.value === props.modelValue)?.label ||
    props.modelValue ||
    props.placeholder ||
    "自动",
);

function pick(v: string) {
  emit("update:modelValue", v);
  open.value = false;
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        variant="outline"
        role="combobox"
        :aria-expanded="open"
        :class="cn('justify-between gap-1 px-2 font-normal has-[>svg]:px-2', props.class)"
      >
        <span class="truncate">{{ display }}</span>
        <ChevronsUpDown class="size-3.5 shrink-0 opacity-50" />
      </Button>
    </PopoverTrigger>
    <PopoverContent class="w-64 p-0" align="start">
      <Command>
        <CommandInput placeholder="搜索语言…" />
        <CommandEmpty>无匹配语言</CommandEmpty>
        <CommandList>
          <CommandGroup>
            <CommandItem
              v-for="i in items"
              :key="i.value"
              :value="`${i.label} ${i.value}`"
              @select="pick(i.value)"
            >
              <Check
                :class="cn('mr-1 size-4', i.value === modelValue ? 'opacity-100' : 'opacity-0')"
              />
              {{ i.label }}
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </PopoverContent>
  </Popover>
</template>
