<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, ChevronsUpDown, Plus } from "@lucide/vue";
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

const props = defineProps<{
  modelValue: string;
  options: string[];
  placeholder?: string;
}>();
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();

const open = ref(false);
const search = ref("");

const canCreate = computed(() => {
  const s = search.value.trim();
  return !!s && !props.options.some((o) => o.toLowerCase() === s.toLowerCase());
});

function pick(v: string) {
  emit("update:modelValue", v.trim());
  open.value = false;
  search.value = "";
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        variant="outline"
        role="combobox"
        :aria-expanded="open"
        class="w-full justify-between font-normal"
      >
        <span class="truncate" :class="{ 'text-muted-foreground': !modelValue }">
          {{ modelValue || placeholder || "选择模型" }}
        </span>
        <ChevronsUpDown class="size-4 shrink-0 opacity-50" />
      </Button>
    </PopoverTrigger>
    <PopoverContent class="w-[320px] p-0" align="start">
      <Command>
        <CommandInput v-model="search" placeholder="搜索或输入模型名…" />
        <CommandEmpty v-if="!canCreate">无匹配模型</CommandEmpty>
        <CommandList>
          <CommandGroup>
            <CommandItem v-for="m in options" :key="m" :value="m" @select="pick(m)">
              <Check class="mr-1 size-4" :class="m === modelValue ? 'opacity-100' : 'opacity-0'" />
              {{ m }}
            </CommandItem>
            <CommandItem v-if="canCreate" :value="search" @select="pick(search)">
              <Plus class="mr-1 size-4" />
              使用「{{ search }}」
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </PopoverContent>
  </Popover>
</template>
