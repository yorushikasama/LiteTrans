// 弹卡页验收:420x330 下是否溢出、点击目标是否过小、背景是否透明
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { mkdtempSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const OUT = join(tmpdir(), "litetrans-popup");
mkdirSync(OUT, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "edge-pop-"));
const PORT = 9441;
const edge = spawn(
  EDGE,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--disable-gpu",
    "--hide-scrollbars",
    "--window-size=420,330",
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
  ws.onerror = () => rej(new Error("ws"));
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
const ev = async (e) => {
  const r = await send("Runtime.evaluate", { expression: e, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 300));
  return r.result.value;
};

await send("Page.enable");
await send("Runtime.enable");
const loaded = once("Page.loadEventFired");
await send("Page.navigate", { url: "http://[::1]:1420/popup.html" });
await loaded;
await send("Emulation.setDeviceMetricsOverride", { width: 420, height: 330, deviceScaleFactor: 1, mobile: false });
await sleep(1200);

const info = await ev(`(() => {
  const doc = document.documentElement;
  const card = document.querySelector(".lt-card");
  const b = card ? card.getBoundingClientRect() : null;
  const cs = (el) => el ? getComputedStyle(el) : null;
  const small = [];
  for (const el of document.querySelectorAll("button, [role=button], [role=combobox]")) {
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) continue;
    if (r.height < 22 || r.width < 22) small.push({ t: (el.innerText||el.title||"").trim().slice(0,12), w: Math.round(r.width), h: Math.round(r.height) });
  }
  return {
    viewport: innerWidth + "x" + innerHeight,
    overflowX: doc.scrollWidth - innerWidth,
    overflowY: doc.scrollHeight - innerHeight,
    htmlBg: cs(doc).backgroundColor,
    bodyBg: cs(document.body).backgroundColor,
    cardBg: cs(card) ? cs(card).backgroundColor : null,
    cardRect: b ? { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) } : null,
    smallTargets: small,
  };
})()`);
console.log(JSON.stringify(info, null, 1));
const s = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
if (s.data) writeFileSync(join(OUT, "popup.png"), Buffer.from(s.data, "base64"));
console.log("shot:", OUT);
try {
  ws.close();
} catch {}
edge.kill();
process.exit(0);
