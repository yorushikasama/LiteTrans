<script setup lang="ts">
/**
 * 主窗口外壳:侧栏 + 三个页面 + 全局事件接线。
 *
 * 这里只做三件事:持有各页面的状态、把后端事件分发下去、把页面之间
 * 的跳转接起来。具体页面在 views/,页面内部逻辑在 composables/。
 */
import { onMounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import { api } from "@/api";
import { toneMode } from "@/theme/tones";
import type { HistoryItem, RuntimeInfo } from "@/types";
import { useSettings } from "@/composables/useSettings";
import { useTranslate } from "@/composables/useTranslate";
import SideBar from "@/layout/SideBar.vue";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { Toaster } from "@/components/ui/sonner";
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
import { cn } from "@/lib/utils";
import HistoryView from "@/views/HistoryView.vue";
import SettingsView from "@/views/SettingsView.vue";
import TranslateView from "@/views/TranslateView.vue";

type TabValue = "translate" | "history" | "settings";

const tab = ref<TabValue>("translate");

const s = useSettings();
const t = useTranslate(() => s.settings.value);

const historyItems = ref<HistoryItem[]>([]);
const runtime = ref<RuntimeInfo | null>(null);

const { settings, presets, activeProfile, activeReady, persist, load } = s;
/**
 * 删除确认对话框用的两个状态。
 *
 * 模板里不能写 `s.confirmDelete.value`:对象本身的成员不会自动解包,
 * 只有顶层 ref 才会。所以这里单独取出来,模板直接用自动解包后的值。
 */
const confirmDelete = s.confirmDelete;
const deleteTarget = s.deleteTarget;
function confirmDeleteNow() {
  void s.confirmDeleteNow();
}

async function loadRuntime() {
  try {
    runtime.value = await api.getRuntimeInfo();
  } catch {
    // 浏览器调试态没有这个命令,静默跳过
  }
}

async function loadHistory() {
  historyItems.value = await api.getHistory();
}

async function clearHistory() {
  await api.clearHistory();
  historyItems.value = [];
}

/** 历史里点一条:原文带回翻译页 */
function pickHistory(source: string) {
  t.source.value = source;
  tab.value = "translate";
}

function openDataDir() {
  api.openDataDir().catch((e) => toast.error(String(e)));
}

watch(tab, (v) => {
  if (v === "history") loadHistory();
});

onMounted(async () => {
  await Promise.all([load(), loadRuntime()]);

  // 弹卡/托盘把文本推给主窗口
  await listen<{ text: string }>("fill-source", (e) => {
    t.source.value = e.payload.text;
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
</script>

<template>
  <div class="h-screen">
    <Toaster theme="system" position="top-center" />

    <Tabs v-model="tab" class="flex h-full flex-row gap-0">
      <SideBar
        :active-profile="activeProfile"
        :tone-label="activeProfile ? toneMode(activeProfile.id) : ''"
        @open-settings="tab = 'settings'"
      />

      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <TabsContent value="translate" class="mt-0 flex min-h-0 flex-1 flex-col overflow-hidden">
          <TranslateView
            v-if="settings"
            :settings="settings"
            :active-profile="activeProfile"
            :active-ready="activeReady"
            :t="t"
            @settings-changed="persist(true)"
            @open-settings="tab = 'settings'"
          />
        </TabsContent>

        <TabsContent value="history" class="mt-0 flex min-h-0 flex-1 flex-col overflow-hidden">
          <HistoryView :items="historyItems" @clear="clearHistory" @pick="pickHistory" />
        </TabsContent>

        <TabsContent value="settings" class="mt-0 flex min-h-0 flex-1 flex-col overflow-hidden">
          <SettingsView
            v-if="settings"
            :settings="settings"
            :presets="presets"
            :runtime="runtime"
            :s="s"
            @save="persist()"
            @autosave="persist(true)"
            @open-data-dir="openDataDir"
          />
        </TabsContent>
      </div>
    </Tabs>

    <!-- 删除配置确认 -->
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
