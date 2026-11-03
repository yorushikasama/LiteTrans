<script setup lang="ts">
/** 设置页:把三块 section 组装起来,只负责滚动容器与标题。 */
import type { Preset, RuntimeInfo, Settings } from "@/types";
import type { useSettings } from "@/composables/useSettings";
import AboutSection from "@/components/settings/AboutSection.vue";
import GeneralSection from "@/components/settings/GeneralSection.vue";
import ProfileSection from "@/components/settings/ProfileSection.vue";

const props = defineProps<{
  settings: Settings;
  presets: Preset[];
  runtime: RuntimeInfo | null;
  /** useSettings() 的返回值,宿主持有以跨页面保留编辑态 */
  s: ReturnType<typeof useSettings>;
}>();

const emit = defineEmits<{
  (e: "save"): void;
  (e: "autosave"): void;
  (e: "open-data-dir"): void;
}>();

const { saveErr } = props.s;
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="mx-auto w-full max-w-2xl px-5 pb-10">
        <div class="flex h-14 items-center">
          <h1 class="text-[15px] font-medium">设置</h1>
        </div>

        <ProfileSection :settings="settings" :presets="presets" :s="s" />
        <GeneralSection
          :settings="settings"
          :save-err="saveErr"
          @save="emit('save')"
          @autosave="emit('autosave')"
        />
        <AboutSection v-if="runtime" :runtime="runtime" @open-data-dir="emit('open-data-dir')" />
      </div>
    </div>
  </div>
</template>
