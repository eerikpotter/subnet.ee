import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { localeHost } from './i18n-test-helpers.mjs';
import { domHost, Element } from './i18n-dom-helpers.mjs';

const html = readFileSync(new URL('../portal/pilveruum/index.html', import.meta.url),'utf8');
const source = readFileSync(new URL('../portal/cloud.js', import.meta.url),'utf8').replace(/^import .*;\n/,'');
const tick = () => new Promise(resolve=>setImmediate(resolve));
function fixture(options={}) {
  const nodes = [...html.matchAll(/id="([^"]+)"/g)].map(([,id])=>new Element('div',{id}));
  const document=domHost(nodes);document.body=new Element('body');document.body.classList.add=()=>{};
  for(const node of nodes) {node.dataset={};node.showModal=()=>{node.open=true;};node.close=()=>{const open=node.open;node.open=false;if(open)node.dispatchEvent(new Event('close'));};node.focus=()=>{};node.select=()=>{};}
  const create=document.createElement; document.createElement=tag=>{const node=create(tag);node.dataset={};node.focus=()=>{};node.select=()=>{};node.pause=()=>{node.paused=true;};node.load=()=>{};return node;};
  const host=localeHost({pathname:'/pilveruum/',search:options.search||'#failid',document});const calls=[];const selections=[];host.window.parent={postMessage:(data,origin)=>selections.push({data,origin})};
  const root={id:'1',owner:'alice',folder:true,name:'My files',capabilities:{read:true,create:true,edit:true,delete:true},...options.root};
  const file=id=>({id,spaceId:'1',owner:'alice',name:'Ideed',folder:false,format:{craft:{format_id:'grid',format_version:1}},bytes:1572864,updatedMs:1700000000000,revision:'44',version:'1',capabilities:{read:true,edit:true,create:true,delete:true},...options.file});
  const urls=[];class PreviewURL extends URL {static createObjectURL(blob){urls.push(['create',blob]);return 'blob:preview';}static revokeObjectURL(url){urls.push(['revoke',url]);}}
  Object.assign(host.context,{URL:PreviewURL,Blob,TextDecoder,TextEncoder,location:host.window.location,crypto:{randomUUID:()=> 'test-request-id-0001'},setInterval(){},async init(){},
    async fetch(){return {ok:true,json:async()=>({apps:[{id:'gridcraft',name:'GridCraft',cloud:true,cloudExtension:'scjson',openFormats:[{id:'grid',version:1}]}]})};},
    CloudPortal:class {
      async restore(){calls.push(['restore']);return {authenticated:true,principal:'alice',spaceId:'1',usedBytes:100,quotaBytes:10000,...(options.home ? {home:{folder:root,breadcrumbs:[root],entries:[file('first')],cursor:null,catalog:options.catalog}} : {})};}
      needs_refresh(){return options.stale || false;}
      async upload(payload,bytes,progress){calls.push(['upload',JSON.parse(payload)]);if(options.upload)return options.upload(payload,bytes,progress);}
      async download(id,revision){calls.push(['download',id,revision]);return options.download ? options.download(id,revision) : new TextEncoder().encode('Example text');}
      async drive(method,payload){const p=JSON.parse(payload);calls.push([method,p]);
        if(options.drive){const result=options.drive(method,p);if(result!==undefined)return result;}
        if(method==='initialize')return root;
        if(method==='browse')return {folder:{...root,id:p.parent,name:p.parent==='1'?'My files':'Nested'},breadcrumbs:[root],entries:[file(p.cursor?'second':'first')],cursor:p.cursor?'third-page':'second-page'};
        if(method==='list')return {entries:[file(p.cursor?'second':'first')],cursor:p.cursor?'third-page':'second-page'};
        if(method==='folder')return {...root,id:'2',name:p.name};
        if(method==='get')return {...root,id:p.id,parent:'1',name:'Nested'};
        throw new Error('Unexpected call '+method);
      }
    },
  });
  host.start();return {host,document,calls,urls,selections};
}

test('drive language switches preserve session, loaded pages, cursor, file names and app links',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  const rows=[...document.getElementById('file-list').children],before=JSON.stringify(calls);
  for(const language of ['en','et','en'])host.i18n.setLanguage(language);
  assert.equal(JSON.stringify(calls),before);assert.deepEqual(document.getElementById('file-list').children,rows);assert.equal(rows.length,2);
  assert.equal(rows[0].children[1].children[0].textContent,'Ideed');assert.equal(document.getElementById('account-status').textContent,'My account');
  assert.equal(rows[0].children.at(-1).children[0].getAttribute('aria-label'),'File actions');
  assert.equal(rows[0].children[2].href,'/apps/gridcraft/?cloud_service=cloud&cloud_space=1&cloud_file=first&lang=en');
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.at(-1)[1].cursor,'third-page');
});

test('new folder submits the selected parent and navigates through the canonical drive API',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('new-folder').dispatchEvent(new Event('click'));
  const input=document.getElementById('dialog-body').children[0].children[0];input.value='Test folder';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.find(c=>c[0]==='folder')[1],{parent:'1',name:'Test folder',requestId:'test-request-id-0001'});
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(calls.at(-1)[0],'browse');assert.equal(calls.at(-1)[1].parent,'2');
  assert.equal(document.getElementById('files-title').textContent,'Nested');
});


test('cached navigation renders immediately, revalidates, and removes a revoked view',async()=>{
  let resolve, hold=false;
  const {host,document,calls}=fixture({drive(method){if(method==='browse' && hold)return new Promise(r=>{resolve=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  host.context.location.hash='#failid';host.window.dispatchEvent(new Event('hashchange'));await tick();
  const before=calls.length;hold=true;
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));
  assert.equal(document.getElementById('files-title').textContent,'Nested');
  assert.equal(document.getElementById('file-list').children.length,1);
  assert.equal(document.getElementById('new-folder').disabled,true);
  assert.equal(calls.length,before+1);assert.equal(calls.at(-1)[0],'browse');
  resolve(Promise.reject(new Error('Not found or access unavailable')));await tick();
  assert.equal(document.getElementById('file-list').children.length,0);
  assert.equal(document.getElementById('new-folder').disabled,true);
});

test('upload is visible before reading bytes and remains active through final commit and navigation',async()=>{
  let readBytes, commit, notify;
  const {host,document,calls}=fixture({upload(payload,bytes,progress){notify=progress;return new Promise(r=>{commit=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const input=document.getElementById('upload-input');
  input.files=[{name:'progress.txt',size:1024,type:'text/plain',arrayBuffer:()=>new Promise(r=>{readBytes=r;})}];
  input.dispatchEvent(new Event('change'));
  assert.equal(document.getElementById('transfer-status').hidden,false);
  assert.match(document.getElementById('transfer-label').textContent,/Valmistan ette/);
  readBytes(new ArrayBuffer(1024));await tick();notify(1,2);
  assert.equal(document.getElementById('transfer-progress').value,0.5);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  notify(2,2);assert.match(document.getElementById('transfer-label').textContent,/Salvestan/);
  assert.equal(document.getElementById('transfer-status').hidden,false);
  commit({id:'99'});await tick();
  assert.equal(document.getElementById('transfer-status').hidden,true);
  assert.equal(calls.find(c=>c[0]==='upload')[1].parent,'1');
  assert.equal(document.getElementById('account-message').textContent,'Üles laaditud.');
});


test('a late folder response cannot overwrite a newer navigation',async()=>{
  let resolve;
  const {host,document}=fixture({drive(method,p){if(method==='browse' && p.parent==='2')return new Promise(r=>{resolve=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  host.context.location.hash='#folder/3';host.window.dispatchEvent(new Event('hashchange'));await tick();
  resolve({folder:{id:'2',owner:'alice',name:'Outdated response',capabilities:{}},breadcrumbs:[],entries:[],cursor:null});await tick();
  assert.equal(document.getElementById('files-title').textContent,'Nested');
  assert.equal(document.getElementById('file-list').children.length,1);
});


test('initial home response renders without follow-up metadata calls',async()=>{
  const {host,document,calls}=fixture({home:true});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.deepEqual(calls,[['restore']]);
  assert.equal(document.getElementById('file-list').children.length,1);
  assert.equal(document.getElementById('new-folder').disabled,false);
});


test('the first visit to a prefetched folder displays content before its network reply',async()=>{
  const child={id:'2',parent:'1',owner:'alice',folder:true,name:'Prefetched folder',capabilities:{read:true,create:true}};
  const file={id:'3',parent:'2',owner:'alice',name:'Already here.txt',folder:false,bytes:10,updatedMs:1700000000000,capabilities:{read:true}};
  let finish;
  const {host,document}=fixture({home:true,catalog:{entries:[child,file],completeFolders:['2']},drive(method,p){if(method==='browse' && p.parent==='2')return new Promise(r=>{finish=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));
  assert.equal(document.getElementById('files-title').textContent,'Prefetched folder');
  assert.equal(document.getElementById('file-list').children[0].children[1].children[0].textContent,'Already here.txt');
  assert.equal(document.getElementById('new-folder').disabled,true);
  finish({folder:child,breadcrumbs:[child],entries:[file],cursor:null});await tick();
  assert.equal(document.getElementById('new-folder').disabled,false);
});

const previewFile={name:'example.txt',format:{media_type:'text/plain',craft:null},bytes:100};
function openPreview(document, event='dblclick') {
  const row=document.getElementById('file-list').children[0];
  const input=new Event(event,{cancelable:true});if(event==='keydown')input.key='Enter';row.dispatchEvent(input);
}
test('text preview uses literal text, keyboard opening and closes on Escape',async()=>{
  const value='<img src=x onerror="alert(1)"><script>steal()</script>';
  const {host,document,calls}=fixture({file:previewFile,download:async()=>new TextEncoder().encode(value)});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  openPreview(document,'keydown');await tick();
  assert.equal(document.getElementById('preview-dialog').open,true);
  const pre=document.getElementById('preview-content').children[0];
  assert.equal(pre.tagName,'PRE');assert.equal(pre.textContent,value);assert.equal(pre.children.length,0);
  assert.deepEqual(calls.at(-1),['download','first','44']);
  document.getElementById('preview-dialog').dispatchEvent(new Event('cancel',{cancelable:true}));
  assert.equal(document.getElementById('preview-dialog').open,false);
  assert.equal(document.getElementById('preview-content').children.length,0);
});
test('SVG previews use an image context, release their URL and preserve filename across translation',async()=>{
  const {host,document,urls}=fixture({file:{...previewFile,name:'drawing.svg',format:{media_type:'image/svg+xml'}},download:async()=>new TextEncoder().encode('<svg><script>alert(1)</script></svg>')});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);openPreview(document);await tick();
  const image=document.getElementById('preview-content').children.at(-1);
  assert.equal(image.tagName,'IMG');assert.equal(image.src,'blob:preview');assert.equal(image.children.length,0);
  assert.equal(urls[0][1].type,'image/svg+xml');host.i18n.setLanguage('en');
  assert.equal(document.getElementById('preview-name').textContent,'drawing.svg');
  document.getElementById('preview-close').dispatchEvent(new Event('click'));
  assert.deepEqual(urls.at(-1),['revoke','blob:preview']);
});
test('video preview has controls, no autoplay, and stops when closed',async()=>{
  const {host,document,urls}=fixture({file:{...previewFile,name:'clip.mp4',format:{media_type:'video/mp4'}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);openPreview(document);await tick();
  const video=document.getElementById('preview-content').children.at(-1);
  assert.equal(video.tagName,'VIDEO');assert.equal(video.controls,true);assert.notEqual(video.autoplay,true);
  document.getElementById('preview-close').dispatchEvent(new Event('click'));assert.equal(video.paused,true);
  assert.deepEqual(urls.at(-1),['revoke','blob:preview']);
});
test('closed or account-stale previews cannot publish late bytes',async()=>{
  let finish;const options={file:previewFile,download:()=>new Promise(r=>{finish=r;})};
  const {host,document}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  openPreview(document);await tick();document.getElementById('preview-close').dispatchEvent(new Event('click'));
  finish(new TextEncoder().encode('late private content'));await tick();
  assert.equal(document.getElementById('preview-content').children.length,0);
  openPreview(document);await tick();options.stale=true;
  const change=new Event('storage');change.key='ic-identity';host.window.dispatchEvent(change);
  finish(new TextEncoder().encode('other account'));await tick();
  assert.equal(document.getElementById('preview-dialog').open,false);
  assert.equal(document.getElementById('preview-content').children.length,0);
});
test('large text and unsupported files do not automatically fetch content',async()=>{
  for(const file of [{...previewFile,bytes:1024*1024+1},{...previewFile,name:'archive.zip',format:{media_type:'application/zip'}}]) {
    const {host,document,calls}=fixture({file});await vm.runInContext(`(async()=>{${source}})()`,host.context);
    openPreview(document);await tick();assert.equal(calls.some(c=>c[0]==='download'),false);
    assert.match(document.getElementById('preview-content').textContent,/alla laadida/);
  }
});
test('sharing badges distinguish direct/inherited sharing without another request',async()=>{
  for(const sharing of [{direct:true,inherited:false},{direct:false,inherited:true},{direct:true,inherited:true}]) {
    const {host,document,calls}=fixture({file:{sharing}});await vm.runInContext(`(async()=>{${source}})()`,host.context);
    const name=document.getElementById('file-list').children[0].children[1].children[0];
    assert.equal(name.children[1].getAttribute('role'),'img');assert.match(name.children[1].title,/Jagatud/);
    if(sharing.inherited)assert.match(name.children[1].title,/ülemkausta/);
    assert.equal(calls.some(c=>c[0]==='grants'),false);host.i18n.setLanguage('en');
    assert.match(name.children[1].getAttribute('aria-label'),/Shared/);
  }
});
test('file menus close on another menu, outside click and Escape',async()=>{
  const {host,document}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  const [first,second]=document.getElementById('file-list').children.map(row=>row.children.at(-1));
  first.open=true;second.children[0].dispatchEvent(new Event('click'));second.open=true;
  assert.equal(first.open,false);assert.equal(second.open,true);
  const outside=new Event('click');Object.defineProperty(outside,'target',{value:new Element('main')});
  document.dispatchEvent(outside);assert.equal(second.open,false);
  first.open=true;const escape=new Event('keydown');escape.key='Escape';document.dispatchEvent(escape);
  assert.equal(first.open,false);
});

const pickerSearch=mode=>'?picker='+mode+'&app=gridcraft&channel=01234567-0123-0123-0123-0123456789ab&name=Shared';
test('shared Craft links open the same editor without requiring file ownership',async()=>{
  const {host,document}=fixture({file:{owner:'bob'}});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.match(document.getElementById('file-list').children[0].children[2].href,/cloud_file=first/);
});
test('picker opens a readable shared Craft file by stable reference after revalidation',async()=>{
  const file={id:'9',spaceId:'7',owner:'bob',name:'Shared.scjson',folder:false,revision:'44',format:{craft:{format_id:'grid',format_version:1}},capabilities:{read:true,edit:false}};
  const {host,document,selections,calls}=fixture({search:pickerSearch('open'),file,drive(method){if(method==='get')return file;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  openPreview(document);await tick();
  assert.equal(selections.length,1);assert.equal(selections[0].origin,'https://subnet.ee');
  assert.deepEqual(JSON.parse(JSON.stringify(selections[0].data)),{type:'subnet-cloud-selection',channel:'01234567-0123-0123-0123-0123456789ab',principal:'alice',action:'open',space:'7',file:'9',revision:'44'});
  assert.equal(calls.filter(c=>c[0]==='download').length,0);
});
test('Save As selects a shared Create-only folder without requiring Edit or uploading bytes',async()=>{
  const root={id:'1',owner:'bob',folder:true,capabilities:{read:true,create:true,edit:false}};
  const {host,document,selections,calls}=fixture({search:pickerSearch('save'),root,drive(method){if(method==='get')return root;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('picker-submit').disabled,false);
  document.getElementById('picker-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(selections[0].data.parent,'1');assert.equal(selections[0].data.name,'Shared.scjson');
  assert.equal(calls.filter(c=>c[0]==='upload').length,0);
});
test('Save As revalidates revoked Create and discards account-stale selections',async()=>{
  const root={id:'1',owner:'bob',folder:true,capabilities:{read:true,create:true}};
  const options={search:pickerSearch('save'),root,drive(method){if(method==='get')return {...root,capabilities:{read:true}};}};
  const {host,document,selections}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('picker-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(selections.length,0);assert.match(document.getElementById('account-message').textContent,/loomise/);
  options.stale=true;document.getElementById('picker-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(selections.length,0);
});
test('picker excludes incompatible file selections and cancels without a mutation',async()=>{
  const {host,document,selections,calls}=fixture({search:pickerSearch('open'),file:{format:{craft:{format_id:'other',format_version:1}}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);openPreview(document);await tick();
  assert.equal(selections.length,0);assert.equal(document.getElementById('picker-submit').disabled,true);
  document.getElementById('picker-cancel').dispatchEvent(new Event('click'));
  assert.equal(selections[0].data.action,'cancel');assert.equal(calls.filter(c=>c[0]==='get').length,0);
});
