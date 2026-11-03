<script setup lang="ts">
/** 模型配置区:档案列表 + 行内编辑器。设置页里最长的一段。 */
import { Loader2, Plus, Trash2 } from "@lucide/vue";
import { protocolShort, PROTOCOLS } from "@/types";
import type { Preset, Profile, Settings } from "@/types";
import type { useSettings } from "@/composables/useSettings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import ModelCombobox from "@/components/ModelCombobox.vue";
import ToneChip from "@/components/ToneChip.vue";

const props = defineProps<{
  settings: Settings;
  presets: Preset[];
  /** useSettings() 的返回值,宿主持有以跨页面保留编辑态 */
  s: ReturnType<typeof useSettings>;
}>();

const {
  st,
  ed,
  editing,
  testMsg,
  modelLoading,
  modelErr,
  editorEl,
  urlHint,
  modelOptions,
  applyPreset,
  newProfile,
  editProfile,
  saveProfile,
  askDelete,
  activateProfile,
  testProfile,
  fetchModels,
  setTemperature,
} = props.s;
</script>

<template>
  <section class="border-t pt-4">
    <div class="flex items-baseline gap-2">
      <h2 class="text-[13px] font-medium">模型配置</h2>
      <span class="lt-num text-[11px] text-muted-foreground">
        {{ settings.profiles.length }} 条
      </span>
    </div>
    <p class="mt-1 text-[11.5px] leading-5 text-muted-foreground">
      每条配置是一个「调」,随时切换;API Key 只存在 Windows 凭据管理器。
    </p>

    <div
      v-if="!settings.profiles.length && !editing"
      class="mt-3 rounded-lg border border-dashed px-4 py-6 text-center"
    >
      <div class="text-[13px]">还没有任何配置</div>
      <div class="mt-1 text-[11.5px] text-muted-foreground">
        选一个服务商预设,填入 API Key 就能开始
      </div>
      <Button size="sm" class="mt-3" @click="newProfile"><Plus />新增配置</Button>
    </div>

    <!-- -ml-2 让配置行的悬停底色向左外扩;右侧那 8px 会算进容器宽度
         造成横向溢出(实测 648>632),所以只外扩左侧 -->
    <div v-else class="mt-2 -ml-2">
      <!-- 配置行:窄屏时操作按钮折到下一行,否则固定宽度按钮组会把容器撑破 -->
      <div
        v-for="p in settings.profiles"
        :key="p.id"
        class="flex flex-wrap items-start gap-x-3 gap-y-1 rounded-md px-2 py-2.5 transition-colors"
        :class="p.id === settings.activeId && 'bg-gong/10'"
      >
        <ToneChip :id="p.id" :filled="p.id === settings.activeId" class="mt-0.5" />

        <div class="min-w-0 flex-1 basis-48">
          <div class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
            <span class="text-[13px] font-medium">{{ p.name }}</span>
            <span class="lt-num text-[10px] tracking-wider text-shang-ink uppercase">
              {{ protocolShort(p.protocol) }}
            </span>
            <span v-if="p.id === settings.activeId" class="lt-num text-[10px] text-gong-ink">
              · 使用中
            </span>
          </div>
          <!-- 模型与地址分两行各自截断:写在同一行里时,内层 inline 的 span
               会把父级的 scrollWidth 撑开,truncate 失效 -->
          <div class="lt-num truncate text-[11px] text-muted-foreground">
            {{ p.model || "未指定模型" }}
          </div>
          <div class="lt-num truncate text-[10px] text-muted-foreground/70">
            {{ p.baseUrl || "预设地址" }}
          </div>
          <div
            v-if="testMsg[p.id]"
            class="mt-1 text-[11px]"
            :class="testMsg[p.id].startsWith('连接正常') ? 'text-jue-ink' : 'text-destructive'"
          >
            {{ testMsg[p.id] }}
          </div>
        </div>

        <div class="flex shrink-0 items-center gap-0.5">
          <Button
            v-if="p.id !== settings.activeId"
            variant="ghost"
            size="sm"
            class="h-7 px-2 text-xs"
            @click="activateProfile(p.id)"
            >启用</Button
          >
          <Button variant="ghost" size="sm" class="h-7 px-2 text-xs" @click="editProfile(p)"
            >编辑</Button
          >
          <Button variant="ghost" size="sm" class="h-7 px-2 text-xs" @click="testProfile(p)"
            >测试</Button
          >
          <Button
            variant="ghost"
            size="icon-sm"
            class="text-muted-foreground hover:text-destructive"
            title="删除"
            @click="askDelete(p)"
          >
            <Trash2 />
          </Button>
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
        <span class="text-[13px] font-medium">编辑配置</span>
      </div>

      <label class="flex flex-col gap-1">
        <span class="lt-label">名称</span>
        <Input v-model="ed.name" placeholder="如 DeepSeek 官方 / 我的 OpenRouter" />
      </label>

      <div class="grid grid-cols-2 gap-3">
        <label class="flex flex-col gap-1">
          <span class="lt-label">服务商预设</span>
          <Select v-model="ed.provider" @update:model-value="applyPreset">
            <SelectTrigger class="w-full">
              <SelectValue placeholder="选择服务商" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="p in presets" :key="p.id" :value="p.id">
                {{ p.name }}
              </SelectItem>
            </SelectContent>
          </Select>
        </label>
        <label class="flex flex-col gap-1">
          <span class="lt-label">接口协议</span>
          <Select v-model="ed.protocol">
            <SelectTrigger class="w-full"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem v-for="x in PROTOCOLS" :key="x.value" :value="x.value">
                {{ x.label }}
              </SelectItem>
            </SelectContent>
          </Select>
        </label>
      </div>

      <label class="flex flex-col gap-1">
        <span class="lt-label">API 地址(Base URL)</span>
        <Input
          v-model="ed.baseUrl"
          class="lt-num text-[12.5px]"
          placeholder="https://api.example.com/v1"
        />
        <span v-if="urlHint" class="text-[11px] text-muted-foreground">{{ urlHint }}</span>
      </label>

      <div class="flex flex-col gap-1">
        <span class="lt-label">模型</span>
        <div class="flex items-start gap-2">
          <div class="min-w-0 flex-1">
            <ModelCombobox v-model="ed.model" :options="modelOptions" placeholder="deepseek-chat" />
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
        <span v-if="modelErr" class="text-[11px] text-destructive">{{ modelErr }}</span>
        <span v-else-if="modelOptions.length" class="text-[11px] text-muted-foreground">
          {{ modelOptions.length }} 个候选模型,可直接输入未列出的模型名
        </span>
      </div>

      <label v-if="ed.provider !== 'ollama'" class="flex flex-col gap-1">
        <span class="lt-label">API Key</span>
        <Input
          v-model="ed.apiKey"
          type="password"
          class="lt-num"
          placeholder="sk-…"
          autocomplete="off"
        />
      </label>

      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between gap-2">
          <span class="lt-label">Temperature</span>
          <span class="lt-num text-[11px] text-muted-foreground">
            {{ ed.temperature.toFixed(1) }} · 越低越稳定
          </span>
        </div>
        <Slider
          :model-value="[ed.temperature]"
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
  </section>
</template>
