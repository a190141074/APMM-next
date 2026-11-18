import { portableBuildReport } from "./work-paths.mjs";
// APX 状态和缓存导出使用真实桌面 IPC，全部落点限于新建的隔离目录。
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, writeFileSync, copyFileSync, renameSync, existsSync } from "node:fs";
import { randomUUID, createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
const root=path.resolve(import.meta.dirname,"..");
const build=JSON.parse(readFileSync(portableBuildReport,"utf8"));
const output=path.join("B:/APMM-next-work/apx-validation",randomUUID());
assert.ok(!path.resolve(output).startsWith(root+path.sep));
const app=path.join(output,"app");cpSync(build.payload,app,{recursive:true});
const cache=path.join(app,"data/cache/apx");
cpSync(path.join(root,"dist/APMM-next/data/cache/apx"),cache,{recursive:true});
cpSync(path.join(root,"dist/APMM-next/data/plugins"),path.join(app,"data/plugins"),{recursive:true});
const roots={pluginRootDir:path.join(output,"game"),pluginDataDir:path.join(output,"game/Data"),pluginLocallowDir:path.join(output,"locallow")};
for(const dir of Object.values(roots))mkdirSync(dir,{recursive:true});
writeFileSync(path.join(app,"data/library.json"),JSON.stringify({version:1,settings:{...roots,modimgPath:path.join(output,"game/modimg"),pluginPlatform:"CN_STEAM",updateAutoCheck:false},heroes:{},monsters:{}}));
const child=spawn(path.join(app,"astral-mod-manager.exe"),[],{cwd:app,windowsHide:true,stdio:"ignore",env:{...process.env,APMM_WORK_DIR:"B:/APMM-next-work",TEMP:"B:/APMM-next-work/temp",TMP:"B:/APMM-next-work/temp",WEBVIEW2_USER_DATA_FOLDER:path.join(output,"webview"),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:"--remote-debugging-port=9423"}});
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let browser,page;const checks=[];
try {
 for(let i=0;i<120;i++){try{browser=await chromium.connectOverCDP("http://127.0.0.1:9423");break;}catch{await delay(500);}}
 assert.ok(browser);for(let i=0;i<80;i++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().startsWith("devtools:"));if(page)break;await delay(250);}
 await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__);
 const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 const status=()=>call("plugin_status",{platform:"CN_STEAM"});
 const initial=await status();assert.ok(initial.cacheComplete);assert.equal(initial.installedFiles,0);assert.ok(initial.totalFiles>0);
 const manifestPath=path.join(app,"data/plugins/CN_STEAM/manifest.json");const manifest=JSON.parse(readFileSync(manifestPath,"utf8"));
 function cacheFile(file){return path.join(cache,"CN_STEAM",initial.cacheVersion,"files",file.relativePath);}
 const first=initial.files[0],second=initial.files[1];assert.ok(path.resolve(first.target).startsWith(path.resolve(output)+path.sep));
 mkdirSync(path.dirname(first.target),{recursive:true});copyFileSync(cacheFile(first),first.target);
 let state=await status();assert.equal(state.installedFiles,1);checks.push("文件哈希匹配的一项安装与其余缺失");
 mkdirSync(path.dirname(second.target),{recursive:true});writeFileSync(second.target,"invalid patch");
 state=await status();assert.ok(state.files.find(f=>f.relativePath===second.relativePath).validationError);checks.push("文件存在但校验失败不会被算为已安装");
 manifest.version="fixture-newer-version";writeFileSync(manifestPath,JSON.stringify(manifest));state=await status();assert.equal(state.cacheUpToDate,false);assert.equal(state.cacheComplete,true);checks.push("完整旧缓存与远端清单版本差异");
 const before=createHash("sha256").update(readFileSync(first.target)).digest("hex");
 const id=await call("task_execute",{kind:"apx-export",title:"隔离旧缓存导出",args:{platform:"CN_STEAM",destination:path.join(output,"APX-old-cache.zip"),useCached:true}});
 let task;for(let i=0;i<300;i++){task=(await call("tasks_list")).find(t=>t.id===id);if(task && !["queued","running"].includes(task.status))break;await delay(200);}
 assert.equal(task.status,"completed",JSON.stringify(task));assert.ok(existsSync(path.join(output,"APX-old-cache.zip")));assert.equal(createHash("sha256").update(readFileSync(first.target)).digest("hex"),before);checks.push("导出旧缓存成功且不修改隔离游戏文件");
 renameSync(cacheFile(first),cacheFile(first)+".missing");try{state=await status();assert.equal(state.cacheComplete,false);}finally{renameSync(cacheFile(first)+".missing",cacheFile(first));}checks.push("缺缓存文件会使完整性检查失败");
 const welcome=page.getByRole("button",{name:"先进入工作区",exact:true});if(await welcome.count())await welcome.click();
 await page.locator(".nav-item").filter({hasText:"APX 插件"}).click();await page.locator(".apx-stats").waitFor();await page.waitForFunction(() => document.querySelectorAll(".apx-stats .stat strong")[3]?.textContent?.startsWith("1 /"));await page.screenshot({path:path.join(output,"apx-real-partial.png")});
 writeFileSync(path.join(root,"verification-apx-runtime.json"),JSON.stringify({output,checks,initial,state,export:task.result},null,2));
 console.log(JSON.stringify({output,checks}));
} finally {await browser?.close().catch(()=>{});if(child.exitCode===null)child.kill();}
