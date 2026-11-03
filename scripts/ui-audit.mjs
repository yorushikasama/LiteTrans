// UI 布局审计:无头 Edge + CDP,在 4 个窗口尺寸下真实点击三个页面,
// 报告横向/纵向溢出、过小点击目标(<24px)、被截断文本,并逐页截图。
// 需要 dev server 已启动: npm run dev (127.0.0.1 上没监听,用 [::1])
// 用法: node scripts/ui-audit.mjs [url]
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { mkdtempSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const URL_TARGET = process.argv[2] || "http://[::1]:1420/";
const EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const OUT = join(tmpdir(), "litetrans-uiaudit");
mkdirSync(OUT, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "edge-audit-"));
const PORT = 9412;

const edge = spawn(
  EDGE,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-gpu",
    "--hide-scrollbars",
    "--window-size=1280,800",
    "about:blank",
  ],
  { stdio: "ignore" },
);

let ready = false;
for (let i = 0; i < 80; i++) {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/json/version`, { signal: AbortSignal.timeout(1500) });
    if ((await r.json()).webSocketDebuggerUrl) {
      ready = true;
      break;
    }
  } catch {}
  await sleep(250);
}
if (!ready) {
  console.error("无头 Edge 未就绪");
  process.exit(1);
}

const pageInfo = await (await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, { method: "PUT" })).json();
const ws = new WebSocket(pageInfo.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error("ws 连接失败"));
});
let seq = 0;
const pending = new Map();
const listeners = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  } else if (msg.method && listeners.has(msg.method)) {
    listeners.get(msg.method)(msg.params);
  }
};
const send = (method, params = {}) =>
  new Promise((res) => {
    const id = ++seq;
    pending.set(id, (m) => res(m.result ?? m.error ?? {}));
    ws.send(JSON.stringify({ id, method, params }));
  });
const once = (method) =>
  new Promise((res) => {
    listeners.set(method, (p) => {
      listeners.delete(method);
      res(p);
    });
  });
async function ev(expression) {
  const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
  return r.result.value;
}
async function realClick(x, y) {
  await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none", buttons: 0 });
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 });
  await sleep(120);
}
async function screenshot(name) {
  const s = await send("Page.captureScreenshot", { format: "png" });
  if (s.data) writeFileSync(join(OUT, `${name}.png`), Buffer.from(s.data, "base64"));
}

await send("Page.enable");
await send("Runtime.enable");
const loaded = once("Page.loadEventFired");
await send("Page.navigate", { url: URL_TARGET });
await loaded;
for (let i = 0; i < 60; i++) {
  if (await ev(`!!document.querySelector('aside')`)) break;
  await sleep(150);
}
await sleep(700);

const METRICS = String.raw`(() => {
  const doc = document.documentElement;
  const r = (el) => { if (!el) return null; const b = el.getBoundingClientRect();
    return { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) }; };
  const t = (el) => el ? (el.innerText||"").replace(/\s+/g," ").trim().slice(0,50) : null;
  const active = document.querySelector('[role="tab"][aria-selected="true"]');
  const panel = document.querySelector('[role="tabpanel"]:not([hidden])');
  const out = {
    viewport: innerWidth + "x" + innerHeight,
    overflowX: doc.scrollWidth - innerWidth,
    overflowY: doc.scrollHeight - innerHeight,
    activeTab: t(active),
    panel: r(panel),
    title: document.title,
  };
  // 面板内所有直接子块(用于判断纵向节奏)
  if (panel) {
    out.panelChildren = [...panel.children].map((c) => ({
      tag: c.tagName.toLowerCase(),
      cls: (c.className||"").toString().slice(0,48),
      ...r(c),
    }));
  }
  // 点击目标过小
  const small = [];
  for (const el of document.querySelectorAll('button, a, [role="tab"], [role="switch"], input, select')) {
    const b = el.getBoundingClientRect();
    if (!b.width || !b.height) continue;
    if (b.height < 24 || b.width < 24) small.push({ tag: el.tagName.toLowerCase(), text: t(el), w: Math.round(b.width), h: Math.round(b.height) });
  }
  out.smallTargets = small.slice(0, 15);
  // 溢出
  const over = [];
  for (const el of document.querySelectorAll("*")) {
    if (el.clientWidth > 0 && el.scrollWidth - el.clientWidth > 1) {
      const cs = getComputedStyle(el);
      if (cs.overflowX === "visible" || cs.overflowX === "clip")
        over.push({ tag: el.tagName.toLowerCase(), cls: (el.className||"").toString().slice(0,40), sw: el.scrollWidth, cw: el.clientWidth });
    }
    if (over.length > 10) break;
  }
  out.overflowEls = over;
  // 文本被截断(可能丢信息)
  const clipped = [];
  for (const el of document.querySelectorAll("span, div, p")) {
    const b = el.getBoundingClientRect();
    if (b.width < 1 || b.height < 1) continue;
    if (el.children.length) continue;
    const cs = getComputedStyle(el);
    if (cs.textOverflow !== "ellipsis" && cs.overflow !== "hidden") continue;
    if (el.scrollWidth - el.clientWidth > 2)
      clipped.push({ text: (el.innerText||"").trim().slice(0,40), cw: el.clientWidth, sw: el.scrollWidth });
  }
  out.clippedText = clipped.slice(0, 10);
  return out;
})()`;

const tabCenter = async (label) =>
  ev(`(() => {
    const t = [...document.querySelectorAll('[role="tab"]')].find(e => e.innerText.trim() === ${JSON.stringify(label)});
    if (!t) return null;
    const b = t.getBoundingClientRect();
    return { x: Math.round(b.x + b.width/2), y: Math.round(b.y + b.height/2) };
  })()`);

const report = {};
// 640x500 是 tauri.conf.json 声明的最小窗口;但 800x600 在 150% 缩放下
// 只有 533x400 个 CSS 像素(窗口尺寸被当作物理像素),所以必须把这个
// 真实生效尺寸也纳入审计 —— 它比"最小尺寸"还窄,是最紧的一档。
const VIEWPORTS = [
  { w: 533, h: 400 }, // 800x600 窗口 @150% 缩放
  { w: 640, h: 500 },
  { w: 800, h: 600 },
  { w: 1024, h: 700 },
  { w: 1280, h: 800 },
];

for (const page of ["翻译", "历史", "设置"]) {
  report[page] = [];
  for (const vp of VIEWPORTS) {
    await send("Emulation.setDeviceMetricsOverride", {
      width: vp.w, height: vp.h, deviceScaleFactor: 1, mobile: false,
    });
    await sleep(250);
    const c = await tabCenter(page);
    if (c) await realClick(c.x, c.y);
    await sleep(350);
    const m = await ev(METRICS);
    report[page].push({ vp: `${vp.w}x${vp.h}`, ...m });
    await screenshot(`${page}-${vp.w}x${vp.h}`);
  }
}

// 深色模式抽查(设置页 1024)
await send("Emulation.setDeviceMetricsOverride", { width: 1024, height: 700, deviceScaleFactor: 1, mobile: false });
const darkTab = await tabCenter("设置");
if (darkTab) await realClick(darkTab.x, darkTab.y);
await sleep(250);
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
await sleep(400);
report.dark = await ev(METRICS);
await screenshot("dark-设置-1024x700");

console.log(JSON.stringify({ out: OUT, report }, null, 1));

try { ws.close(); } catch {}
edge.kill();
process.exit(0);
