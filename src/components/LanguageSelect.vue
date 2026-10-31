<script setup lang="ts">
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
  defineProps<{ modelValue: string; placeholder?: string; withAuto?: boolean }>(),
  { placeholder: "", withAuto: false },
);
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();

const open = ref(false);
const items = [
  ...(props.withAuto ? [{ value: "auto", label: "自动检测" }] : []),
  ...LANGUAGES.filter((l) => l.value !== "auto").map((l) => ({ value: l.value, label: l.label })),
];
const display = computed(
  () => items.find((i) => i.value === props.modelValue)?.label ?? props.modelValue ?? props.placeholder ?? "自动",
);

function pick(v: string) {
  emit("update:modelValue", v);
  open.value = false;
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button variant="outline" role="combobox" :aria-expanded="open" class="w-60 justify-between font-normal">
        <span class="truncate">{{ display }}</span>
        <ChevronsUpDown class="size-4 shrink-0 opacity-50" />
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
              <Check :class="cn('mr-1 size-4', i.value === modelValue ? 'opacity-100' : 'opacity-0')" />
              {{ i.label }}
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </PopoverContent>
  </Popover>
</template>
