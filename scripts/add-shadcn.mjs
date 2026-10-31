// 从 shadcn-vue 注册表拉取组件源码(带重试,替代官方 CLI 的弱超时)
// 用法: node scripts/add-shadcn.mjs button input select ...
import { writeFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

const STYLE = "new-york-v4";
const BASE = `https://shadcn-vue.com/r/styles/${STYLE}/`;
const requested = process.argv.slice(2);
if (!requested.length) {
  console.error("usage: node scripts/add-shadcn.mjs <component...>");
  process.exit(1);
}

const done = new Set();
const queue = [...requested];
const deps = new Set();

async function fetchItem(name) {
  let lastErr;
  for (let t = 1; t <= 6; t++) {
    try {
      const res = await fetch(BASE + name + ".json", { signal: AbortSignal.timeout(45000) });
      if (!res.ok) throw new Error("HTTP " + res.status);
      return await res.json();
    } catch (e) {
      lastErr = e;
      console.log(`  retry ${name} #${t}: ${e.message}`);
      await new Promise((r) => setTimeout(r, 1200 * t));
    }
  }
  throw new Error(`fetch ${name} failed: ${lastErr && lastErr.message}`);
}

while (queue.length) {
  const name = queue.shift();
  if (done.has(name)) continue;
  done.add(name);
  process.stdout.write(`fetch ${name} ... `);
  const json = await fetchItem(name);
  console.log("ok");
  for (const f of json.files || []) {
    const rel = (f.target || f.path || "").replace(/\\/g, "/");
    if (!rel) continue;
    const abs = join(process.cwd(), rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, f.content, "utf8");
    console.log(`  wrote ${rel}`);
  }
  for (const d of json.dependencies || []) deps.add(d);
  for (const d of json.registryDependencies || []) {
    if (!done.has(d)) queue.push(d);
  }
}

// 合并写入 package.json
if (deps.size) {
  const pkgPath = join(process.cwd(), "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  pkg.dependencies = pkg.dependencies || {};
  let added = [];
  for (const d of deps) {
    if (!pkg.dependencies[d] && !existsSync(join(process.cwd(), "node_modules", d))) {
      pkg.dependencies[d] = "latest";
      added.push(d);
    } else if (!pkg.dependencies[d]) {
      // 已在 node_modules(之前手动装过),仍补记录版本
      try {
        const v = JSON.parse(readFileSync(join(process.cwd(), "node_modules", d, "package.json"), "utf8")).version;
        pkg.dependencies[d] = "^" + v;
      } catch {
        pkg.dependencies[d] = "latest";
      }
    }
  }
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n", "utf8");
  console.log("deps added to package.json:", added.join(", ") || "(none)");
}
console.log("DONE");
