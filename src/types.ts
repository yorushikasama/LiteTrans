export interface StreamMsg {
  type: "start" | "delta" | "done" | "error" | "cancelled";
  detected?: string;
  target?: string;
  dict?: boolean;
  text?: string;
  message?: string;
}

export interface Preset {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  needKey: boolean;
  protocol: string;
  models: string[];
}

export type Protocol = "openai" | "anthropic" | "gemini";

export interface Profile {
  id: string;
  name: string;
  provider: string;
  protocol: string;
  baseUrl: string;
  model: string;
  temperature: number;
  apiKey: string;
}

export const PROTOCOLS = [
  { value: "openai", label: "OpenAI 兼容(/chat/completions)", hint: "API 地址通常以 /v1 结尾,如 https://api.deepseek.com/v1" },
  { value: "anthropic", label: "Anthropic 兼容(/v1/messages)", hint: "API 地址填主机根,如 https://api.anthropic.com" },
  { value: "gemini", label: "Gemini 兼容(generateContent)", hint: "API 地址填主机根,如 https://generativelanguage.googleapis.com" },
] as const;

export function protocolShort(v: string): string {
  return v === "anthropic" ? "Anthropic" : v === "gemini" ? "Gemini" : "OpenAI";
}

export interface Settings {
  profiles: Profile[];
  activeId: string;
  /** auto = 智能模式(自动识别→中文);custom = 自定义方向 */
  transMode: string;
  /** custom 模式源语言,"auto" 表示仍自动检测 */
  sourceLang: string;
  targetLang: string;
  hotkey: string;
  customPrompt: string;
  autostart: boolean;
  /** 划词弹卡完成后自动复制译文 */
  autoCopy: boolean;
  setupDone: boolean;
}

export function genId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `p${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * 目标语言表:value 直接作为提示词里的语言名(English name),
 * 后端透传,新增语言只需在此追加一行。
 */
export const LANGUAGES = [
  { value: "Simplified Chinese", label: "中文(简体)" },
  { value: "Traditional Chinese", label: "中文(繁體)" },
  { value: "English", label: "英语" },
  { value: "Japanese", label: "日语" },
  { value: "Korean", label: "韩语" },
  { value: "French", label: "法语" },
  { value: "German", label: "德语" },
  { value: "Spanish", label: "西班牙语" },
  { value: "Portuguese", label: "葡萄牙语" },
  { value: "Italian", label: "意大利语" },
  { value: "Russian", label: "俄语" },
  { value: "Arabic", label: "阿拉伯语" },
  { value: "Hindi", label: "印地语" },
  { value: "Thai", label: "泰语" },
  { value: "Vietnamese", label: "越南语" },
  { value: "Indonesian", label: "印尼语" },
  { value: "Malay", label: "马来语" },
  { value: "Filipino", label: "菲律宾语" },
  { value: "Turkish", label: "土耳其语" },
  { value: "Persian", label: "波斯语" },
  { value: "Hebrew", label: "希伯来语" },
  { value: "Greek", label: "希腊语" },
  { value: "Dutch", label: "荷兰语" },
  { value: "Polish", label: "波兰语" },
  { value: "Ukrainian", label: "乌克兰语" },
  { value: "Czech", label: "捷克语" },
  { value: "Hungarian", label: "匈牙利语" },
  { value: "Romanian", label: "罗马尼亚语" },
  { value: "Bulgarian", label: "保加利亚语" },
  { value: "Swedish", label: "瑞典语" },
  { value: "Norwegian", label: "挪威语" },
  { value: "Danish", label: "丹麦语" },
  { value: "Finnish", label: "芬兰语" },
  { value: "Mongolian", label: "蒙古语" },
  { value: "Kazakh", label: "哈萨克语" },
  { value: "Swahili", label: "斯瓦希里语" },
  { value: "Burmese", label: "缅甸语" },
  { value: "Khmer", label: "高棉语" },
  { value: "Lao", label: "老挝语" },
  { value: "Bengali", label: "孟加拉语" },
  { value: "Urdu", label: "乌尔都语" },
  { value: "Nepali", label: "尼泊尔语" },
  { value: "Sinhala", label: "僧伽罗语" },
  { value: "Latin", label: "拉丁语" },
  { value: "Esperanto", label: "世界语" },
] as const;

/** 自定义方向的源语言选项(含自动检测);须声明在 LANGUAGES 之后 */
export const SOURCE_OPTIONS = [
  { value: "auto", label: "自动检测" },
  ...LANGUAGES.map((l) => ({ value: l.value, label: l.label })),
];

export function langLabel(v: string): string {
  return LANGUAGES.find((l) => l.value === v)?.label ?? v;
}
