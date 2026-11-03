<script setup lang="ts">
/** 通用设置区:热键、开关、默认方向、附加提示词。 */
import type { Settings } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import TranslateDirection from "@/components/TranslateDirection.vue";

defineProps<{ settings: Settings; saveErr: string }>();

const emit = defineEmits<{
  (e: "save"): void;
  /** 单个开关改动后立即静默保存 */
  (e: "autosave"): void;
}>();
</script>

<template>
  <section class="mt-8 border-t pt-4">
    <h2 class="text-[13px] font-medium">通用设置</h2>

    <div class="mt-1 divide-y divide-border/60">
      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">划词热键</div>
          <div class="text-[11.5px] text-muted-foreground">选中文字后按下,自动复制并翻译</div>
        </div>
        <Input
          v-model="settings.hotkey"
          class="lt-num h-8 w-40 text-center text-xs"
          aria-label="划词热键"
        />
      </div>

      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">截图翻译热键</div>
          <div class="text-[11.5px] text-muted-foreground">框选屏幕区域,OCR 识别后翻译</div>
        </div>
        <Input
          v-model="settings.captureHotkey"
          class="lt-num h-8 w-40 text-center text-xs"
          aria-label="截图翻译热键"
        />
      </div>

      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">启用划词热键</div>
          <div class="text-[11.5px] text-muted-foreground">也可在托盘菜单里临时停用</div>
        </div>
        <Switch
          v-model="settings.hotkeyEnabled"
          class="h-6 w-11"
          aria-label="启用划词热键"
          @update:model-value="emit('autosave')"
        />
      </div>

      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">输入即译</div>
          <div class="text-[11.5px] text-muted-foreground">停止输入 0.8 秒后自动翻译</div>
        </div>
        <Switch
          v-model="settings.autoTranslate"
          class="h-6 w-11"
          aria-label="输入即译"
          @update:model-value="emit('autosave')"
        />
      </div>

      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">弹卡自动复制译文</div>
          <div class="text-[11.5px] text-muted-foreground">划词翻译完成后直接进剪贴板</div>
        </div>
        <Switch
          v-model="settings.autoCopy"
          class="h-6 w-11"
          aria-label="弹卡自动复制译文"
          @update:model-value="emit('autosave')"
        />
      </div>

      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="text-[13px]">开机自启</div>
          <div class="text-[11.5px] text-muted-foreground">启动后常驻托盘,不打开主窗口</div>
        </div>
        <Switch
          v-model="settings.autostart"
          class="h-6 w-11"
          aria-label="开机自启"
          @update:model-value="emit('autosave')"
        />
      </div>

      <div class="flex flex-col gap-2 py-3">
        <div>
          <div class="text-[13px]">默认翻译方向</div>
          <div class="text-[11.5px] text-muted-foreground">划词弹卡与主窗口共用同一方向</div>
        </div>
        <TranslateDirection :settings="settings" @change="emit('autosave')" />
      </div>

      <div class="flex flex-col gap-1.5 py-3">
        <div>
          <div class="text-[13px]">附加翻译要求</div>
          <div class="text-[11.5px] text-muted-foreground">写进系统提示词,可留空</div>
        </div>
        <Textarea
          v-model="settings.customPrompt"
          :rows="2"
          class="max-w-xl resize-none"
          placeholder="例如:术语按 XX 译法;语气保持正式"
        />
      </div>
    </div>

    <div class="mt-4 flex items-center gap-3">
      <Button size="sm" @click="emit('save')">保存设置</Button>
      <span v-if="saveErr" class="text-xs text-destructive">{{ saveErr }}</span>
    </div>
  </section>
</template>
