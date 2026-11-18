import { portableBuildReport } from "./work-paths.mjs";
// 通过真实 Windows 进程验证更新助手：启动确认、无法启动和中断恢复三条路径。
import {spawnSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync,copyFileSync,existsSync} from 'node:fs';
import {createHash,randomUUID} from 'node:crypto';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'..');
const build=JSON.parse(readFileSync(portableBuildReport,'utf8'));
const work=process.env.APMM_WORK_DIR||'B:/APMM-next-work';const stage=path.join(work,'staging','updater-acceptance-'+randomUUID());mkdirSync(stage,{recursive:true});
const env={...process.env,APMM_WORK_DIR:work,TEMP:path.join(work,'temp'),TMP:path.join(work,'temp')};
const source=path.join(stage,'probe.rs');
writeFileSync(source,`#![windows_subsystem="windows"]\nfn main(){let dir=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();std::fs::write(dir.join("probe-result.txt"),if cfg!(new_version){"new"}else{"old"}).unwrap();if cfg!(new_version){if let Ok(ack)=std::env::var("APMM_TEST_ACK"){std::fs::write(ack,br#"{"ready":true,"version":"${build.version}"}"#).unwrap();}}}\n`);
for(const version of ['old','new']){
 const vs=process.env.APMM_VSDEV||'B:/VS/BuildTools/Common7/Tools/VsDevCmd.bat';
 const rust=process.env.APMM_RUST_BIN||'B:/Rust/rustup/toolchains/stable-x86_64-pc-windows-msvc/bin';
 const command=`call "${vs}" -arch=x64 -host_arch=x64 >nul && "${path.join(rust,'rustc.exe')}" "${source}" -o "${path.join(stage,version+'.exe')}" ${version==='new'?'--cfg new_version':''}`;
 const result=spawnSync('cmd.exe',['/d','/s','/c',command],{env,windowsVerbatimArguments:true,windowsHide:true,encoding:'utf8'});assert.equal(result.status,0,result.stderr);
}
const hash=f=>createHash('sha256').update(readFileSync(f)).digest('hex');const checks=[];
for(const kind of ['success','bad-launch','interrupted']){
 const area=path.join(stage,kind);const payload=path.join(area,'payload');const destination=path.join(area,'app');const backup=path.join(area,'backup');
 for(const dir of [payload,destination,backup,path.join(destination,'data/updates')])mkdirSync(dir,{recursive:true});
 copyFileSync(path.join(stage,'old.exe'),path.join(destination,'astral-mod-manager.exe'));
 writeFileSync(path.join(destination,'data/user.txt'),'user-owned');writeFileSync(path.join(destination,'unknown-user.txt'),'keep');
 const ended=spawnSync(path.join(destination,'astral-mod-manager.exe'),[],{env,cwd:destination,windowsHide:true});assert.equal(ended.status,0);
 const executable=path.join(payload,'astral-mod-manager.exe');
 if(kind==='bad-launch')writeFileSync(executable,'invalid EXE');else copyFileSync(path.join(stage,'new.exe'),executable);
 mkdirSync(path.join(payload,'licenses'),{recursive:true});mkdirSync(path.join(destination,'licenses'),{recursive:true});
 writeFileSync(path.join(payload,'licenses/THIRD_PARTY_NOTICES.md'),'new-license');writeFileSync(path.join(destination,'licenses/THIRD_PARTY_NOTICES.md'),'old-license');
 const id=randomUUID();const job={schema:1,id,source:payload,destination,backup,pid:ended.pid,version:build.version,files:[{path:'astral-mod-manager.exe',bytes:readFileSync(executable).length,sha256:hash(executable)}],existed:[],status:'new',message:''};
 const license=path.join(payload,'licenses/THIRD_PARTY_NOTICES.md');job.files.push({path:'licenses/THIRD_PARTY_NOTICES.md',bytes:readFileSync(license).length,sha256:hash(license)});
 if(kind==='interrupted'){copyFileSync(path.join(destination,'astral-mod-manager.exe'),path.join(backup,'astral-mod-manager.exe'));copyFileSync(executable,path.join(destination,'astral-mod-manager.exe'));mkdirSync(path.join(backup,'licenses'),{recursive:true});copyFileSync(path.join(destination,'licenses/THIRD_PARTY_NOTICES.md'),path.join(backup,'licenses/THIRD_PARTY_NOTICES.md'));copyFileSync(license,path.join(destination,'licenses/THIRD_PARTY_NOTICES.md'));job.existed=['astral-mod-manager.exe','licenses/THIRD_PARTY_NOTICES.md'];job.status='applying';}
 const jobPath=path.join(area,'job.json');writeFileSync(jobPath,JSON.stringify(job));
 const result=spawnSync(path.join(build.payload,'astral-updater.exe'),[jobPath,...(kind==='interrupted'?['--recover']:[])],{env:{...env,APMM_TEST_ACK:path.join(destination,'data/updates',`ack-${id}.json`)},windowsHide:true,timeout:90000});
 assert.ok(!result.error,result.error?.message);
 const completed=JSON.parse(readFileSync(jobPath,'utf8'));assert.equal(completed.status,kind==='success'?'complete':'rolled-back',JSON.stringify(completed));
 assert.equal(hash(path.join(destination,'astral-mod-manager.exe')),hash(path.join(stage,kind==='success'?'new.exe':'old.exe')));
 assert.equal(readFileSync(path.join(destination,'data/user.txt'),'utf8'),'user-owned');assert.equal(readFileSync(path.join(destination,'unknown-user.txt'),'utf8'),'keep');
 assert.equal(readFileSync(path.join(destination,'licenses/THIRD_PARTY_NOTICES.md'),'utf8'),kind==='success'?'new-license':'old-license');
 checks.push({kind,status:completed.status,exit:result.status,userDataPreserved:true});
}
writeFileSync(path.join(root,'verification-updater.json'),JSON.stringify({stage,checks},null,2));console.log(JSON.stringify({stage,checks}));
