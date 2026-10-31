<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import {
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

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
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
import TranslateDirection from "./components/TranslateDirection.vue";

const tab = ref<"translate" | "settings">("translate");
const source = ref("");
const result = ref("");
const streaming = ref(false);
const errorMsg = ref("");
const langInfo = ref("");
const speaking = ref(false);
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

function composeLangInfo(detected?: string, target?: string, dict?: boolean): string {
  if (dict) return "词典查询";
  const s = settings.value;
  if (!s) return "";
  if (s.transMode === "custom") {
    const src =
      s.sourceLang && s.sourceLang !== "auto" ? langLabel(s.sourceLang) : detected || "自动";
    return `${src} → ${langLabel(target || s.targetLang)}`;
  }
  return `${detected || "自动"} → 中文`;
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
  return sameDay ? hm : `${d.getMonth() + 1}/${d.getDate()} ${hm}`;
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
      <header
        class="flex items-center justify-between border-b bg-card px-4 py-2.5 select-none"
      >
        <div class="flex items-center gap-2">
          <span
            class="flex size-6.5 items-center justify-center rounded-[7px] bg-gradient-to-br from-indigo-500 to-blue-600 text-[13px] font-bold text-white"
          >译</span>
          <span class="text-sm font-semibold tracking-wide">LiteTrans</span>
        </div>
        <TabsList>
          <TabsTrigger value="translate">
            <Languages class="size-4" />翻译
          </TabsTrigger>
          <TabsTrigger value="history">
            <History class="size-4" />历史
          </TabsTrigger>
          <TabsTrigger value="settings">
            <Settings2 class="size-4" />设置
          </TabsTrigger>
        </TabsList>
      </header>

      <!-- 翻译页 -->
      <TabsContent value="translate" class="min-h-0 flex-1 mt-0">
        <div v-if="settings" class="flex h-full flex-col gap-2.5 px-4 pt-3 pb-4">
          <div
            v-if="!activeReady"
            class="flex items-center justify-between gap-3 rounded-lg border border-primary/40 bg-primary/10 px-4 py-2.5"
          >
            <span class="text-sm">首次使用:请先添加一条模型配置(服务商 + API Key)。</span>
            <Button size="sm" @click="tab = 'settings'">去配置</Button>
          </div>

          <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
            <TranslateDirection :settings="settings" @change="persist(true)" />
            <span class="text-xs text-muted-foreground">
              Ctrl+Enter 翻译 · 划词按 {{ settings.hotkey || "Alt+Q" }}
              <template v-if="activeProfile">
                · 当前:{{ activeProfile.name }}{{ activeProfile.model ? ` · ${activeProfile.model}` : "" }}
              </template>
            </span>
          </div>

          <Textarea
            v-model="source"
            class="min-h-24 flex-1 resize-none leading-relaxed"
            placeholder="输入要翻译的文本…"
            @keydown.ctrl.enter.prevent="translate"
          />

          <div class="flex flex-wrap items-center gap-2">
            <Button :disabled="!source.trim() || streaming" @click="translate">
              <Loader2 v-if="streaming" class="animate-spin" />
              <Languages v-else />
              翻译
            </Button>
            <Button v-if="streaming" variant="outline" @click="cancel">
              <Square />
              停止
            </Button>
            <Button variant="ghost" size="sm" :disabled="!source.trim() && !result" @click="clearAll">
              <Eraser />
              清空
            </Button>
            <Button variant="ghost" size="sm" @click="pasteFromClipboard">
              <ClipboardPaste />
              粘贴
            </Button>
            <Button variant="ghost" size="sm" :disabled="!result" @click="copyResult">
              <Copy />
              复制
            </Button>
            <Button variant="ghost" size="sm" :disabled="!result && !source.trim()" @click="speak">
              <Volume2 />
              {{ speaking ? "停止朗读" : "朗读" }}
            </Button>
            <span v-if="langInfo" class="ml-1 text-xs text-muted-foreground">{{ langInfo }}</span>
          </div>

          <div
            ref="resultBox"
            class="min-h-28 flex-[1.2] overflow-y-auto rounded-lg border bg-card p-3.5 leading-7 whitespace-pre-wrap"
            :class="{ 'text-muted-foreground': !result && !errorMsg }"
          >
            <div v-if="errorMsg" class="text-destructive text-sm">{{ errorMsg }}</div>
            <template v-else>
              {{ result || "译文会显示在这里" }}
              <span v-if="streaming" class="lt-cursor">▍</span>
            </template>
          </div>
        </div>
      </TabsContent>

      <!-- 历史页 -->
      <TabsContent value="history" class="min-h-0 flex-1 mt-0">
        <div class="flex h-full flex-col gap-2.5 px-4 pt-3 pb-4">
          <div class="flex items-center justify-between">
            <span class="text-[13px] font-semibold tracking-wide text-muted-foreground">
              翻译历史(最近 50 条,仅存本地)
            </span>
            <Button
              v-if="historyItems.length"
              variant="outline" size="sm" class="h-7 text-xs"
              @click="clearHistory"
            >清空历史</Button>
          </div>

          <div v-if="!historyItems.length" class="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
            还没有翻译记录
          </div>
          <div v-else class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
            <button
              v-for="(h, i) in historyItems"
              :key="`${h.ts}-${i}`"
              class="rounded-xl border bg-card p-3 text-left transition-colors hover:border-primary/50"
              @click="source = h.source; tab = 'translate'"
            >
              <div class="flex items-center gap-2 text-[11px] text-muted-foreground">
                <span>{{ fmtTime(h.ts) }}</span>
                <span class="rounded border border-primary/40 px-1.5 text-primary">{{ h.direction }}</span>
                <span>{{ h.profile }}</span>
                <span v-if="h.dict" class="text-primary">词典</span>
              </div>
              <div class="mt-1 line-clamp-2 text-xs text-muted-foreground">{{ h.source }}</div>
              <div class="mt-0.5 line-clamp-3 text-sm">{{ h.result }}</div>
            </button>
          </div>
        </div>
      </TabsContent>

      <!-- 设置页 -->
      <TabsContent value="settings" class="min-h-0 flex-1 mt-0">
        <div v-if="settings" class="flex flex-col gap-4 overflow-y-auto px-4 pt-3 pb-6 h-full">
          <h2 class="text-[13px] font-semibold tracking-wide text-muted-foreground">模型配置</h2>

          <div
            v-if="!settings.profiles.length && !editing"
            class="rounded-xl border border-dashed p-5 text-center text-sm text-muted-foreground"
          >
            还没有配置,点下方「新增配置」添加服务商与 API Key。
          </div>

          <div
            v-for="p in settings.profiles"
            :key="p.id"
            class="rounded-xl border bg-card p-3.5"
            :class="{ 'border-primary bg-primary/10': p.id === settings.activeId }"
          >
            <div class="flex items-center gap-2.5">
              <span class="text-sm font-semibold">{{ p.name }}</span>
              <span class="rounded border border-primary/50 px-1.5 text-[11px] leading-4 text-primary">
                {{ protocolShort(p.protocol) }}
              </span>
              <span v-if="p.model" class="text-xs text-muted-foreground">{{ p.model }}</span>
              <Badge v-if="p.id === settings.activeId" class="ml-auto">使用中</Badge>
            </div>
            <div class="mt-0.5 text-xs break-all text-muted-foreground">
              {{ p.baseUrl || "(使用预设地址)" }}
            </div>
            <div class="mt-2.5 flex flex-wrap items-center gap-1.5">
              <Button
                v-if="p.id !== settings.activeId"
                variant="outline" size="sm" class="h-7 px-3 text-xs"
                @click="activateProfile(p.id)"
              >启用</Button>
              <Button variant="outline" size="sm" class="h-7 px-3 text-xs" @click="editProfile(p)">编辑</Button>
              <Button variant="outline" size="sm" class="h-7 px-3 text-xs" @click="testProfile(p)">测试</Button>
              <Button
                variant="outline" size="sm"
                class="h-7 px-3 text-xs text-destructive hover:text-destructive"
                @click="askDelete(p)"
              >删除</Button>
              <span
                v-if="testMsg[p.id]"
                class="text-xs"
                :class="testMsg[p.id].startsWith('连接正常') ? 'text-emerald-500' : 'text-destructive'"
              >{{ testMsg[p.id] }}</span>
            </div>
          </div>

          <!-- 编辑器 -->
          <div
            v-if="editing"
            ref="editorEl"
            class="flex flex-col gap-3 rounded-xl border border-primary/60 bg-card p-4"
          >
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground">名称</Label>
              <Input v-model="editing.name" placeholder="如 DeepSeek 官方 / 我的 OpenRouter" />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div class="space-y-1.5">
                <Label class="text-xs text-muted-foreground">服务商预设</Label>
                <Select v-model="editing.provider" @update:model-value="applyPreset">
                  <SelectTrigger class="w-full"><SelectValue placeholder="选择服务商" /></SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="p in presets" :key="p.id" :value="p.id">{{ p.name }}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div class="space-y-1.5">
                <Label class="text-xs text-muted-foreground">接口协议</Label>
                <Select v-model="editing.protocol">
                  <SelectTrigger class="w-full"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="x in PROTOCOLS" :key="x.value" :value="x.value">{{ x.label }}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground">API 地址(Base URL)</Label>
              <Input v-model="editing.baseUrl" placeholder="https://api.example.com/v1" />
              <div v-if="urlHint" class="text-[11px] text-muted-foreground">{{ urlHint }}</div>
            </div>
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground">模型(可输入,或从候选/在线列表选择)</Label>
              <div class="flex items-start gap-2">
                <div class="min-w-0 flex-1">
                  <ModelCombobox
                    v-model="editing.model"
                    :options="modelOptions"
                    placeholder="deepseek-chat"
                  />
                </div>
                <Button variant="outline" size="sm" class="h-9 shrink-0 text-xs" :disabled="modelLoading" @click="fetchModels">
                  <Loader2 v-if="modelLoading" class="animate-spin" />
                  {{ modelLoading ? "获取中…" : "在线获取" }}
                </Button>
              </div>
              <div v-if="modelErr" class="text-destructive text-[11px]">{{ modelErr }}</div>
              <div v-else-if="modelOptions.length" class="text-[11px] text-muted-foreground">
                {{ modelOptions.length }} 个候选模型
              </div>
            </div>
            <div v-if="editing.provider !== 'ollama'" class="space-y-1.5">
              <Label class="text-xs text-muted-foreground">API Key</Label>
              <Input v-model="editing.apiKey" type="password" placeholder="sk-…" autocomplete="off" />
            </div>
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground">
                Temperature(0 = 更稳定,1 = 更有创造性)· 当前 {{ editing.temperature.toFixed(1) }}
              </Label>
              <Slider
                :model-value="[editing.temperature]"
                :min="0" :max="2" :step="0.1"
                class="w-64"
                @update:model-value="setTemperature"
              />
            </div>
            <div class="flex items-center gap-2">
              <Button size="sm" @click="saveProfile">保存此配置</Button>
              <Button variant="ghost" size="sm" @click="editing = null">取消</Button>
            </div>
          </div>

          <div v-else>
            <Button variant="outline" @click="newProfile"><Plus />新增配置</Button>
          </div>

          <h2 class="mt-2 text-[13px] font-semibold tracking-wide text-muted-foreground">通用设置</h2>

          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground">划词热键</Label>
            <Input v-model="settings.hotkey" class="max-w-60" placeholder="如 alt+q / ctrl+shift+t / f9" />
          </div>

          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground">截图翻译热键(框选屏幕区域识别并翻译)</Label>
            <Input v-model="settings.captureHotkey" class="max-w-60" placeholder="如 alt+x / ctrl+shift+s" />
          </div>

          <div class="flex items-center gap-2.5">
            <Switch v-model="settings.hotkeyEnabled" @update:model-value="persist(true)" />
            <span class="text-sm">启用划词热键(临时停用可在托盘菜单操作)</span>
          </div>

          <div class="flex items-center gap-2.5">
            <Switch v-model="settings.autoTranslate" @update:model-value="persist(true)" />
            <span class="text-sm">输入即译(停止输入 0.8 秒后自动翻译)</span>
          </div>

          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground">默认翻译方向(划词弹卡与主窗口共用)</Label>
            <TranslateDirection :settings="settings" @change="persist(true)" />
          </div>

          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground">附加翻译要求(可选,写进系统提示词)</Label>
            <Textarea
              v-model="settings.customPrompt"
              :rows="2"
              class="max-w-xl resize-none"
              placeholder="例如:术语按 XX 译法;语气保持正式"
            />
          </div>

          <div class="flex items-center gap-2.5">
            <Switch v-model="settings.autoCopy" @update:model-value="persist(true)" />
            <span class="text-sm">划词弹卡自动复制译文</span>
          </div>

          <div class="flex items-center gap-2.5">
            <Switch v-model="settings.autostart" @update:model-value="persist(true)" />
            <span class="text-sm">开机自启(启动后常驻托盘)</span>
          </div>

          <div class="flex items-center gap-3">
            <Button size="sm" @click="persist">保存设置</Button>
            <span v-if="saveErr" class="text-destructive text-xs">{{ saveErr }}</span>
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
          >删除</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
