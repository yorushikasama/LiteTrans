// 弹卡四角透明度检验。
//
// 关键在于**读 alpha**:把页面默认背景设成全透明后用 CDP 截图,PNG 会保留
// 每像素的 alpha。角上如果是 alpha=0,说明窗口那里是真正的空洞(桌面的
// 内容会透出来);如果 alpha>0,说明有东西把方形窗口的角填上了 ——
// 用户看到的"
// 四个角有东西"就是后者。
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { mkdtempSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { inflateSync } from "node:zlib";

const EDGE = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const OUT = join(tmpdir(), "lt-alpha");
mkdirSync(OUT, { recursive: true });
const profile = mkdtempSync(join(tmpdir(), "edge-al-"));
const PORT = 9471;
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
await send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });

await sleep(1200);

// 去掉调试角标,它固定在右下角会污染采样
await ev(`document.getElementById("lt-mock-badge")?.remove();`);

const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
if (!shot.data) throw new Error("截图失败");
const png = Buffer.from(shot.data, "base64");
writeFileSync(join(OUT, "popup-alpha.png"), png);

/** 极简 PNG 解码:只处理 8 位 RGBA / RGB,足够读角上像素 */
function decodePng(buf) {
  let pos = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString("ascii", pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    pos += 12 + len;
  }
  if (bitDepth !== 8 || (colorType !== 6 && colorType !== 2)) {
    throw new Error(`不支持的 PNG 格式 bitDepth=${bitDepth} colorType=${colorType}`);
  }
  const channels = colorType === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const out = Buffer.alloc(height * stride);
  let prev = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, y * (stride + 1) + 1 + stride);
    const cur = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? cur[x - channels] : 0;
      const b = prev[x];
      const c = x >= channels ? prev[x - channels] : 0;
      let v = line[x];
      switch (filter) {
        case 0: break;
        case 1: v += a; break;
        case 2: v += b; break;
        case 3: v += (a + b) >> 1; break;
        case 4: {
          const p = a + b - c;
          const pa = Math.abs(p - a);
          const pb = Math.abs(p - b);
          const pc = Math.abs(p - c);
          v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
          break;
        }
        default: throw new Error("未知 PNG filter " + filter);
      }
      cur[x] = v & 0xff;
    }
    cur.copy(out, y * stride);
    prev = cur;
  }
  return { width, height, channels, data: out };
}

const img = decodePng(png);
const px = (x, y) => {
  const i = (y * img.width + x) * img.channels;
  return { r: img.data[i], g: img.data[i + 1], b: img.data[i + 2], a: img.channels === 4 ? img.data[i + 3] : 255 };
};

const probe = (name, x, y) => {
  const p = px(x, y);
  return { corner: name, x, y, alpha: p.a };
};

const corners = [
  probe("左上", 0, 0),
  probe("右上", img.width - 1, 0),
  probe("左下", 0, img.height - 1),
  probe("右下", img.width - 1, img.height - 1),
  // 边中点:应当是不透明的卡片
  probe("上边中点", Math.floor(img.width / 2), 2),
  probe("中心", Math.floor(img.width / 2), Math.floor(img.height / 2)),
];

const opaqueCorners = corners.filter((c) => c.corner !== "上边中点" && c.corner !== "中心" && c.alpha > 8);
console.log(JSON.stringify({ out: OUT, size: `${img.width}x${img.height}`, corners, opaqueCorners }, null, 1));
console.log(
  opaqueCorners.length === 0
    ? "✓ 四角全部透明,窗口是干净的圆角"
    : `✗ ${opaqueCorners.length} 个角不透明,仍会看到方形残影`,
);

try {
  ws.close();
} catch {}
edge.kill();
process.exit(opaqueCorners.length === 0 ? 0 : 1);
