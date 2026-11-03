<script setup lang="ts">
/**
 * 翻译页。布局是四条横带:工具行 / 原文 / 方向读数 / 译文,底部操作条。
 *
 * 这里只负责渲染与转发事件,状态和请求都在 useTranslate 里 ——
 * 该页面要处理流式输出、剪贴板、朗读、防抖,逻辑比模板长得多。
 */
import {
  Check,
  ClipboardPaste,
  Copy,
  Eraser,
  Languages,
  Loader2,
  Square,
  Volume2,
} from "@lucide/vue";
import type { Profile, Settings } from "@/types";
import type { useTranslate } from "@/composables/useTranslate";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import ToneMark from "@/components/ToneMark.vue";
import TranslateDirection from "@/components/TranslateDirection.vue";

const props = defineProps<{
  settings: Settings;
  activeProfile: Profile | null;
  /** 当前档案是否已具备翻译条件;false 时顶部显示配置引导条 */
  activeReady: boolean;
  /** useTranslate() 的返回值,宿主持有以便跨页面保留状态 */
  t: ReturnType<typeof useTranslate>;
}>();

const emit = defineEmits<{
  (e: "settings-changed"): void;
  (e: "open-settings"): void;
}>();

const {
  source,
  result,
  errorMsg,
  streaming,
  speaking,
  copied,
  resultBox,
  sourceChars,
  directionReadout,
  translate,
  cancel,
  clearAll,
  copyResult,
  speak,
  pasteFromClipboard,
} = props.t;
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <!-- 未配置模型时的引导条 -->
    <div
      v-if="!activeReady"
      class="flex shrink-0 items-center gap-3 border-b border-gong/40 bg-gong/10 px-5 py-2"
    >
      <span class="text-xs">还没有可用的模型配置,添加服务商与 API Key 后即可翻译。</span>
      <span class="flex-1"></span>
      <Button size="xs" @click="emit('open-settings')">添加配置</Button>
    </div>

    <!-- 工具行:方向 + 热键提示。
         宽屏 nowrap 保持单行(折行会把版面撑高,实测曾达 99px);
         窄屏必须允许换行,否则内容横向溢出。 -->
    <div
      class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1.5 px-4 py-2 lg:flex-nowrap lg:px-5"
    >
      <TranslateDirection :settings="settings" @change="emit('settings-changed')" />
      <span class="flex-1"></span>
      <span class="lt-num hidden shrink-0 text-[11px] text-muted-foreground lg:inline">
        Ctrl+Enter 翻译
      </span>
    </div>

    <!-- 正文:原文 / 方向 / 译文 -->
    <div class="flex min-h-0 flex-1 flex-col border-t">
      <div class="min-h-0 flex-[0.85]">
        <Textarea
          v-model="source"
          class="h-full resize-none rounded-none border-0 bg-transparent px-5 py-3 text-[14.5px] leading-7 shadow-none field-sizing-fixed focus-visible:ring-0"
          placeholder="粘贴或输入要翻译的文本,Ctrl+Enter 翻译"
          @keydown.ctrl.enter.prevent="translate"
        />
      </div>

      <!-- 方向读数条 -->
      <div class="flex shrink-0 items-center gap-3 border-y bg-muted/40 px-5 py-1.5">
        <span class="lt-num flex items-center gap-1.5 text-[11px] text-muted-foreground">
          <span
            class="size-1.5 rounded-full transition-colors"
            :class="streaming ? 'animate-pulse bg-gong' : 'bg-shang-ink/50'"
          ></span>
          {{ directionReadout }}
        </span>
        <span class="flex-1"></span>
        <span v-if="streaming" class="lt-num text-[10px] text-gong-ink">生成中</span>
        <span v-else class="lt-num text-[10px] text-muted-foreground/70">
          {{ sourceChars }} 字
        </span>
      </div>

      <div ref="resultBox" class="flex min-h-0 flex-[1.15] flex-col overflow-y-auto px-5 py-3">
        <div v-if="errorMsg" class="text-sm leading-6 text-destructive">{{ errorMsg }}</div>
        <!-- 空态即操作说明,外层是 overflow-y-auto 的滚动容器。
             这是整个应用最挤的一屏:150% 缩放下默认窗口 800x600 只有
             533x400 个 CSS 像素,扣掉工具行/读数条/操作条后正文区
             仅约 133px。所以这份说明必须按"密"来排 ——
             窄屏隐藏竖标与导语、行距收紧;宽屏(sm 以上)才恢复完整文案。
             居中用 justify-safe-center:放得下居中,放不下退回顶端,
             否则 justify-center 会把溢出均分到上下两端、顶端滚不到。 -->
        <div
          v-else-if="!result"
          class="flex min-h-full flex-col items-center justify-safe-center gap-2 py-1 text-center sm:gap-3 sm:py-2"
        >
          <ToneMark :id="activeProfile?.id" size="lg" class="hidden shrink-0 sm:block" />
          <span class="text-[12px] text-muted-foreground sm:text-[13px]">三种用法</span>
          <div class="flex w-full max-w-md flex-col gap-1.5 text-left sm:gap-2">
            <div class="flex items-center gap-2 sm:items-start sm:gap-2.5">
              <kbd
                class="lt-num shrink-0 rounded border bg-card px-1.5 py-0.5 text-[11px] sm:mt-px"
              >
                {{ settings.hotkey || "Alt+Q" }}
              </kbd>
              <span class="min-w-0 text-[12px] leading-5 text-muted-foreground sm:flex-1">
                在<b class="font-medium text-foreground">别的窗口</b>选中文字后按热键
              </span>
            </div>
            <div class="flex items-center gap-2 sm:items-start sm:gap-2.5">
              <kbd
                class="lt-num shrink-0 rounded border bg-card px-1.5 py-0.5 text-[11px] sm:mt-px"
              >
                {{ settings.captureHotkey || "Alt+X" }}
              </kbd>
              <span class="min-w-0 text-[12px] leading-5 text-muted-foreground sm:flex-1">
                截图翻译,框选图片、PDF、视频字幕里的文字
              </span>
            </div>
            <div class="flex items-center gap-2 sm:items-start sm:gap-2.5">
              <kbd
                class="lt-num shrink-0 rounded border bg-card px-1.5 py-0.5 text-[11px] sm:mt-px"
              >
                Ctrl+Enter
              </kbd>
              <span class="min-w-0 text-[12px] leading-5 text-muted-foreground sm:flex-1">
                在下面贴文字直接翻译
              </span>
            </div>
          </div>
          <span class="hidden text-[11px] leading-4 text-muted-foreground/70 sm:block">
            划词依赖模拟 Ctrl+C,某些终端或游戏里不生效;这些场景请用截图翻译
          </span>
        </div>
        <div v-else class="lt-fade text-[15px] leading-[1.85] whitespace-pre-wrap">
          {{ result }}<span v-if="streaming" class="lt-cursor"></span>
        </div>
      </div>
    </div>

    <!-- 操作条 -->
    <div class="flex shrink-0 items-center gap-1.5 border-t bg-card/50 px-5 py-2">
      <Button :disabled="!source.trim() || streaming" size="sm" @click="translate">
        <Loader2 v-if="streaming" class="animate-spin" />
        <Languages v-else />
        翻译
      </Button>
      <Button v-if="streaming" variant="outline" size="sm" @click="cancel">
        <Square />
        停止
      </Button>
      <span class="mx-1 h-4 w-px bg-border"></span>
      <Button variant="ghost" size="icon" title="粘贴剪贴板" @click="pasteFromClipboard">
        <ClipboardPaste />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        title="清空"
        :disabled="!source.trim() && !result"
        @click="clearAll"
      >
        <Eraser />
      </Button>
      <Button variant="ghost" size="icon" title="复制译文" :disabled="!result" @click="copyResult">
        <Check v-if="copied" class="text-jue-ink" />
        <Copy v-else />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        :title="speaking ? '停止朗读' : '朗读'"
        :disabled="!result && !source.trim()"
        @click="speak"
      >
        <Volume2 :class="speaking && 'text-gong-ink'" />
      </Button>
      <span class="flex-1"></span>
      <span class="lt-num text-[10px] text-muted-foreground/60">
        选中文字按 {{ settings.hotkey || "Alt+Q" }} 划词
      </span>
    </div>
  </div>
</template>
