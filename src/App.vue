<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import {
  Check,
  ClipboardPaste,
  Copy,
  Eraser,
  History,
  Languages,
  Loader2,
  Plus,
  Settings2,
  Square,
  Volume2,
} from "@lucide/vue";
import { genId, langLabel, PROTOCOLS, protocolShort } from "./types";
import type { HistoryItem, Preset, Profile, Settings, StreamMsg } from "./types";
import { cn } from "@/lib/utils";
import { toneMode } from "./tones";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Toaster } from "@/components/ui/sonner";
import LanguageSelect from "./components/LanguageSelect.vue";
import ModelCombobox from "./components/ModelCombobox.vue";
import ToneChip from "./components/ToneChip.vue";
import ToneMark from "./components/ToneMark.vue";
import TranslateDirection from "./components/TranslateDirection.vue";

const tab = ref<"translate" | "history" | "settings">("translate");
const source = ref("");
const result = ref("");
const streaming = ref(false);
const errorMsg = ref("");
const langInfo = ref("");
const speaking = ref(false);
const copiedResult = ref(false);
const resultBox = ref<HTMLElement | null>(null);

const settings = ref<Settings | null>(null);
const presets = ref<Preset[]>([]);
const historyItems = ref<HistoryItem[]>([]);

// 档案编辑状态
const editing = ref<Profile | null>(null);
const testMsg = ref<Record<string, string>>({});
const saveErr = ref("");
const fetchedModels = ref<string[]>([]);
const modelLoading = ref(false);
const modelErr = ref("");
const editorEl = ref<HTMLElement | null>(null);

// 删除确认
const confirmDelete = ref(false);
const deleteTarget = ref<Profile | null>(null);

let channel: Channel<StreamMsg> | null = null;

const activeProfile = computed<Profile | null>(() => {
  const s = settings.value;
  if (!s) return null;
  return s.profiles.find((p) => p.id === s.activeId) ?? s.profiles[0] ?? null;
});

const activeReady = computed(() => {
  const p = activeProfile.value;
  if (!p) return false;
  return p.provider === "ollama" || !!p.apiKey.trim();
});

const urlHint = computed(() => {
  return PROTOCOLS.find((x) => x.value === editing.value?.protocol)?.hint ?? "";
});

const modelOptions = computed(() => {
  const preset = presets.value.find((x) => x.id === editing.value?.provider);
  return Array.from(new Set([...(preset?.models ?? []), ...fetchedModels.value]));
});

/** 未翻译时也显示将要使用的方向,让「当前的设置」始终可见 */
const plannedDirection = computed(() => {
  const s = settings.value;
  if (!s) return "";
  if (s.transMode === "custom") {
    const src = s.sourceLang && s.sourceLang !== "auto" ? langLabel(s.sourceLang) : "自动";
    return `${src} → ${langLabel(s.targetLang)}`;
  }
  return "自动 → 中文(简体)";
});

const directionReadout = computed(() => langInfo.value || plannedDirection.value);
const sourceChars = computed(() => source.value.length);

function composeLangInfo(detected?: string, target?: string, dict?: boolean): string {
  if (dict) return "词典释义";
  const s = settings.value;
  if (!s) return "";
  if (s.transMode === "custom") {
    const src =
      s.sourceLang && s.sourceLang !== "auto" ? langLabel(s.sourceLang) : detected || "自动";
    return `${src} → ${langLabel(target || s.targetLang)}`;
  }
  return `${detected || "自动"} → 中文(简体)`;
}

onMounted(async () => {
  await Promise.all([
    refreshSettings(),
    invoke<Preset[]>("get_presets").then((p) => (presets.value = p)),
  ]);
  await listen<{ text: string }>("fill-source", (e) => {
    source.value = e.payload.text;
    tab.value = "translate";
  });
  await listen("open-settings", () => {
    tab.value = "settings";
  });
  await listen("tray-hint", () => {
    toast.info("已最小化到托盘,选中文字按 Alt+Q 随时划词", { duration: 6000 });
  });
  await listen<{ message: string }>("app-notify", (e) => {
    toast.error(e.payload.message, { duration: 10000 });
  });
});

// 输入即译:800ms 防抖
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
watch(source, (v) => {
  if (!settings.value?.autoTranslate) return;
  if (debounceTimer) clearTimeout(debounceTimer);
  if (!v.trim()) return;
  debounceTimer = setTimeout(() => translate(), 800);
});

function clearAll() {
  if (streaming.value) invoke("cancel_translate", { source: "main" });
  source.value = "";
  result.value = "";
  errorMsg.value = "";
  langInfo.value = "";
  streaming.value = false;
}

async function loadHistory() {
  historyItems.value = await invoke<HistoryItem[]>("get_history");
}

function fmtTime(ts: number): string {
  const d = new Date(ts * 1000);
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  return sameDay
    ? hm
    : `${String(d.getMonth() + 1).padStart(2, "0")}/${String(d.getDate()).padStart(2, "0")} ${hm}`;
}

async function clearHistory() {
  await invoke("clear_history");
  historyItems.value = [];
}

async function pasteFromClipboard() {
  const text = await invoke<string | null>("read_clipboard");
  if (text) source.value = text;
}

watch(tab, (t) => {
  if (t === "history") loadHistory();
});

async function refreshSettings() {
  settings.value = await invoke<Settings>("get_settings");
}

function onMsg(m: StreamMsg) {
  switch (m.type) {
    case "start":
      langInfo.value = composeLangInfo(m.detected, m.target, m.dict);
      break;
    case "delta":
      result.value += m.text ?? "";
      nextTick(() => {
        if (resultBox.value) resultBox.value.scrollTop = resultBox.value.scrollHeight;
      });
      break;
    case "done":
      streaming.value = false;
      break;
    case "error":
      errorMsg.value = m.message ?? "未知错误";
      streaming.value = false;
      break;
    case "cancelled":
      streaming.value = false;
      break;
  }
}

async function translate() {
  const text = source.value.trim();
  if (!text) return;
  await cancel();
  result.value = "";
  errorMsg.value = "";
  langInfo.value = "";
  streaming.value = true;
  channel = new Channel<StreamMsg>();
  channel.onmessage = onMsg;
  try {
    await invoke("translate", {
      text,
      source: "main",
      onEvent: channel,
    });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    streaming.value = false;
  }
}

async function cancel() {
  if (streaming.value) await invoke("cancel_translate", { source: "main" });
}

async function copyResult() {
  if (result.value) {
    await invoke("write_clipboard", { text: result.value });
    copiedResult.value = true;
    setTimeout(() => (copiedResult.value = false), 1400);
    toast.success("已复制译文");
  }
}

async function speak() {
  const text = result.value || source.value;
  if (!text) return;
  if (speaking.value) {
    await invoke("tts_stop");
    speaking.value = false;
    return;
  }
  await invoke("tts_speak", { text });
  speaking.value = true;
}

/* ────────── 模型配置档案 ────────── */

async function persist(silent = false) {
  saveErr.value = "";
  try {
    await invoke("save_settings", { settings: settings.value });
    if (!silent) toast.success("已保存");
    return true;
  } catch (e) {
    saveErr.value = String(e);
    toast.error(String(e));
    return false;
  }
}

function applyPreset() {
  const e = editing.value;
  if (!e) return;
  const p = presets.value.find((x) => x.id === e.provider);
  if (p) {
    e.baseUrl = p.baseUrl;
    e.model = p.model;
    e.protocol = p.protocol;
  }
  fetchedModels.value = [];
  modelErr.value = "";
}

function newProfile() {
  const dp = presets.value.find((p) => p.id === "deepseek");
  editing.value = {
    id: genId(),
    name: "",
    provider: "deepseek",
    protocol: dp?.protocol ?? "openai",
    baseUrl: dp?.baseUrl ?? "",
    model: dp?.model ?? "",
    apiKey: "",
    temperature: 0.3,
  };
  scrollToEditor();
}

function editProfile(p: Profile) {
  editing.value = { ...p };
  scrollToEditor();
}

function scrollToEditor() {
  nextTick(() => {
    editorEl.value?.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}

async function saveProfile() {
  const s = settings.value;
  const e = editing.value;
  if (!s || !e) return;
  if (!e.name.trim()) e.name = e.model.trim() || "未命名配置";
  const list = [...s.profiles];
  const i = list.findIndex((x) => x.id === e.id);
  if (i >= 0) list[i] = { ...e };
  else list.push({ ...e });
  s.profiles = list;
  if (!s.activeId || !list.some((x) => x.id === s.activeId)) {
    s.activeId = e.id;
  }
  editing.value = null;
  if (await persist(true)) toast.success(`已保存「${e.name}」`);
}

function askDelete(p: Profile) {
  deleteTarget.value = p;
  confirmDelete.value = true;
}

async function confirmDeleteNow() {
  const s = settings.value;
  const p = deleteTarget.value;
  if (!s || !p) return;
  s.profiles = s.profiles.filter((x) => x.id !== p.id);
  if (s.activeId === p.id) s.activeId = s.profiles[0]?.id ?? "";
  if (editing.value?.id === p.id) editing.value = null;
  deleteTarget.value = null;
  if (await persist(true)) toast.success(`已删除「${p.name}」`);
}

async function activateProfile(id: string) {
  if (!settings.value) return;
  settings.value.activeId = id;
  await persist(true);
  toast.success(`已启用「${activeProfile.value?.name ?? id}」`);
}

async function testProfile(p: Profile) {
  testMsg.value[p.id] = "测试中…";
  try {
    testMsg.value[p.id] = await invoke<string>("test_profile", { profile: p });
  } catch (e) {
    testMsg.value[p.id] = String(e);
  }
}

async function fetchModels() {
  if (!editing.value) return;
  modelLoading.value = true;
  modelErr.value = "";
  try {
    fetchedModels.value = await invoke<string[]>("fetch_models", { profile: editing.value });
    if (!fetchedModels.value.length) modelErr.value = "接口未返回模型列表";
  } catch (e) {
    modelErr.value = String(e);
  } finally {
    modelLoading.value = false;
  }
}

function setTemperature(v: number | number[]) {
  if (editing.value) {
    editing.value.temperature = Array.isArray(v) ? v[0] : v;
  }
}
</script>

<template>
  <div class="h-screen">
    <Toaster theme="system" position="top-center" />

    <Tabs v-model="tab" class="flex h-full flex-col gap-0">
      <!-- 顶栏:五音立标 + 字数标牌 + 页签 + 当前调式读数 -->
      <header class="flex h-12 shrink-0 items-center gap-4 border-b bg-card pr-3 pl-4 select-none">
        <div class="flex items-center gap-2.5">
          <ToneMark :id="activeProfile?.id" size="sm" />
          <span class="lt-wordmark">LiteTrans</span>
        </div>

        <TabsList class="h-8 gap-0.5 bg-transparent p-0">
          <TabsTrigger
            value="translate"
            class="h-full rounded-md border-0 px-3 data-[state=active]:bg-secondary data-[state=active]:shadow-none"
          >
            <Languages class="size-3.5" />
            翻译
          </TabsTrigger>
          <TabsTrigger
            value="history"
            class="h-full rounded-md border-0 px-3 data-[state=active]:bg-secondary data-[state=active]:shadow-none"
          >
            <History class="size-3.5" />
            历史
          </TabsTrigger>
          <TabsTrigger
            value="settings"
            class="h-full rounded-md border-0 px-3 data-[state=active]:bg-secondary data-[state=active]:shadow-none"
          >
            <Settings2 class="size-3.5" />
            设置
          </TabsTrigger>
        </TabsList>

        <span class="flex-1"></span>

        <!-- 当前调式:点一下进设置 -->
        <button
          v-if="activeProfile"
          class="flex max-w-[52%] items-center gap-2 rounded-md px-1.5 py-1 transition-colors hover:bg-accent"
          :title="`当前配置:${activeProfile.name}(${toneMode(activeProfile.id)})`"
          @click="tab = 'settings'"
        >
          <ToneChip :id="activeProfile.id" />
          <span class="truncate text-xs text-muted-foreground">
            {{ activeProfile.name }}
            <span v-if="activeProfile.model" class="lt-num opacity-70">
              · {{ activeProfile.model }}
            </span>
          </span>
        </button>
      </header>

      <!-- ────────── 翻译 ────────── -->
      <TabsContent value="translate" class="mt-0 min-h-0 flex-1">
        <div v-if="settings" class="flex h-full flex-col">
          <div
            v-if="!activeReady"
            class="flex shrink-0 items-center gap-3 border-b border-gong/40 bg-gong/10 px-4 py-2"
          >
            <span class="text-xs">还没有可用的模型配置,添加服务商与 API Key 后即可翻译。</span>
            <span class="flex-1"></span>
            <Button size="xs" @click="tab = 'settings'">添加配置</Button>
          </div>

          <!-- 工具行 -->
          <div class="flex shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-b px-4 py-2">
            <TranslateDirection :settings="settings" @change="persist(true)" />
            <span class="flex-1"></span>
            <span class="lt-num text-[11px] text-muted-foreground">
              {{ settings.hotkey || "Alt+Q" }} 划词 · Ctrl+Enter 翻译
            </span>
          </div>

          <!-- 输入区:与译文区等分,不抢戏 -->
          <div class="relative min-h-0 flex-1 border-b border-border/70">
            <Textarea
              v-model="source"
              class="h-full resize-none rounded-none border-0 bg-transparent px-4 py-3 text-[15px] leading-7 shadow-none field-sizing-fixed focus-visible:ring-0"
              placeholder="粘贴或输入要翻译的文本,Ctrl+Enter 翻译"
              @keydown.ctrl.enter.prevent="translate"
            />
          </div>

          <!-- 方向读数分隔条 -->
          <div class="flex shrink-0 items-center gap-3 px-4 py-2">
            <span class="h-px flex-1 bg-border"></span>
            <span class="lt-num flex items-center gap-1.5 text-[11px] text-muted-foreground">
              <span
                class="size-1.5 rounded-full transition-colors"
                :class="streaming ? 'bg-gong' : 'bg-shang'"
              ></span>
              {{ directionReadout }}
            </span>
            <span class="h-px flex-1 bg-border"></span>
          </div>

          <!-- 译文区:主角 -->
          <div ref="resultBox" class="min-h-0 flex-1 overflow-y-auto px-4 pb-3">
            <div v-if="errorMsg" class="text-sm leading-6 text-destructive">{{ errorMsg }}</div>
            <div
              v-else-if="!result"
              class="flex h-full flex-col items-center justify-center gap-1.5 text-xs text-muted-foreground"
            >
              <span>译文会显示在这里</span>
              <span class="opacity-70">
                选中任意文字按
                <kbd class="lt-num rounded border bg-muted px-1 py-px">{{ settings.hotkey }}</kbd>
                也能划词翻译
              </span>
            </div>
            <div v-else class="lt-fade text-[15px] leading-[1.8] whitespace-pre-wrap">
              {{ result }}<span v-if="streaming" class="lt-cursor"></span>
            </div>
          </div>

          <!-- 操作行 -->
          <div class="flex shrink-0 items-center gap-1.5 border-t px-3 py-2">
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
            <Button variant="ghost" size="icon-sm" title="粘贴剪贴板" @click="pasteFromClipboard">
              <ClipboardPaste />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="清空"
              :disabled="!source.trim() && !result"
              @click="clearAll"
            >
              <Eraser />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title="复制译文"
              :disabled="!result"
              @click="copyResult"
            >
              <Check v-if="copiedResult" class="text-jue-ink" />
              <Copy v-else />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              :title="speaking ? '停止朗读' : '朗读'"
              :disabled="!result && !source.trim()"
              @click="speak"
            >
              <Volume2 :class="speaking && 'text-gong-ink'" />
            </Button>
            <span class="flex-1"></span>
            <span class="lt-num text-[11px] text-muted-foreground">{{ sourceChars }} 字</span>
          </div>
        </div>
      </TabsContent>

      <!-- ────────── 历史 ────────── -->
      <TabsContent value="history" class="mt-0 min-h-0 flex-1">
        <div class="flex h-full flex-col">
          <div class="flex shrink-0 items-center gap-3 border-b px-4 py-2.5">
            <span class="lt-section">历史记录</span>
            <span class="lt-num text-[11px] text-muted-foreground">
              {{ historyItems.length }} / 50 · 仅存本地
            </span>
            <span class="flex-1"></span>
            <Button
              v-if="historyItems.length"
              variant="ghost"
              size="xs"
              class="text-muted-foreground"
              @click="clearHistory"
              >清空</Button
            >
          </div>

          <div
            v-if="!historyItems.length"
            class="flex flex-1 flex-col items-center justify-center gap-2 text-center"
          >
            <span class="text-sm text-muted-foreground">还没有翻译记录</span>
            <span class="text-xs text-muted-foreground/70">
              翻译过的内容会自动存成历史,点击可重新编辑
            </span>
          </div>

          <div v-else class="min-h-0 flex-1 overflow-y-auto">
            <button
              v-for="(h, i) in historyItems"
              :key="`${h.ts}-${i}`"
              class="flex w-full items-start gap-3 border-b border-border/60 px-4 py-2.5 text-left transition-colors hover:bg-accent/60"
              @click="
                source = h.source;
                tab = 'translate';
              "
            >
              <span class="lt-num w-14 shrink-0 pt-0.5 text-[11px] text-muted-foreground">
                {{ fmtTime(h.ts) }}
              </span>
              <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                <span class="flex items-center gap-2">
                  <ToneChip v-if="h.profileId" :id="h.profileId" />
                  <span class="lt-num truncate text-[11px] text-muted-foreground">
                    {{ h.direction }}<template v-if="h.dict"> · 词典</template>
                    <template v-if="h.profile"> · {{ h.profile }}</template>
                  </span>
                </span>
                <span class="line-clamp-1 text-xs text-muted-foreground">{{ h.source }}</span>
                <span class="line-clamp-2 text-[13px] leading-5">{{ h.result }}</span>
              </span>
            </button>
          </div>
        </div>
      </TabsContent>

      <!-- ────────── 设置 ────────── -->
      <TabsContent value="settings" class="mt-0 min-h-0 flex-1">
        <div v-if="settings" class="h-full overflow-y-auto px-4 pt-3 pb-8">
          <!-- 模型配置 -->
          <div class="flex items-center gap-2">
            <span class="lt-section">模型配置</span>
            <span class="lt-num text-[11px] text-muted-foreground">
              {{ settings.profiles.length }} 条
            </span>
          </div>
          <p class="mt-1 text-xs text-muted-foreground">
            每条配置是一个「调」,随时切换;API Key 只存在 Windows 凭据管理器。
          </p>

          <div
            v-if="!settings.profiles.length && !editing"
            class="mt-3 rounded-lg border border-dashed px-4 py-6 text-center"
          >
            <div class="text-sm">还没有任何配置</div>
            <div class="mt-1 text-xs text-muted-foreground">
              选一个服务商预设,填入 API Key 就能开始
            </div>
            <Button size="sm" class="mt-3" @click="newProfile"><Plus />新增配置</Button>
          </div>

          <div v-else class="mt-1">
            <div
              v-for="p in settings.profiles"
              :key="p.id"
              class="-mx-2 flex items-start gap-3 rounded-md border-b border-border/60 px-2 py-2.5 transition-colors"
              :class="p.id === settings.activeId && 'bg-gong/10'"
            >
              <ToneChip :id="p.id" :filled="p.id === settings.activeId" class="mt-0.5" />

              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
                  <span class="text-sm font-medium">{{ p.name }}</span>
                  <span class="lt-num text-[10px] tracking-wider text-shang-ink uppercase">
                    {{ protocolShort(p.protocol) }}
                  </span>
                  <span v-if="p.id === settings.activeId" class="lt-num text-[10px] text-gong-ink"
                    >· 使用中</span
                  >
                </div>
                <div class="lt-num truncate text-[11px] text-muted-foreground">
                  {{ p.model || "未指定模型" }}
                  <span class="opacity-60">· {{ p.baseUrl || "预设地址" }}</span>
                </div>
                <div
                  v-if="testMsg[p.id]"
                  class="mt-1 text-[11px]"
                  :class="
                    testMsg[p.id].startsWith('连接正常') ? 'text-jue-ink' : 'text-destructive'
                  "
                >
                  {{ testMsg[p.id] }}
                </div>
              </div>

              <div class="flex shrink-0 items-center gap-1">
                <Button
                  v-if="p.id !== settings.activeId"
                  variant="ghost"
                  size="xs"
                  @click="activateProfile(p.id)"
                  >启用</Button
                >
                <Button variant="ghost" size="xs" @click="editProfile(p)">编辑</Button>
                <Button variant="ghost" size="xs" @click="testProfile(p)">测试</Button>
                <Button
                  variant="ghost"
                  size="xs"
                  class="text-muted-foreground hover:text-destructive"
                  @click="askDelete(p)"
                  >删除</Button
                >
              </div>
            </div>
          </div>

          <!-- 编辑器 -->
          <div
            v-if="editing"
            ref="editorEl"
            class="mt-3 flex flex-col gap-3 rounded-lg border border-gong/50 bg-card p-4"
          >
            <div class="flex items-center gap-2">
              <ToneChip :id="editing.id" filled />
              <span class="text-sm font-medium">编辑配置</span>
            </div>

            <div class="space-y-1">
              <span class="lt-label">名称</span>
              <Input v-model="editing.name" placeholder="如 DeepSeek 官方 / 我的 OpenRouter" />
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div class="space-y-1">
                <span class="lt-label">服务商预设</span>
                <Select v-model="editing.provider" @update:model-value="applyPreset">
                  <SelectTrigger class="w-full"
                    ><SelectValue placeholder="选择服务商"
                  /></SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="p in presets" :key="p.id" :value="p.id">{{
                      p.name
                    }}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div class="space-y-1">
                <span class="lt-label">接口协议</span>
                <Select v-model="editing.protocol">
                  <SelectTrigger class="w-full"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="x in PROTOCOLS" :key="x.value" :value="x.value">{{
                      x.label
                    }}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div class="space-y-1">
              <span class="lt-label">API 地址(Base URL)</span>
              <Input
                v-model="editing.baseUrl"
                class="lt-num"
                placeholder="https://api.example.com/v1"
              />
              <div v-if="urlHint" class="text-[11px] text-muted-foreground">{{ urlHint }}</div>
            </div>

            <div class="space-y-1">
              <span class="lt-label">模型</span>
              <div class="flex items-start gap-2">
                <div class="min-w-0 flex-1">
                  <ModelCombobox
                    v-model="editing.model"
                    :options="modelOptions"
                    placeholder="deepseek-chat"
                  />
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  class="h-9 shrink-0"
                  :disabled="modelLoading"
                  @click="fetchModels"
                >
                  <Loader2 v-if="modelLoading" class="animate-spin" />
                  {{ modelLoading ? "获取中…" : "在线获取" }}
                </Button>
              </div>
              <div v-if="modelErr" class="text-[11px] text-destructive">{{ modelErr }}</div>
              <div v-else-if="modelOptions.length" class="text-[11px] text-muted-foreground">
                {{ modelOptions.length }} 个候选模型,可直接输入未列出的模型名
              </div>
            </div>

            <div v-if="editing.provider !== 'ollama'" class="space-y-1">
              <span class="lt-label">API Key</span>
              <Input
                v-model="editing.apiKey"
                type="password"
                class="lt-num"
                placeholder="sk-…"
                autocomplete="off"
              />
            </div>

            <div class="space-y-1.5">
              <div class="flex items-baseline justify-between gap-2">
                <span class="lt-label">Temperature</span>
                <span class="lt-num text-[11px] text-muted-foreground">
                  {{ editing.temperature.toFixed(1) }} · 越低越稳定
                </span>
              </div>
              <Slider
                :model-value="[editing.temperature]"
                :min="0"
                :max="2"
                :step="0.1"
                class="w-full max-w-64"
                @update:model-value="setTemperature"
              />
            </div>

            <div class="flex items-center gap-2 pt-0.5">
              <Button size="sm" @click="saveProfile">保存此配置</Button>
              <Button variant="ghost" size="sm" @click="editing = null">取消</Button>
            </div>
          </div>

          <div v-else class="mt-3">
            <Button variant="outline" size="sm" @click="newProfile"><Plus />新增配置</Button>
          </div>

          <!-- 通用设置 -->
          <div class="mt-7 flex items-center gap-2">
            <span class="lt-section">通用设置</span>
          </div>

          <div class="mt-1">
            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">划词热键</div>
                <div class="text-xs text-muted-foreground">选中文字后按下,自动复制并翻译</div>
              </div>
              <Input v-model="settings.hotkey" class="lt-num h-8 w-44 text-center text-xs" />
            </div>

            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">截图翻译热键</div>
                <div class="text-xs text-muted-foreground">框选屏幕区域,OCR 识别后翻译</div>
              </div>
              <Input v-model="settings.captureHotkey" class="lt-num h-8 w-44 text-center text-xs" />
            </div>

            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">启用划词热键</div>
                <div class="text-xs text-muted-foreground">也可在托盘菜单里临时停用</div>
              </div>
              <Switch v-model="settings.hotkeyEnabled" @update:model-value="persist(true)" />
            </div>

            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">输入即译</div>
                <div class="text-xs text-muted-foreground">停止输入 0.8 秒后自动翻译</div>
              </div>
              <Switch v-model="settings.autoTranslate" @update:model-value="persist(true)" />
            </div>

            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">弹卡自动复制译文</div>
                <div class="text-xs text-muted-foreground">划词翻译完成后直接进剪贴板</div>
              </div>
              <Switch v-model="settings.autoCopy" @update:model-value="persist(true)" />
            </div>

            <div class="flex items-center justify-between gap-4 border-b border-border/60 py-2.5">
              <div>
                <div class="text-sm">开机自启</div>
                <div class="text-xs text-muted-foreground">启动后常驻托盘,不打开主窗口</div>
              </div>
              <Switch v-model="settings.autostart" @update:model-value="persist(true)" />
            </div>

            <div class="flex flex-col gap-2 border-b border-border/60 py-3">
              <div>
                <div class="text-sm">默认翻译方向</div>
                <div class="text-xs text-muted-foreground">划词弹卡与主窗口共用同一方向</div>
              </div>
              <TranslateDirection :settings="settings" @change="persist(true)" />
            </div>

            <div class="flex flex-col gap-1.5 py-3">
              <div>
                <div class="text-sm">附加翻译要求</div>
                <div class="text-xs text-muted-foreground">写进系统提示词,可留空</div>
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
            <Button size="sm" @click="persist">保存设置</Button>
            <span v-if="saveErr" class="text-xs text-destructive">{{ saveErr }}</span>
          </div>
        </div>
      </TabsContent>
    </Tabs>

    <!-- 删除确认 -->
    <AlertDialog v-model:open="confirmDelete">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>删除配置</AlertDialogTitle>
          <AlertDialogDescription>
            确定删除「{{ deleteTarget?.name }}」?其 API Key 凭据会一并移除。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            :class="cn('bg-destructive text-white hover:bg-destructive/90')"
            @click="confirmDeleteNow"
            >删除</AlertDialogAction
          >
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
