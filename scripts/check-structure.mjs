/**
 * 项目结构守卫。
 *
 * 约定是"只有维护者记得"的东西,过几个月就会失效。这里把三条最容易
 * 退化的规则变成可执行检查,`npm run lint` 会一起跑:
 *
 *   1. 单文件不超过 MAX_LINES —— 超过说明该拆了(App.vue 曾长到 1133 行)。
 *   2. 前端不得直接 import "@tauri-apps/api/core" —— 一律走 src/api.ts,
 *      否则命令改名时 TypeScript 抓不到。
 *   3. src/views 与 src/components 里只许出现 .vue,不许混入 .ts 逻辑。
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const SRC = join(ROOT, "src");

/** 单文件行数上限。超过就该拆分,而不是继续往里加。 */
const MAX_LINES = 450;

/** 允许直接调用 Tauri invoke 的文件(封装层自身) */
const INVOKE_ALLOWED = new Set(["src/api.ts", "src/lib/devMock.ts"]);

const problems = [];

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      // shadcn 自动生成的基础组件不参与任何约束
      if (name === "ui" && dir.includes("components")) continue;
      walk(full);
    } else {
      check(full);
    }
  }
}

function check(file) {
  const rel = relative(ROOT, file).replace(/\\/g, "/");
  if (!/\.(ts|vue)$/.test(file)) return;

  const text = readFileSync(file, "utf8");
  const lines = text.split("\n").length;

  if (lines > MAX_LINES) {
    problems.push(`${rel}: ${lines} 行,超过 ${MAX_LINES} 行上限,请拆分`);
  }

  if (/@tauri-apps\/api\/core/.test(text) && !INVOKE_ALLOWED.has(rel)) {
    problems.push(`${rel}: 直接 import 了 @tauri-apps/api/core,请改用 src/api.ts`);
  }

  if (/^src\/(views|components)\//.test(rel) && rel.endsWith(".ts")) {
    problems.push(`${rel}: views/components 下只放 .vue,逻辑请移到 src/composables`);
  }
}

walk(SRC);

if (problems.length) {
  console.error("项目结构检查未通过:\n");
  for (const p of problems) console.error("  ✗ " + p);
  console.error("");
  process.exit(1);
}
console.log(`项目结构检查通过(单文件 ≤ ${MAX_LINES} 行,invoke 统一走 api.ts)`);
