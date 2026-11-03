/**
 * 五音:宫商角徵羽
 *
 * 每条模型配置是一个「调」(宫调/商调/角调/徵调/羽调),托盘菜单、设置列表、
 * 弹卡共用同一套标识。调由配置 id 派生,保证同一条配置在任何界面都是同一个调。
 *
 * 取模算法必须与 Rust 侧 `settings::tone_index` 完全一致:
 * 字符码位求和后对 5 取模。
 */

export const TONES = [
  { name: "宫", key: "gong", color: "var(--gong)", ink: "var(--shang-ink)" },
  { name: "商", key: "shang", color: "var(--shang)", ink: "var(--shang-ink)" },
  { name: "角", key: "jue", color: "var(--jue)", ink: "var(--jue-ink)" },
  { name: "徵", key: "zhi", color: "var(--zhi)", ink: "var(--zhi-ink)" },
  { name: "羽", key: "yu", color: "var(--yu)", ink: "var(--yu-ink)" },
] as const;

export type Tone = (typeof TONES)[number];

export function toneIndex(id: string): number {
  let sum = 0;
  for (let i = 0; i < id.length; i++) sum += id.charCodeAt(i);
  return sum % TONES.length;
}
export function toneOf(id: string | undefined): Tone {
  if (!id) return TONES[0];
  return TONES[toneIndex(id)];
}

/** 调式名,如「宫调」 */
export function toneMode(id: string | undefined): string {
  return `${toneOf(id).name}调`;
}
