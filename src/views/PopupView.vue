<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { api } from "@/api";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import { Check, Copy, Maximize2, Pin, PinOff, RotateCcw, Volume2, X } from "@lucide/vue";
import type { Profile, StreamMsg } from "@/types";
import { langLabel } from "@/types";
import { TONES, toneIndex } from "@/theme/tones";
import { resolvedTheme } from "@/lib/theme";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Toaster } from "@/components/ui/sonner";

const source = ref("");
const result = ref("");
const status = ref<"idle" | "loading" | "streaming" | "done" | "error">("idle");
const errorMsg = ref("");
const langInfo = ref("");
const dictMode = ref(false);
const pinned = ref(false);
const copied = ref(false);
const resultBox = ref<HTMLElement | null>(null);
const transMode = ref("auto");
const sourceLang = ref("auto");
const targetLang = ref("Simplified Chinese");
const profiles = ref<Profile[]>([]);
const activeId = ref("");
const prevActiveId = ref("");
const autoCopy = ref(true);
const cardKey = ref(0);

const isConfigError = computed(() => /API Key|配置|设置|热键/.test(errorMsg.value));

/** 当前配置的调,弹卡与主窗口、托盘显示同一个字 */
const tone = computed(() => TONES[toneIndex(activeId.value)]);
const activeName = computed(() => profiles.value.find((p) => p.id === activeId.value)?.name ?? "");

function composeLangInfo(detected?: string, target?: string, dict?: boolean): string {
  if (dict) return "词典释义";
  if (transMode.value === "custom") {
    const src =
      sourceLang.value && sourceLang.value !== "auto"
        ? langLabel(sourceLang.value)
        : detected || "自动";
    return `${src} → ${langLabel(target || targetLang.value)}`;
  }
  return `${detected || "自动"} → 中文(简体)`;
}

async function start(text: string) {
  source.value = text;
  result.value = "";
  errorMsg.value = "";
  langInfo.value = "";
  dictMode.value = false;
  status.value = "loading";
  const onEvent = (m: StreamMsg) => {
    switch (m.type) {
      case "start":
        dictMode.value = !!m.dict;
        langInfo.value = composeLangInfo(m.detected, m.target, m.dict);
        status.value = "streaming";
        cardKey.value++;
        break;
      case "delta":
        result.value += m.text ?? "";
        status.value = "streaming";
        nextTick(() => {
          if (resultBox.value) resultBox.value.scrollTop = resultBox.value.scrollHeight;
        });
        break;
      case "done":
        status.value = "done";
        if (autoCopy.value) {
          // 后端已复制,前端给个视觉提示
          copied.value = true;
          setTimeout(() => (copied.value = false), 1500);
        }
        break;
      case "error":
        errorMsg.value = m.message ?? "翻译失败";
        status.value = "error";
        break;
      case "cancelled":
        status.value = "done";
        break;
    }
  };
  try {
    await api.translate(text, "popup", onEvent);
  } catch (e) {
    errorMsg.value = String(e);
    status.value = "error";
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  // 跟随设置中的默认目标语言与配置档案
  try {
    const s = await api.getSettings();
    transMode.value = s.transMode || "auto";
    sourceLang.value = s.sourceLang || "auto";
    targetLang.value = s.targetLang || "Simplified Chinese";
    autoCopy.value = s.autoCopy !== false;
    profiles.value = s.profiles;
    activeId.value = s.activeId || s.profiles[0]?.id || "";
    prevActiveId.value = activeId.value;
  } catch {
    /* 保持默认 */
  }
  // 首次创建时页面加载较慢,由后端暂存文本,挂载后主动取
  const pending = await api.takePendingText();
  const pendingErr = await api.takePendingError();
  if (pendingErr) {
    source.value = "";
    errorMsg.value = pendingErr;
    status.value = "error";
  } else if (pending) {
    await start(pending);
  }
  // 之后每次唤起由事件推送
  await listen<{ text: string }>("do-translate", (e) => start(e.payload.text));
  await listen<{ message: string }>("translate-error", (e) => {
    source.value = "";
    result.value = "";
    errorMsg.value = e.payload.message;
    status.value = "error";
  });
});

onUnmounted(() => window.removeEventListener("keydown", onKeydown));

function onKeydown(ev: KeyboardEvent) {
  if (ev.key === "Escape") api.hidePopup();
}

async function togglePin() {
  pinned.value = await api.togglePin();
}

async function copy() {
  if (!result.value) return;
  await api.writeClipboard(result.value);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}

function speak() {
  const text = result.value || source.value;
  if (text) api.ttsSpeak(text);
}

async function openMain() {
  await api.openMainWithText(source.value);
  await api.hidePopup();
}

async function switchProfile() {
  if (!activeId.value) return;
  const prev = prevActiveId.value;
  try {
    await api.setActiveProfile(activeId.value);
    prevActiveId.value = activeId.value;
    toast.success(`已切换到「${activeName.value}」`);
  } catch (e) {
    activeId.value = prev; // 失败回滚下拉显示
    toast.error(String(e));
  }
}

async function openSettings() {
  await api.openMainSettings();
  await api.hidePopup();
}
</script>

<template>
  <!-- 弹卡窗口 420x330 且不可缩放。卡片直接铺满整个窗口:
       早先留了 6px 外边距(为了阴影有落脚处),但那圈透明边在浅色桌面上
       会被看成"白边",在深色桌面上又像一圈脏边 —— 用户反馈的就是这个。
       现在取消外边距,阴影改由窗口自身承担,卡片只用细边框勾轮廓。 -->
  <div class="h-screen overflow-hidden">
    <Toaster :theme="resolvedTheme" position="top-center" />
    <div
      :key="cardKey"
      class="lt-pop lt-card flex h-screen flex-col overflow-hidden rounded-[10px] border border-border/80"
    >
      <!-- 顶条:调式 + 方向读数 + 操作。
           整条设为拖拽区(data-tauri-drag-region="deep"),否则这个无边框
           窗口没有任何地方能拖动 —— 用户拖着卡片想挪位置时会发现纹丝不动。
           右侧按钮不受影响:Tauri 的 drag.js 会跳过 BUTTON/INPUT 等可点击
           元素,它们仍然正常响应点击。 -->
      <div
        data-tauri-drag-region="deep"
        class="flex h-8 shrink-0 items-center gap-1.5 border-b border-border/70 px-2 select-none"
      >
        <span
          class="grid size-[17px] shrink-0 place-items-center rounded-[3px] text-[10px] leading-none font-semibold"
          :style="{ background: tone.color, color: 'oklch(0.19 0.02 258)' }"
          :title="`${tone.name}调 · ${activeName}`"
          >{{ tone.name }}</span
        >
        <span class="lt-num truncate text-[11px] text-muted-foreground">
          {{ langInfo || "准备翻译" }}
        </span>
        <span class="flex-1"></span>
        <Button
          variant="ghost"
          size="icon-xs"
          :class="pinned && 'text-gong-ink'"
          :title="pinned ? '取消钉住' : '钉住(失焦不隐藏)'"
          @click="togglePin"
        >
          <PinOff v-if="pinned" />
          <Pin v-else />
        </Button>
        <Button
          v-if="source && (status === 'done' || status === 'error')"
          variant="ghost"
          size="icon-xs"
          title="重新翻译"
          @click="start(source)"
        >
          <RotateCcw />
        </Button>
        <Button variant="ghost" size="icon-xs" title="在主窗口中打开" @click="openMain">
          <Maximize2 />
        </Button>
        <Button variant="ghost" size="icon-xs" title="关闭 (Esc)" @click="api.hidePopup()">
          <X />
        </Button>
      </div>

      <!-- 原文:三行封顶。首帧尚未取到文本时留空,不要塞占位文案,
           否则会被误读成"识别出来的原文" -->
      <div
        v-if="source"
        class="line-clamp-3 shrink-0 border-b border-border/50 px-2.5 py-1.5 text-[12px] leading-[1.55] text-muted-foreground"
        :title="source"
      >
        {{ source }}
      </div>

      <!-- 译文:主角 -->
      <div ref="resultBox" class="min-h-0 flex-1 overflow-y-auto px-2.5 py-2">
        <div v-if="status === 'error'" class="text-[12px] leading-5 text-destructive">
          {{ errorMsg }}
          <div v-if="isConfigError" class="mt-2">
            <Button size="xs" @click="openSettings">打开设置</Button>
          </div>
        </div>
        <div
          v-else-if="status === 'loading'"
          class="flex items-center gap-1.5 text-[12px] text-muted-foreground"
        >
          <span class="size-1.5 animate-pulse rounded-full bg-gong"></span>
          正在翻译…
        </div>
        <div v-else class="text-[13.5px] leading-[1.72] whitespace-pre-wrap">
          {{ result }}<span v-if="status === 'streaming'" class="lt-cursor"></span>
        </div>
      </div>

      <!-- 底条:复制 / 朗读 / 切换配置。
           同样设为拖拽区:顶条只有 32px,手指能抓的余量太小,底条中间
           那片空白(两个按钮与右侧配置下拉之间)是第二处可拖的地方。 -->
      <div
        data-tauri-drag-region="deep"
        class="flex h-8 shrink-0 items-center gap-0.5 border-t border-border/70 px-1.5 select-none"
      >
        <Button variant="ghost" size="xs" :disabled="!result" @click="copy">
          <Check v-if="copied" class="text-jue-ink" />
          <Copy v-else />
          <span class="text-[11px]">{{ copied ? "已复制" : "复制" }}</span>
        </Button>
        <Button variant="ghost" size="xs" :disabled="!result && !source" @click="speak">
          <Volume2 />
          <span class="text-[11px]">朗读</span>
        </Button>
        <span class="flex-1"></span>
        <span v-if="dictMode" class="lt-num mr-1 text-[10px] text-jue-ink">词典</span>
        <!-- SelectTrigger 自带 data-[size=default]:h-9(36px),会把 32px 高的底条
             撑破 4px(实测 scrollHeight 34 > clientHeight 31),必须把高度也顶掉,
             只写 h-6 会被基础样式的 h-9 覆盖。 -->
        <Select v-if="profiles.length" v-model="activeId" @update:model-value="switchProfile">
          <SelectTrigger
            class="!h-6 w-auto max-w-36 gap-1 border-0 bg-transparent px-1.5 py-0 text-[11px] shadow-none hover:bg-accent"
            title="切换模型配置"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent align="end">
            <SelectItem v-for="p in profiles" :key="p.id" :value="p.id">
              <span class="lt-num mr-1 text-[10px] opacity-60">{{
                TONES[toneIndex(p.id)].name
              }}</span>
              {{ p.name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  </div>
</template>
