// 生成 README 用的界面截图。
//
// 用无头 Edge + CDP 渲染真实页面(DEV 模式自带后端 mock),按 2x 缩放截图
// 以保证在高分屏和 GitHub 上都不糊。产物落在 docs/screenshots/。
//
// 深色截图走设置页里真实的三态开关,不是用 CDP 伪造 prefers-color-scheme ——
// README 要展示的是用户真能看到的那个功能。
//
// 用法: 先 npm run dev,再 npm run shots
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const URL_BASE = process.argv[2] || "http://[::1]:1420";
const EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const OUT = join(process.cwd(), "docs", "screenshots");
mkdirSync(OUT, { recursive: true });

const profile = mkdtempSync(join(tmpdir(), "edge-shots-"));
const PORT = 9481;
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
    "--force-color-profile=srgb",
    "--window-size=1200,760",
    "about:blank",
  ],
  { stdio: "ignore" },
);

for (let i = 0; i < 80; i++) {
  try {
    const r = await fetch(`http://127.0.0.1:${PORT}/json/version`, { signal: AbortSignal.timeout(1500) });
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
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pend.has(m.id)) {
    pend.get(m.id)(m);
    pend.delete(m.id);
  }
};
const send = (method, params = {}) =>
  new Promise((r) => {
    const id = ++seq;
    pend.set(id, (m) => r(m.result ?? m.error ?? {}));
    ws.send(JSON.stringify({ id, method, params }));
  });
const ev = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
  return r.result.value;
};

/** 真实鼠标点击:reka-ui 的 Tabs 监听 pointerdown,element.click() 切不动页 */
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

const clickTab = (label) =>
  realClick(`[...document.querySelectorAll('[role="tab"]')].find(e => e.innerText.trim() === ${JSON.stringify(label)})`);

/**
 * 按文字点按钮,但排除 role="tab"。
 * 侧栏导航项也是 button(内容 「翻译」),不排除的话点"翻译"总会先命中导航项,
 * 结果是一直停在翻译页、从没真正发起过翻译。
 */
const clickButton = (label) =>
  realClick(
    `[...document.querySelectorAll("button")].filter(e => e.getAttribute("role") !== "tab").find(e => e.innerText.trim() === ${JSON.stringify(label)})`,
  );

async function goto(url) {
  // 不用 Page.loadEventFired 等:导航若很快完成(缓存/同页刷新)事件可能
  // 在登记监听之前就派发完,脚本会一直挂住。轮询 readyState 更稳。
  await send("Page.navigate", { url });
  for (let i = 0; i < 200; i++) {
    const ready = await ev(`document.readyState`).catch(() => "");
    if (ready === "complete") break;
    await sleep(100);
  }
  for (let i = 0; i < 80; i++) {
    if (await ev(`!!document.querySelector("aside") || !!document.querySelector(".lt-card")`)) break;
    await sleep(150);
  }
  await sleep(900);
  // 去掉调试角标:README 里不该出现"浏览器调试"字样
  await ev(`document.getElementById("lt-mock-badge")?.remove();`);
}

/** 在已加载的主窗口里切到设置页,把界面主题设成 label 指定的那一档 */
async function setTheme(label) {
  await clickTab("设置");
  await sleep(500);
  const ok = await clickButton(label);
  await sleep(500);
  // 切回翻译页,截图里不出现设置页(除了专门的设置截图)
  await clickTab("翻译");
  await sleep(400);
  return ok;
}
async function shot(name, width = 1200, height = 760) {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 2,
    mobile: false,
  });
  await sleep(300);
  await ev(`document.getElementById("lt-mock-badge")?.remove();`);
  const s = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  writeFileSync(join(OUT, name), Buffer.from(s.data, "base64"));
  console.log("  ✓", name);
}

await send("Page.enable");
await send("Runtime.enable");
await send("Emulation.setDeviceMetricsOverride", { width: 1200, height: 760, deviceScaleFactor: 2, mobile: false });
// 系统偏好固定成浅色:截图的明暗只由开关决定,不受截图机当前设置影响
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "light" }] });

console.log("生成截图 →", OUT);

// 清掉浏览器调试态留在 localStorage 里的设置:上一次运行改过的目标语言
// 会跟着下一次进来,截图就自相矛盾了(顶部选"英语"、读数条却是"→ 中文")。
await send("Page.navigate", { url: `${URL_BASE}/` });
await sleep(600);
await ev(`(() => { try { localStorage.removeItem("litetrans.mock.settings"); } catch {} })()`);

/* ── 浅色 ── */
await goto(`${URL_BASE}/`);
await setTheme("浅色");
await goto(`${URL_BASE}/`);
await shot("main-translate.png");

// 历史页
await goto(`${URL_BASE}/`);
await clickTab("历史");
await sleep(700);
await shot("main-history.png");

// 设置页(模型配置)
await goto(`${URL_BASE}/`);
await clickTab("设置");
await sleep(700);
await shot("main-settings.png");

// 设置页(通用设置:主题三态开关在这里)
await goto(`${URL_BASE}/`);
await clickTab("设置");
await sleep(600);
await ev(`(() => {
  const btn = [...document.querySelectorAll("button")].find(e => e.innerText.trim() === "浅色");
  btn?.scrollIntoView({ block: "center" });
})()`);
await sleep(400);
await shot("main-settings-general.png");

/* ── 深色(走真实开关) ── */
await goto(`${URL_BASE}/`);
await setTheme("深色");
await goto(`${URL_BASE}/`);
await shot("main-translate-dark.png");

/* ── 弹卡 ── */
await goto(`${URL_BASE}/popup.html`);
await shot("popup-dark.png", 420, 330);

await goto(`${URL_BASE}/`);
await setTheme("浅色");
await goto(`${URL_BASE}/popup.html`);
await shot("popup.png", 420, 330);

console.log("完成。");
try {
  ws.close();
} catch {}
edge.kill();
setTimeout(() => process.exit(0), 300);
