<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import { BookOpen, Copy, Maximize2, Pin, PinOff, Volume2, X } from "@lucide/vue";
import type { Profile, Settings, StreamMsg } from "./types";
import { langLabel } from "./types";

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

const isConfigError = computed(() =>
  /API Key|配置|设置|热键/.test(errorMsg.value),
);

function composeLangInfo(detected?: string, target?: string, dict?: boolean): string {
  if (dict) return "词典查询";
  if (transMode.value === "custom") {
    const src =
      sourceLang.value && sourceLang.value !== "auto"
        ? langLabel(sourceLang.value)
        : detected || "自动";
    return `${src} → ${langLabel(target || targetLang.value)}`;
  }
  return `${detected || "自动"} → 中文`;
}

async function start(text: string) {
  source.value = text;
  result.value = "";
  errorMsg.value = "";
  langInfo.value = "";
  status.value = "loading";
  const ch = new Channel<StreamMsg>();
  ch.onmessage = (m) => {
    switch (m.type) {
      case "start":
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
    await invoke("translate", { text, source: "popup", onEvent: ch });
  } catch (e) {
    errorMsg.value = String(e);
    status.value = "error";
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  // 跟随设置中的默认目标语言与配置档案
  try {
    const s = await invoke<Settings>("get_settings");
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
  const pending = await invoke<string | null>("take_pending_text");
  if (pending) await start(pending);
  // 之后每次唤起由事件推送
  await listen<{ text: string }>("do-translate", (e) => start(e.payload.text));
});

onUnmounted(() => window.removeEventListener("keydown", onKeydown));

function onKeydown(ev: KeyboardEvent) {
  if (ev.key === "Escape") invoke("hide_popup");
}

async function togglePin() {
  pinned.value = await invoke<boolean>("toggle_pin");
}

async function copy() {
  if (!result.value) return;
  await invoke("write_clipboard", { text: result.value });
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}

function speak() {
  const text = result.value || source.value;
  if (text) invoke("tts_speak", { text });
}

async function openMain() {
  await invoke("open_main_with_text", { text: source.value });
  await invoke("hide_popup");
}

async function switchProfile() {
  if (!activeId.value) return;
  const prev = prevActiveId.value;
  try {
    await invoke("set_active_profile", { id: activeId.value });
    prevActiveId.value = activeId.value;
    toast.success("已切换模型配置");
  } catch (e) {
    activeId.value = prev; // 失败回滚下拉显示
    toast.error(String(e));
  }
}

async function openSettings() {
  await invoke("open_main_settings");
  await invoke("hide_popup");
}
</script>

<template>
  <div class="h-screen">
    <Toaster theme="system" position="top-center" />
    <div
      :key="cardKey"
      class="lt-pop m-1.5 flex h-[calc(100vh-12px)] flex-col rounded-xl border bg-popover/85 p-2.5 shadow-xl backdrop-blur-xl"
    >
      <div class="flex items-center gap-1">
        <BookOpen v-if="langInfo === '词典查询'" class="size-3.5 text-primary" />
        <span class="text-xs text-muted-foreground">{{ langInfo || "翻译" }}</span>
        <span class="flex-1"></span>
        <Button
          variant="ghost" size="icon"
          class="size-6 rounded-md"
          :class="pinned && 'text-primary'"
          :title="pinned ? '取消钉住' : '钉住(失焦不隐藏)'"
          @click="togglePin"
        >
          <PinOff v-if="pinned" />
          <Pin v-else />
        </Button>
        <Button variant="ghost" size="icon" class="size-6 rounded-md" title="在主窗口中打开" @click="openMain">
          <Maximize2 />
        </Button>
        <Button variant="ghost" size="icon" class="size-6 rounded-md" title="关闭 (Esc)" @click="invoke('hide_popup')">
          <X />
        </Button>
      </div>

      <div class="line-clamp-3 border-b pb-2 leading-5 text-muted-foreground" :title="source" style="font-size: 12px">
        {{ source }}
      </div>

      <div ref="resultBox" class="min-h-0 flex-1 overflow-y-auto pt-2 leading-relaxed whitespace-pre-wrap text-sm">
        <div v-if="status === 'error'" class="text-destructive text-xs">
          {{ errorMsg }}
          <div v-if="isConfigError" class="mt-2">
            <Button size="sm" class="h-7 text-xs" @click="openSettings">去设置</Button>
          </div>
        </div>
        <div v-else-if="status === 'loading'" class="text-muted-foreground text-xs">翻译中…</div>
        <template v-else>
          {{ result }}<span v-if="status === 'streaming'" class="lt-cursor">▍</span>
        </template>
      </div>

      <div class="flex items-center gap-1 pt-1.5">
        <Button variant="ghost" size="sm" class="h-6 px-2 text-xs" :disabled="!result" @click="copy">
          <Copy class="size-3.5" />
          {{ copied ? "已复制" : "复制" }}
        </Button>
        <Button variant="ghost" size="sm" class="h-6 px-2 text-xs" :disabled="!result && !source" @click="speak">
          <Volume2 class="size-3.5" />
          朗读
        </Button>
        <span class="flex-1"></span>
        <Select v-if="profiles.length" v-model="activeId" @update:model-value="switchProfile">
          <SelectTrigger
            class="h-7 w-36 gap-1 border-none bg-transparent px-2 text-xs shadow-none hover:bg-accent"
            title="切换模型配置"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="p in profiles" :key="p.id" :value="p.id">{{ p.name }}</SelectItem>
          </SelectContent>
        </Select>
        <span v-if="status === 'streaming' || status === 'loading'" class="text-[11px] text-muted-foreground">
          生成中
        </span>
      </div>
    </div>
  </div>
</template>
