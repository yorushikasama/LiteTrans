/**
 * 设置页状态:全局设置、模型配置档案的增删改查、连通性测试与模型拉取。
 *
 * 从 App.vue 抽出。这块逻辑的耦合点是「档案列表」和「正在编辑的副本」:
 * 编辑必须操作副本,保存时才合并回 settings.profiles,否则用户点开编辑
 * 随手改一个字、再点取消,原配置就被污染了。
 */
import { computed, ref } from "vue";
import { toast } from "vue-sonner";
import { api } from "@/api";
import { genId, PROTOCOLS } from "@/types";
import type { Preset, Profile, Settings } from "@/types";

export function useSettings() {
  const settings = ref<Settings | null>(null);
  const presets = ref<Preset[]>([]);
  const saveErr = ref("");

  /**
   * 模板里 v-model 的写入目标。
   *
   * 设置页整体在 v-if="settings" 之内,读表达式能正常收窄;但 v-model 会生成
   * 一句赋值,vue-tsc 不会把收窄带进赋值目标,于是每个 v-model="settings.xxx"
   * 都报「'settings' is possibly 'null'」。这里给一个非空视图专门用于写入,
   * 运行时由 v-if 保证非空。
   */
  const st = computed(() => settings.value as Settings);

  /** 正在编辑的档案副本;null = 编辑器关闭 */
  const editing = ref<Profile | null>(null);
  /** 同上:编辑器整体在 v-if="editing" 之内 */
  const ed = computed(() => editing.value as Profile);

  const testMsg = ref<Record<string, string>>({});
  const fetchedModels = ref<string[]>([]);
  const modelLoading = ref(false);
  const modelErr = ref("");
  const editorEl = ref<HTMLElement | null>(null);

  const confirmDelete = ref(false);
  const deleteTarget = ref<Profile | null>(null);

  const activeProfile = computed<Profile | null>(() => {
    const s = settings.value;
    if (!s) return null;
    return s.profiles.find((p) => p.id === s.activeId) ?? s.profiles[0] ?? null;
  });

  /** 当前档案是否具备翻译条件(ollama 本地模型不需要 Key) */
  const activeReady = computed(() => {
    const p = activeProfile.value;
    if (!p) return false;
    return p.provider === "ollama" || !!p.apiKey.trim();
  });

  const urlHint = computed(
    () => PROTOCOLS.find((x) => x.value === editing.value?.protocol)?.hint ?? "",
  );

  const modelOptions = computed(() => {
    const preset = presets.value.find((x) => x.id === editing.value?.provider);
    return Array.from(new Set([...(preset?.models ?? []), ...fetchedModels.value]));
  });

  async function load() {
    const [s, p] = await Promise.all([api.getSettings(), api.getPresets()]);
    settings.value = s;
    presets.value = p;
  }

  async function persist(silent = false) {
    saveErr.value = "";
    try {
      await api.saveSettings(settings.value as Settings);
      if (!silent) toast.success("已保存");
      return true;
    } catch (e) {
      saveErr.value = String(e);
      toast.error(String(e));
      return false;
    }
  }

  function scrollToEditor() {
    editorEl.value?.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  /** 应用服务商预设(填地址/模型/协议) */
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
      testMsg.value[p.id] = await api.testProfile(p);
    } catch (e) {
      testMsg.value[p.id] = String(e);
    }
  }

  async function fetchModels() {
    if (!editing.value) return;
    modelLoading.value = true;
    modelErr.value = "";
    try {
      fetchedModels.value = await api.fetchModels(editing.value);
      if (!fetchedModels.value.length) modelErr.value = "接口未返回模型列表";
    } catch (e) {
      modelErr.value = String(e);
    } finally {
      modelLoading.value = false;
    }
  }

  /** Slider 的 update 回调签名是 `number[] | undefined`,这里显式对齐 */
  function setTemperature(v: number[] | undefined) {
    if (editing.value && v?.length) editing.value.temperature = v[0];
  }

  return {
    settings,
    presets,
    saveErr,
    st,
    ed,
    editing,
    testMsg,
    fetchedModels,
    modelLoading,
    modelErr,
    editorEl,
    confirmDelete,
    deleteTarget,
    activeProfile,
    activeReady,
    urlHint,
    modelOptions,
    load,
    persist,
    applyPreset,
    newProfile,
    editProfile,
    saveProfile,
    askDelete,
    confirmDeleteNow,
    activateProfile,
    testProfile,
    fetchModels,
    setTemperature,
  };
}
