<script setup lang="ts">
/** 关于区:版本、便携/安装、数据目录位置。 */
import type { RuntimeInfo } from "@/types";
import { Button } from "@/components/ui/button";

defineProps<{ runtime: RuntimeInfo }>();
const emit = defineEmits<{ (e: "open-data-dir"): void }>();
</script>

<template>
  <section class="mt-8 border-t pt-4">
    <h2 class="text-[13px] font-medium">关于</h2>
    <div class="mt-1 divide-y divide-border/60">
      <div class="flex items-center justify-between gap-4 py-2.5">
        <div class="min-w-0">
          <div class="flex items-center gap-2 text-[13px]">
            <span
              class="rounded border px-1.5 text-[10px] leading-4"
              :class="
                runtime.portable
                  ? 'border-jue-ink/50 text-jue-ink'
                  : 'border-border text-muted-foreground'
              "
              >{{ runtime.portable ? "便携版" : "安装版" }}</span
            >
            <span class="lt-num text-muted-foreground">v{{ runtime.version }}</span>
          </div>
          <div class="mt-0.5 text-[11.5px] text-muted-foreground">
            {{
              runtime.portable
                ? "配置与历史都存在程序目录的 data/,整个文件夹拷走即可带走设置"
                : "配置与历史存在系统用户目录;如需便携使用,下载便携版"
            }}
          </div>
        </div>
        <Button variant="outline" size="sm" class="shrink-0" @click="emit('open-data-dir')"
          >打开数据目录</Button
        >
      </div>
      <div class="py-2.5">
        <div class="lt-label">数据位置</div>
        <div class="lt-num mt-0.5 text-[11px] break-all text-muted-foreground">
          {{ runtime.dataDir || "(未知)" }}
        </div>
        <div class="mt-1 text-[11px] text-muted-foreground/80">
          API Key 不在此目录,单独存放在 Windows 凭据管理器
        </div>
      </div>
    </div>
  </section>
</template>
