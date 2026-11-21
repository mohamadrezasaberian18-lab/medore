// Drive the running MEDORE windows over WebView2's DevTools protocol.
//   node scripts/cdp.mjs <window: main|overlay> <command> [args...]
// commands:
//   eval "<js expression>"          print the JSON result
//   click "<css selector>"          real mouse click on the element's centre
//   rclick "<css selector>"         right click
//   hover "<css selector>"
//   key "<key>"                     e.g. Space, ArrowRight, F1, Control+o
//   type "<text>"
//   shot <out.png>                  screenshot of the web page only
//   wait <ms>
// Several commands can be chained with " ;; ".
import { chromium } from "playwright-core";

const [, , win, ...rest] = process.argv;
const browser = await chromium.connectOverCDP("http://127.0.0.1:9333", { timeout: 40000 });
const pages = browser.contexts().flatMap((c) => c.pages());
const page = pages.find((p) => (win === "overlay" ? p.url().includes("#overlay") : !p.url().includes("#overlay")));
if (!page) {
  console.error("window not found; pages:", pages.map((p) => p.url()));
  process.exit(2);
}

const steps = rest.join(" ").split(" ;; ").map((s) => s.trim()).filter(Boolean);
for (const step of steps) {
  const m = /^(\w+)\s*(.*)$/s.exec(step);
  const cmd = m[1];
  let arg = m[2].trim();
  if (arg.startsWith('"') && arg.endsWith('"')) arg = arg.slice(1, -1);
  switch (cmd) {
    case "eval": {
      const r = await page.evaluate(`(async () => (${arg}))()`);
      console.log(JSON.stringify(r, null, 1));
      break;
    }
    case "click":
      await page.click(arg, { timeout: 5000 });
      break;
    case "rclick":
      await page.click(arg, { button: "right", timeout: 5000 });
      break;
    case "dblclick":
      await page.dblclick(arg, { timeout: 5000 });
      break;
    case "hover":
      await page.hover(arg, { timeout: 5000 });
      break;
    case "key":
      await page.keyboard.press(arg);
      break;
    case "type":
      await page.keyboard.type(arg);
      break;
    case "shot":
      await page.screenshot({ path: arg });
      console.log("shot ->", arg);
      break;
    case "wait":
      await page.waitForTimeout(Number(arg));
      break;
    case "mouse": {
      const [x, y, button] = arg.split(/\s+/);
      await page.mouse.click(Number(x), Number(y), { button: button || "left" });
      break;
    }
    default:
      console.error("unknown command", cmd);
  }
}
await browser.close().catch(() => {});
process.exit(0);
