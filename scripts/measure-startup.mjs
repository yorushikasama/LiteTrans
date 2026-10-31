// 测量 release 版启动耗时:通过 WebView2 远程调试端口读取页面 Navigation Timing
// 用法: 先以 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9223" 启动应用,
//       然后 node scripts/measure-startup.mjs [进程启动时刻的毫秒时间戳]
import { setTimeout as sleep } from "node:timers/promises";

const procStartTs = Number(process.argv[2] || 0);
const CDP = "http://127.0.0.1:9223";

let target = null;
const t0 = Date.now();
for (let i = 0; i < 60; i++) {
  try {
    const res = await fetch(`${CDP}/json`, { signal: AbortSignal.timeout(2000) });
    const list = await res.json();
    target = list.find((t) => t.type === "page" && /tauri|localhost|index\.html/i.test(t.url));
    if (target) break;
  } catch {
    /* webview 未就绪,继续等 */
  }
  await sleep(500);
}
if (!target) {
  console.error("CDP 目标未找到(应用未启动或调试端口未生效)");
  process.exit(1);
}
const cdpUpAt = Date.now();

// 连接 WebSocket
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error("ws 连接失败"));
});
let seq = 0;
const pending = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
};
function send(method, params = {}) {
  const id = ++seq;
  return new Promise((res) => {
    pending.set(id, (msg) => res(msg.result ?? msg.error ?? {}));
    ws.send(JSON.stringify({ id, method, params }));
  });
}

await send("Runtime.enable");
const nav = await send("Runtime.evaluate", {
  expression: `JSON.stringify((() => {
    const nav = performance.getEntriesByType("navigation")[0];
    const paint = performance.getEntriesByType("paint");
    const fcp = paint.find(p => p.name === "first-contentful-paint");
    return {
      url: location.href,
      readyState: document.readyState,
      fetchStart: Math.round(nav ? nav.fetchStart : 0),
      responseEnd: Math.round(nav ? nav.responseEnd : 0),
      domContentLoaded: Math.round(nav ? nav.domContentLoadedEventEnd : 0),
      loadEvent: Math.round(nav ? nav.loadEventEnd : 0),
      fcp: fcp ? Math.round(fcp.startTime) : null,
      transferSize: nav ? nav.transferSize : 0,
    };
})())`,
  returnByValue: true,
});
const t = JSON.parse(nav.result.value);
const rel = (v) => (v == null ? "—" : `${v}ms`);

console.log("=== 页面加载分解(相对页面 navigation start)===");
console.log(`页面地址       : ${t.url}`);
console.log(`HTML 响应完成  : ${rel(t.responseEnd)}  (内嵌资源应≈0)`);
console.log(`DOM 就绪       : ${rel(t.domContentLoaded)}`);
console.log(`首次内容绘制   : ${rel(t.fcp)}`);
console.log(`load 完成      : ${rel(t.loadEvent)}`);
console.log(`资源传输量     : ${t.transferSize} 字节`);

if (procStartTs) {
  const origin = await send("Runtime.evaluate", {
    expression: "String(performance.timeOrigin)",
    returnByValue: true,
  });
  const timeOrigin = Number(origin.result.value);
  console.log("=== 端到端(相对进程启动)===");
  console.log(`进程启动 → 导航开始 : ${timeOrigin - procStartTs}ms  (进程+WebView2 初始化)`);
  console.log(`进程启动 → load 完成 : ${Math.round(timeOrigin + (t.loadEvent || 0) - procStartTs)}ms  (可交互)`);
}
process.exit(0);
