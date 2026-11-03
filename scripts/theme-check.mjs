// 深色/浅色三态实测。
//
// 验证"设置里的 theme 真的能改界面",而不只是 CSS 里写了变量。
// 做法:直接驱动页面里的设置对象(等价于点设置页的按钮),
// 读 html[data-theme] 与 body 的实际计算背景色。
//
// 用法: npm run dev 之后 node scripts/theme-check.mjs
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const URL_BASE = process.argv[2] || "http://[::1]:1420";
const EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const OUT = join(tmpdir(), "lt-theme-check");
mkdirSync(OUT, { recursive: true });

const profile = mkdtempSync(join(tmpdir(), "edge-theme-"));
const PORT = 9493;
const edge = spawn(
  EDGE,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--disable-gpu",
    "--hide-scrollbars",
    "--force-color-profile=srgb",
    "about:blank",
  ],
  { stdio: "ignore" },
);

for (let i = 0; i < 80; i++) {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/json/version`, {
      signal: AbortSignal.timeout(1500),
    });
    if ((await r.json()).webSocketDebuggerUrl) break;
  } catch {}
  await sleep(250);
}
const pi = await (await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, { method: "PUT" })).json();
const ws = new WebSocket(pi.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error("ws 连接失败"));
});
let seq = 0;
const pend = new Map();
const ls = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pend.has(m.id)) {
    pend.get(m.id)(m);
    pend.delete(m.id);
  } else if (m.method && ls.has(m.method)) ls.get(m.method)(m.params);
};
const send = (method, params = {}) =>
  new Promise((r) => {
    const id = ++seq;
    pend.set(id, (m) => r(m.result ?? m.error ?? {}));
    ws.send(JSON.stringify({ id, method, params }));
  });
const once = (m) =>
  new Promise((r) =>
    ls.set(m, (p) => {
      ls.delete(m);
      r(p);
    }),
  );
const ev = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
  return r.result.value;
};

await send("Page.enable");
await send("Runtime.enable");
await send("Emulation.setDeviceMetricsOverride", { width: 1200, height: 760, deviceScaleFactor: 2, mobile: false });

/** 系统偏好设为指定值,然后重新加载 —— 模拟"用户系统是深色/浅色" */
async function loadAs(systemTheme) {
  await send("Emulation.setEmulatedMedia", {
    features: [{ name: "prefers-color-scheme", value: systemTheme }],
  });
  const loaded = once("Page.loadEventFired");
  await send("Page.navigate", { url: `${URL_BASE}/` });
  await loaded;
  for (let i = 0; i < 60; i++) {
    if (await ev(`!!document.querySelector("aside")`)) break;
    await sleep(150);
  }
  await sleep(700);
}

/** 在元素中心派发一次真实鼠标点击。
    reka-ui 的 Tabs 监听 pointerdown,`element.click()` 不会切页 —— 必须走 CDP。 */
async function realClick(selectorExpr) {
  const box = await ev(`(() => {
    const el = ${selectorExpr};
    if (!el) return null;
    el.scrollIntoView({ block: "center" });
    const r = el.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  })()`);
  if (!box) return false;
  for (const type of ["mousePressed", "mouseReleased"]) {
    await send("Input.dispatchMouseEvent", {
      type,
      x: Math.round(box.x),
      y: Math.round(box.y),
      button: "left",
      clickCount: 1,
    });
    await sleep(40);
  }
  return true;
}

/** 点设置页里那个三态按钮(真实点击,验证 UI 接线而不只是数据) */
async function clickTheme(label) {
  const tabOk = await realClick(
    `[...document.querySelectorAll('[role="tab"]')].find(e => e.innerText.trim() === "设置")`,
  );
  if (!tabOk) return "找不到设置页签";
  await sleep(600);
  const ok = await realClick(
    `[...document.querySelectorAll("button")].find(e => e.innerText.trim() === ${JSON.stringify(label)})`,
  );
  await sleep(500);
  return ok ? "clicked" : `找不到按钮:${label}`;
}

async function probe() {
  return ev(`(() => {
    // getComputedStyle 返回的是 oklch(...),字符串比对看不出深浅。
    // 画到 canvas 上取一个像素,拿到真正的 sRGB,再算相对亮度。
    const cv = document.createElement("canvas");
    cv.width = cv.height = 1;
    const ctx = cv.getContext("2d", { willReadFrequently: true });
    const toRgb = (color) => {
      ctx.clearRect(0, 0, 1, 1);
      ctx.fillStyle = "#000";
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
      return { r, g, b, a };
    };
    const rgb = getComputedStyle(document.body).backgroundColor;
    const px = toRgb(rgb);
    const btn = [...document.querySelectorAll("button")].find(
      (e) => ["跟随系统", "浅色", "深色"].includes(e.innerText.trim()) && e.getAttribute("aria-pressed") === "true",
    );
    return {
      dataTheme: document.documentElement.dataset.theme ?? null,
      bodyBg: rgb,
      srgb: px,
      // 相对亮度 0(黑)~255(白),用来判断"看起来是不是深色"
      luminance: Math.round(px.r * 0.299 + px.g * 0.587 + px.b * 0.114),
      mediaDark: matchMedia("(prefers-color-scheme: dark)").matches,
      activeButton: btn ? btn.innerText.trim() : null,
    };
  })()`);
}

const results = [];
const record = async (label, expect) => {
  const got = await probe();
  results.push({ case: label, expect, ...got });
  return got;
};

// ── 1) 系统深色 + 跟随系统 → 应为深色 ──
await loadAs("dark");
await record("系统深色 / 默认(system)", "dark");

// ── 2) 强制浅色,即使系统是深色 ──
console.log(await clickTheme("浅色"));
await record("系统深色 / 用户选 light", "light");
const s2 = await send("Page.captureScreenshot", { format: "png" });
writeFileSync(join(OUT, "forced-light-on-dark-system.png"), Buffer.from(s2.data, "base64"));

// ── 3) 强制深色 ──
console.log(await clickTheme("深色"));
await record("系统深色 / 用户选 dark", "dark");
const s3 = await send("Page.captureScreenshot", { format: "png" });
writeFileSync(join(OUT, "forced-dark.png"), Buffer.from(s3.data, "base64"));

// ── 4) 系统浅色 + 强制深色(最容易漏的一档)──
await loadAs("light");
await clickTheme("深色");
await record("系统浅色 / 用户选 dark", "dark");

// ── 5) 系统浅色 + 跟随系统 → 浅色 ──
await loadAs("light");
await clickTheme("跟随系统");
await record("系统浅色 / 用户选 system", "light");

// ── 6) 弹卡也要跟着变 ──
await ev(`document.title`);
const popupLoaded = once("Page.loadEventFired");
await send("Page.navigate", { url: `${URL_BASE}/popup.html` });
await popupLoaded;
await sleep(1200);
const popup = await ev(`(() => {
  const card = document.querySelector(".lt-card");
  return {
    dataTheme: document.documentElement.dataset.theme ?? null,
    cardBg: card ? getComputedStyle(card).backgroundColor : null,
    bodyBg: getComputedStyle(document.body).backgroundColor,
    bodyTransparent: getComputedStyle(document.body).backgroundColor === "rgba(0, 0, 0, 0)",
  };
})()`);
const s4 = await send("Page.captureScreenshot", { format: "png" });
writeFileSync(join(OUT, "popup-dark.png"), Buffer.from(s4.data, "base64"));

console.log(JSON.stringify({ out: OUT, results, popup }, null, 1));

const bad = results.filter((r) => r.dataTheme !== r.expect);
// 属性对了但颜色没跟着变(变量写错/被覆盖)也算失败,所以再验一次实测亮度
const wrongColor = results.filter((r) =>
  r.expect === "dark" ? r.luminance > 80 : r.luminance < 180,
);
if (bad.length || wrongColor.length) {
  if (bad.length) console.error("✗ 主题属性不符合预期:", bad.map((b) => `${b.case} 期望 ${b.expect} 实际 ${b.dataTheme}`));
  if (wrongColor.length)
    console.error(
      "✗ 属性对了但实际颜色不对:",
      wrongColor.map((b) => `${b.case} 期望 ${b.expect} 实测亮度 ${b.luminance}`),
    );
  process.exitCode = 1;
} else {
  const d = results.find((r) => r.expect === "dark");
  const l = results.find((r) => r.expect === "light");
  console.log(`✓ 三态全部符合预期(深色实测 rgb ${d.srgb.r},${d.srgb.g},${d.srgb.b} / 浅色 ${l.srgb.r},${l.srgb.g},${l.srgb.b})`);
}
if (!popup.bodyTransparent) {
  console.error("✗ 弹卡 body 不是透明底:", popup.bodyBg);
  process.exitCode = 1;
}

try {
  ws.close();
} catch {}
edge.kill();
setTimeout(() => process.exit(process.exitCode ?? 0), 300);
