import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const context=vm.createContext({TextEncoder,Uint8Array});
vm.runInContext(readFileSync(new URL('../portal/drive-transfers.js',import.meta.url),'utf8').replace(/^export /gm,''),context);
const api=vm.runInContext('({TransferQueue,TransferCancelled,TransferSkipped,localFiles,droppedFiles,uploadJobs,walkFiles,relativePath,keepBothName})',context);
const file=(name,path=name)=>({name,webkitRelativePath:path,size:1,type:'text/plain',arrayBuffer:async()=>new Uint8Array([42])});
const tick=()=>new Promise(resolve=>setImmediate(resolve));
test('queue keeps successful siblings, retries only failed work and distinguishes cancellation from a committed result',async()=>{
  const queue=new api.TransferQueue(),calls=[];let failed=true,release,progress;
  await queue.add(['a','b','c'].map(name=>({name,valid:()=>true,run:async()=>{calls.push(name);if(name==='b' && failed)throw new Error('lost response');}})));
  assert.deepEqual(Array.from(queue.jobs,j=>j.status),['done','failed','done']);failed=false;await queue.retry(queue.jobs[1]);assert.deepEqual(calls,['a','b','c','b']);
  const pending=queue.add([{name:'slow',valid:()=>true,run:async control=>{progress=control.progress;await new Promise(r=>{release=r;});}}]);
  const job=queue.jobs.at(-1);queue.cancel(job);assert.equal(progress(1,2),false);release();await pending;assert.equal(job.status,'done');
  queue.clear();assert.equal(queue.jobs.length,0);
});
test('account switch prevents remaining jobs and aborts work before publication',async()=>{
  let valid=true,release,published=false;const queue=new api.TransferQueue();
  const running=queue.add([{name:'a',valid:()=>valid,run:async control=>{await new Promise(r=>{release=r;});control.check();published=true;}},{name:'b',valid:()=>valid,run:()=>{published=true;}}]);
  valid=false;queue.reset();release();await running;assert.equal(published,false);assert.equal(queue.jobs.length,0);
});
test('folder selection preserves hierarchy and rejects ambiguous, traversal and excessive-depth paths',()=>{
  const items=api.localFiles([file('a.txt','Root/Nested/a.txt'),file('b.txt','Root/b.txt')]);
  assert.deepEqual(Array.from(items,item=>item.path.join('/')),['Root','Root/Nested','Root/Nested/a.txt','Root/b.txt']);
  for(const path of ['../x','a//x','/root','a/../b','a\\b','a/'.repeat(65)+'b'])assert.throws(()=>api.relativePath(path));
  assert.throws(()=>api.localFiles([file('Root'),file('a','Root/a')]));
});
test('directory drops drain all reader batches and retain empty folders',async()=>{
  let reads=0;const leaf={name:'a.txt',isFile:true,file:resolve=>resolve(file('a.txt'))};
  const empty={name:'Empty',isDirectory:true,createReader:()=>({readEntries:resolve=>resolve([])})};
  const root={name:'Root',isDirectory:true,createReader:()=>({readEntries:resolve=>resolve([ [leaf], [empty], [] ][reads++])})};
  const items=await api.droppedFiles({items:[{kind:'file',webkitGetAsEntry:()=>root}],files:[]});
  assert.equal(reads,3);assert.deepEqual(Array.from(items,item=>item.path.join('/')),['Root','Root/a.txt','Root/Empty']);
});
test('folder/file jobs retry the same publication payload, merge explicitly, and skip entire subtrees',async()=>{
  const uploads=[],folders=[],choices=[],children=new Map([['1',[]]]);let id=1,lost=true;
  const adapter={id:()=>`request-${id++}`,valid:()=>true,format:()=>({mediaType:'text/plain',craft:null}),
    list:async parent=>({entries:children.get(parent)||[],cursor:null}),
    collision:async(item,existing)=>{choices.push(item.path.join('/'));return item.folder?'merge':'keep';},
    folder:async payload=>{folders.push(payload);children.set('2',[]);children.get('1').push({id:'2',name:'Root',folder:true});return {id:'2'};},
    upload:async payload=>{uploads.push(payload);if(lost){lost=false;throw new Error('lost commit');}return {id:'3'};}};
  const queue=new api.TransferQueue();await queue.add(api.uploadJobs(api.localFiles([file('a.txt','Root/a.txt')]),'1',adapter));
  assert.equal(folders.length,1);assert.equal(queue.jobs[1].status,'failed');await queue.retry(queue.jobs[1]);
  assert.deepEqual(uploads[0],uploads[1]);assert.equal(folders.length,1);
  adapter.collision=async()=> 'skip';
  const skipQueue=new api.TransferQueue();await skipQueue.add(api.uploadJobs(api.localFiles([file('a.txt','Root/a.txt')]),'1',adapter));
  assert.deepEqual(Array.from(skipQueue.jobs,j=>j.status),['skipped','skipped']);assert.equal(uploads.length,2);
});
test('replacement captures the existing revision and keep-both preserves extensions',async()=>{
  const uploaded=[];const queue=new api.TransferQueue();
  const adapter={id:()=> 'replace-request-1',valid:()=>true,format:()=>({}),list:async()=>({entries:[{id:'4',name:'a.txt',revision:'8',folder:false,capabilities:{edit:true}}],cursor:null}),collision:async()=> 'replace',upload:async payload=>uploaded.push(payload)};
  await queue.add(api.uploadJobs(api.localFiles([file('a.txt')]),'1',adapter));assert.equal(uploaded[0].file,'4');assert.equal(uploaded[0].revision,'8');
  assert.equal(api.keepBothName('a.txt',[{name:'a.txt'},{name:'a (2).txt'}]),'a (3).txt');
});
test('cancelled folders cancel descendants without creating them, while unrelated files finish',async()=>{
  let release,folders=0;const uploaded=[];
  const adapter={id:()=> 'cancel-folder-request',valid:()=>true,format:()=>({}),list:async()=>({entries:[],cursor:null}),
    folder:async()=>{folders++;return {id:'2'};},upload:async target=>uploaded.push(target.name)};
  const queue=new api.TransferQueue();
  const pending=queue.add([{name:'hold',valid:()=>true,run:()=>new Promise(r=>{release=r;})},
    ...api.uploadJobs(api.localFiles([file('a.txt','Root/Nested/a.txt'),file('other.txt')]),'1',adapter)]);
  queue.cancel(queue.jobs.find(job=>job.name==='Root'));release();await pending;
  assert.equal(folders,0);assert.deepEqual(uploaded,['other.txt']);
  assert.deepEqual(Array.from(queue.jobs.filter(job=>job.name.startsWith('Root')),job=>job.status),['cancelled','cancelled','cancelled']);
});
test('a failed folder is retried explicitly before its failed children, retaining its request',async()=>{
  let lost=true;const created=[],uploaded=[];
  const adapter={id:()=> 'same-folder-request',valid:()=>true,format:()=>({}),list:async()=>({entries:[],cursor:null}),
    folder:async target=>{created.push(target);if(lost)throw new Error('lost folder response');return {id:'2'};},upload:async target=>uploaded.push(target)};
  const queue=new api.TransferQueue();await queue.add(api.uploadJobs(api.localFiles([file('a.txt','Root/a.txt')]),'1',adapter));
  assert.equal(created.length,1);assert.equal(uploaded.length,0);
  assert.deepEqual(Array.from(queue.jobs,job=>job.status),['failed','failed']);
  lost=false;await queue.retry(queue.jobs[0]);await queue.retry(queue.jobs[1]);
  assert.deepEqual(created[0],created[1]);assert.equal(uploaded[0].parent,'2');
  assert.deepEqual(Array.from(queue.jobs,job=>job.status),['done','done']);
});
test('folder traversal handles pagination, preserves empty directories and rejects cycles or foreign children',async()=>{
  const root={id:'1',spaceId:'1',folder:true,name:'Root'};
  const files=await api.walkFiles([root],async(parent,cursor)=>parent==='2'?{entries:[],cursor:null}:{entries:[{id:cursor?'3':'2',parent:'1',spaceId:'1',folder:!cursor,name:cursor?'file.txt':'Empty'}],cursor:cursor?null:'next'});
  assert.deepEqual(Array.from(files,item=>item.path),['Root','Root/Empty','Root/file.txt']);
  await assert.rejects(api.walkFiles([root],async()=>({entries:[{...root,parent:'1'}],cursor:null})),/traversal/);
  await assert.rejects(api.walkFiles([root],async()=>({entries:[{id:'5',parent:'1',spaceId:'9',name:'private'}],cursor:null})),/Folder changed/);
});
