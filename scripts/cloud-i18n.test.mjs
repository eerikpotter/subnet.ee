import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { localeHost } from './i18n-test-helpers.mjs';
import { domHost, Element } from './i18n-dom-helpers.mjs';

const html = readFileSync(new URL('../portal/pilveruum/index.html', import.meta.url),'utf8');
const source = readFileSync(new URL('../portal/cloud.js', import.meta.url),'utf8').replace(/^import .*;\n/gm,'');
const browserState=readFileSync(new URL('../portal/drive-browser.js',import.meta.url),'utf8').replace(/^export /gm,'');
const archiveSource=readFileSync(new URL('../portal/download-archive.js',import.meta.url),'utf8').replace(/^export /gm,'');
const tick = () => new Promise(resolve=>setImmediate(resolve));
function fileDrag(type, files=[], items=[]) {
  const event=new Event(type,{cancelable:true});
  event.dataTransfer={types:['Files'],files,items,dropEffect:'none'};
  return event;
}
function fixture(options={}) {
  const nodes = [...html.matchAll(/id="([^"]+)"/g)].map(([,id])=>new Element('div',{id}));
  const document=domHost(nodes);document.body=new Element('body');document.body.classList.add=()=>{};
  for(const node of nodes) {node.dataset={};node.showModal=()=>{node.open=true;};node.close=()=>{const open=node.open;node.open=false;if(open)node.dispatchEvent(new Event('close'));};node.focus=()=>{document.activeElement=node;};node.select=()=>{};}
  const create=document.createElement; document.createElement=tag=>{const node=create(tag);node.dataset={};node.focus=()=>{document.activeElement=node;};node.select=()=>{};node.setSelectionRange=(start,end)=>{node.selectionStart=start;node.selectionEnd=end;};node.pause=()=>{node.paused=true;};node.load=()=>{};return node;};
  const host=localeHost({pathname:'/pilveruum/',search:options.search||'#failid',document});const calls=[];const selections=[];host.window.parent={postMessage:(data,origin)=>selections.push({data,origin})};
  const root={id:'1',spaceId:'1',owner:'alice',folder:true,name:'My files',capabilities:{read:true,create:true,edit:true,delete:true},...options.root};
  const file=id=>({id,spaceId:'1',owner:'alice',name:'Ideed',folder:false,format:{craft:{format_id:'grid',format_version:1}},bytes:1572864,updatedMs:1700000000000,revision:'44',version:'1',capabilities:{read:true,edit:true,create:true,delete:true},...options.file});
  let requestSequence=0;const urls=[];class PreviewURL extends URL {static createObjectURL(blob){urls.push(['create',blob]);return 'blob:preview';}static revokeObjectURL(url){urls.push(['revoke',url]);}}
  Object.assign(host.context,{URL:PreviewURL,Blob,TextDecoder,TextEncoder,location:host.window.location,crypto:{randomUUID:()=> 'test-request-id-'+String(++requestSequence).padStart(4,'0')},setTimeout,clearTimeout,setInterval:options.setInterval||(()=>{}),async init(){},
    navigator:{clipboard:{writeText:async value=>{if(options.copy)return options.copy(value);}}},
    async fetch(){if(options.unavailable)throw new Error('Unavailable');return {ok:true,json:async()=>({apps:[{id:'gridcraft',name:'GridCraft',cloud:true,cloudExtension:'scjson',openFormats:[{id:'grid',version:1}]}]})};},
    CloudPortal:class {
      async sign_in(){calls.push(['sign_in']);options.signIn?.();}
      async restore(){calls.push(['restore']);return {authenticated:true,principal:'alice',spaceId:'1',usedBytes:100,quotaBytes:10000,...options.account,...(options.home ? {home:{folder:root,breadcrumbs:[root],entries:[file('first')],cursor:null,catalog:options.catalog}} : {})};}
      needs_refresh(){return options.stale || false;}
      async upload(payload,bytes,progress){calls.push(['upload',JSON.parse(payload)]);if(options.upload)return options.upload(payload,bytes,progress);}
      async download(id,revision){calls.push(['download',id,revision]);return options.download ? options.download(id,revision) : new TextEncoder().encode('Example text');}
      async drive(method,payload){const p=JSON.parse(payload);calls.push([method,p]);
        if(method==='suggestions')return options.suggestions ? options.suggestions() : {entries:[],quickFolders:[],cursor:null};
        if(options.query && method==='query')return options.query(p);
        // Older regression fixtures describe folder/trash content independently
        // of transport. Route the new query contract to those content fixtures.
        const fixtureMethod=method==='query'?(p.scope==='trash'?'trash':p.scope==='folder'||p.scope==='subtree'?'browse':'list'):method==='open'?'get':method;
        if(options.drive){const result=options.drive(fixtureMethod,p);if(result!==undefined)return result;}
        if(method==='emailInvitations')return [];
        if(['preferences','star','acknowledge','dismiss'].includes(method))return null;
        if(method==='open')return file(p.id);
        if(method==='query')return p.scope==='folder'||p.scope==='subtree'
          ? {folder:{...root,id:p.parent,name:p.parent==='1'?'My files':'Nested'},breadcrumbs:[root],entries:[file(p.cursor?'second':'first')],cursor:p.cursor?'third-page':'second-page'}
          : {entries:[file(p.cursor?'second':'first')],cursor:p.cursor?'third-page':'second-page'};
        if(method==='initialize')return root;
        if(method==='account')return {principal:'alice',usedBytes:100,quotaBytes:10000,...options.account};
        if(method==='browse')return {folder:{...root,id:p.parent,name:p.parent==='1'?'My files':'Nested'},breadcrumbs:[root],entries:[file(p.cursor?'second':'first')],cursor:p.cursor?'third-page':'second-page'};
        if(method==='list')return {entries:[file('first')],cursor:null};
        if(method==='movePreview')return {version:'1',treeVersion:'1',affectedObjects:0,affectedSubjects:0,changes:[]};
        if(method==='folder')return {...root,id:'2',name:p.name};
        if(method==='get')return ['1','2'].includes(p.id)?{...root,id:p.id,parent:'1',name:'Nested'}:file(p.id);
        throw new Error('Unexpected call '+method);
      }
    },
  });
  Object.defineProperty(host.context,'location',{get:()=>host.window.location,configurable:true});
  host.start();for(const name of ['drive-navigation','drive-recovery','drive-insights','drive-people','drive-preview'])vm.runInContext(readFileSync(new URL('../portal/'+name+'.js',import.meta.url),'utf8').replace(/^export /gm,'').replace(/^import .*;\n/gm,''),host.context);vm.runInContext(readFileSync(new URL('../portal/account-email.js',import.meta.url),'utf8').replace(/^export /gm,'').replace(/^import .*;\n/gm,''),host.context);vm.runInContext(readFileSync(new URL("../portal/drive-transfers.js",import.meta.url),"utf8").replace(/^export /gm,""),host.context);vm.runInContext(browserState,host.context);vm.runInContext(archiveSource,host.context);return {host,document,calls,urls,selections};
}

const trashEntry=(id,owner='alice')=>({id,owner,spaceId:'1',name:`File ${id}`,version:'2',folder:false,bytes:50,updatedMs:1700000000000,capabilities:{read:true,delete:true}});
function findButton(node,label) {
  if(node.tagName==='BUTTON' && node.textContent===label)return node;
  for(const child of node.children) {const found=findButton(child,label);if(found)return found;}
}

test('permanent deletion requires confirmation, preserves retries and refreshes quota',async()=>{
  let purged=false, fail=true;
  const options={search:'#prugikast',drive(method){
    if(method==='trash')return {entries:purged?[]:[trashEntry('7')],cursor:null};
    if(method==='purge') {if(fail)throw new Error('Temporary error');purged=true;options.account={usedBytes:0};return {};}
  }};
  const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const remove=()=>findButton(document.getElementById('file-list'),'Kustuta jäädavalt').dispatchEvent(new Event('click'));
  remove();assert.equal(calls.some(([m])=>m==='purge'),false);
  assert.match(document.getElementById('dialog-body').textContent,/Seda toimingut ei saa tagasi võtta/);
  document.getElementById('dialog-cancel').dispatchEvent(new Event('click'));
  assert.equal(calls.some(([m])=>m==='purge'),false);
  remove();host.i18n.setLanguage('en');
  assert.equal(document.getElementById('dialog-submit').textContent,'Delete permanently');
  assert.match(document.getElementById('dialog-body').textContent,/cannot be undone/);
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(document.getElementById('drive-dialog').open,true);
  assert.equal(document.getElementById('dialog-error').textContent,'Temporary error');
  fail=false;form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  const mutations=calls.filter(([m])=>m==='purge');assert.equal(mutations.length,2);assert.deepEqual(mutations[0],mutations[1]);
  assert.deepEqual(mutations[0][1].items,[{id:'7',version:'2'}]);
  assert.equal(document.getElementById('drive-dialog').open,false);
  assert.equal(document.getElementById('empty-trash').disabled,true);
  assert.match(document.getElementById('storage-usage').textContent,/0 B/);
  assert.equal(document.getElementById('account-message').textContent,'Permanently deleted.');
});

test('empty trash collects every page but only the current owners items in one mutation',async()=>{
  let purged=false;
  const {host,document,calls}=fixture({search:'#prugikast',drive(method,p){
    if(method==='trash')return purged ? {entries:[trashEntry('shared','bob')],cursor:null} : p.cursor ?
      {entries:[trashEntry('last')],cursor:null} : {entries:[trashEntry('first'),trashEntry('shared','bob')],cursor:'next'};
    if(method==='purge'){purged=true;return {};}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const shared=document.getElementById('file-list').children.find(row=>row.dataset.fileId==='shared');
  assert.equal(findButton(shared,'Kustuta jäädavalt'),undefined);assert.ok(findButton(shared,'Taasta'));
  document.getElementById('empty-trash').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.some(([m])=>m==='purge'),false);
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='purge').map(([,p])=>p.items),[[{id:'first',version:'2'},{id:'last',version:'2'}]]);
  assert.equal(document.getElementById('file-list').children.length,1);
  assert.equal(document.getElementById('empty-trash').disabled,true);
});

test('failed pagination and closed confirmation never submit a partial empty-trash snapshot',async()=>{
  let finish, pageCalls=0;
  const {host,document,calls}=fixture({search:'#prugikast',drive(method,p){
    if(method==='trash') {pageCalls++;if(p.cursor)return new Promise(resolve=>{finish=resolve;});return {entries:[trashEntry('first')],cursor:'next'};}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('empty-trash').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('dialog-submit').disabled,true);
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(calls.some(([m])=>m==='purge'),false);
  document.getElementById('dialog-cancel').dispatchEvent(new Event('click'));
  finish({entries:[trashEntry('last')],cursor:null});await tick();
  assert.equal(document.getElementById('drive-dialog').open,false);assert.equal(calls.some(([m])=>m==='purge'),false);
  assert.equal(pageCalls,3);
});

test('account changes invalidate pending destructive confirmations',async()=>{
  const options={search:'#prugikast',drive(method){if(method==='trash')return {entries:[trashEntry('7')],cursor:null};}};
  const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Kustuta jäädavalt').dispatchEvent(new Event('click'));
  options.stale=true;
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(calls.some(([m])=>m==='purge'),false);
});

test('drive language switches preserve session, loaded pages, cursor, file names and app links',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  const rows=[...document.getElementById('file-list').children],before=JSON.stringify(calls);
  for(const language of ['en','et','en'])host.i18n.setLanguage(language);
  assert.equal(JSON.stringify(calls),before);assert.deepEqual(document.getElementById('file-list').children,rows);assert.equal(rows.length,2);
  assert.equal(rows[0].children[1].children[0].textContent,'Ideed');assert.equal(document.getElementById('account-status').textContent,'My account');
  assert.equal(rows[0].querySelector('summary').getAttribute('aria-label'),'File actions');
  assert.equal(rows[0].querySelector('a').href,'/apps/gridcraft/?cloud_service=cloud&cloud_space=1&cloud_file=first&lang=en');
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.at(-1)[1].cursor,'third-page');
});

test('AI setup has its own route, survives language changes and stays out of file views',async()=>{
  const {host,document,calls}=fixture({search:'#ai',drive(method){if(method==='trash')return {entries:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('ai-view').hidden,false);
  assert.equal(document.getElementById('failid').hidden,true);
  assert.equal(document.getElementById('ai-nav').getAttribute('aria-current'),'page');
  assert.equal(calls.some(([method])=>['query','browse','list','trash'].includes(method)),false);
  const before=JSON.stringify(calls);host.i18n.setLanguage('en');
  assert.equal(document.getElementById('page-name').textContent,'Work with AI');
  assert.equal(JSON.stringify(calls),before);
  for(const hash of ['#failid','#jagatud','#prugikast','#rakendused']) {
    host.context.location.hash=hash;host.window.dispatchEvent(new Event('hashchange'));await tick();
    assert.equal(document.getElementById('ai-view').hidden,true);
    assert.notEqual(document.getElementById('ai-nav').getAttribute('aria-current'),'page');
  }
});

test('AI setup deep link is readable before sign-in and if cloud startup fails',async()=>{
  for(const options of [{account:{authenticated:false}},{unavailable:true}]) {
    const {host,document,calls}=fixture({search:'#ai',...options});
    await vm.runInContext(`(async()=>{${source}})()`,host.context);
    assert.equal(document.getElementById('ai-view').hidden,false);
    assert.equal(document.getElementById('ai-account').hidden,true);
    assert.equal(document.getElementById('ai-principal').textContent,'');
    assert.equal(calls.some(([method])=>['initialize','query','browse','list'].includes(method)),false);
  }
});

test('a pending file listing cannot replace AI setup after navigation',async()=>{
  let finish;
  const {host,document}=fixture({drive(method){if(method==='browse')return new Promise(resolve=>{finish=resolve;});}});
  const startup=vm.runInContext(`(async()=>{${source}})()`,host.context);await tick();
  host.context.location.hash='#ai';host.window.dispatchEvent(new Event('hashchange'));await tick();
  finish({folder:{id:'1',owner:'alice',name:'My files'},breadcrumbs:[],entries:[],cursor:null});await startup;
  assert.equal(document.getElementById('ai-view').hidden,false);
  assert.equal(document.getElementById('failid').hidden,true);
  assert.equal(document.getElementById('page-name').textContent,'Tööta AI-ga');
});

test('AI principal copy follows the current account and clears on sign-out',async()=>{
  const copied=[], options={search:'#ai',copy:value=>copied.push(value)};
  const {host,document}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const copy=document.getElementById('ai-copy-account');
  copy.dispatchEvent(new Event('click'));await tick();assert.deepEqual(copied,['alice']);
  options.account={principal:'agent'};host.window.dispatchEvent(new Event('storage'));await tick();
  assert.equal(document.getElementById('ai-principal').textContent,'agent');
  copy.dispatchEvent(new Event('click'));await tick();assert.deepEqual(copied,['alice','agent']);
  options.stale=true;copy.dispatchEvent(new Event('click'));await tick();assert.equal(copied.length,2);
  options.stale=false;options.account={authenticated:false};host.window.dispatchEvent(new Event('storage'));await tick();
  assert.equal(document.getElementById('ai-account').hidden,true);
  assert.equal(document.getElementById('ai-principal').textContent,'');
  copy.dispatchEvent(new Event('click'));await tick();assert.equal(copied.length,2);
});

test('new folder submits the selected parent and navigates through the canonical drive API',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('new-folder').dispatchEvent(new Event('click'));
  const input=document.getElementById('dialog-body').children[0].children[0];input.value='Test folder';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.find(c=>c[0]==='folder')[1],{parent:'1',name:'Test folder',requestId:'test-request-id-0001'});
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(calls.filter(([m])=>m==='query').at(-1)[0],'query');assert.equal(calls.filter(([m])=>m==='query').at(-1)[1].parent,'2');
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
  assert.equal(calls.length,before+1);assert.equal(calls.filter(([m])=>m==='query').at(-1)[0],'query');
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
  const existing=document.getElementById('file-list').children[0];
  input.dispatchEvent(new Event('change'));
  assert.equal(document.getElementById('file-list').children[0],existing);
  assert.equal(document.getElementById('file-area').getAttribute('aria-busy'),'false');
  assert.equal(findButton(existing,'Nimeta ümber').disabled,false);
  assert.equal(document.getElementById('transfer-status').hidden,false);
  assert.match(document.getElementById('transfer-label').textContent,/Ülekanded/);
  await tick();readBytes(new ArrayBuffer(1024));await tick();notify(1,2);
  assert.match(document.getElementById('transfer-items').textContent,/50%/);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  notify(2,2);assert.match(document.getElementById('transfer-items').textContent,/100%/);
  assert.equal(document.getElementById('transfer-status').hidden,false);
  commit({id:'99'});await tick();
  assert.equal(document.getElementById('transfer-status').hidden,false);
  assert.match(document.getElementById('transfer-items').textContent,/Valmis/);
  assert.equal(calls.find(c=>c[0]==='upload')[1].parent,'1');
  assert.equal(calls.filter(([method])=>method==='restore').length,1);
  assert.equal(calls.filter(([method])=>method==='account').length,1);
  document.getElementById('transfer-clear').dispatchEvent(new Event('click'));assert.equal(document.getElementById('transfer-status').hidden,true);
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


test('an unvisited folder waits for authoritative sorting instead of displaying an ID-ordered prefetch',async()=>{
  const child={id:'2',parent:'1',owner:'alice',folder:true,name:'Prefetched folder',capabilities:{read:true,create:true}};
  const file={id:'3',parent:'2',owner:'alice',name:'Already here.txt',folder:false,bytes:10,updatedMs:1700000000000,capabilities:{read:true}};
  let finish;
  const {host,document}=fixture({home:true,catalog:{entries:[child,file],completeFolders:['2']},drive(method,p){if(method==='browse' && p.parent==='2')return new Promise(r=>{finish=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));
  assert.equal(document.getElementById('file-list').children.length,0);
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
  assert.deepEqual(calls.find(([m])=>m==='download'),['download','first','44']);
  assert.deepEqual(calls.at(-1),['acknowledge',{id:'first',revision:'44'}]);
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
  const [first,second]=document.getElementById('file-list').children.map(row=>row.querySelector('details'));
  first.open=true;second.children[0].dispatchEvent(new Event('click'));second.open=true;
  assert.equal(first.open,false);assert.equal(second.open,true);
  const outside=new Event('click');Object.defineProperty(outside,'target',{value:new Element('main')});
  document.dispatchEvent(outside);assert.equal(second.open,false);
  first.open=true;const escape=new Event('keydown');escape.key='Escape';document.dispatchEvent(escape);
  assert.equal(first.open,false);
});

const pickerSearch=mode=>'?picker='+mode+'&app=gridcraft&channel=01234567-0123-0123-0123-0123456789ab&name=Shared';
test('editor pickers cannot navigate into AI setup',async()=>{
  const {host,document,calls}=fixture({search:pickerSearch('open')+'#ai'});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('ai-view').hidden,true);
  assert.equal(document.getElementById('failid').hidden,false);
  assert.equal(calls.some(([method])=>method==='query'),true);
});
test('shared Craft links open the same editor without requiring file ownership',async()=>{
  const {host,document}=fixture({file:{owner:'bob'}});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.match(document.getElementById('file-list').children[0].querySelector('a').href,/cloud_file=first/);
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
  const root={id:'1',spaceId:'1',owner:'bob',folder:true,capabilities:{read:true,create:true,edit:false}};
  const {host,document,selections,calls}=fixture({search:pickerSearch('save'),root,drive(method){if(method==='get')return root;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('picker-submit').disabled,false);
  document.getElementById('picker-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(selections[0].data.parent,'1');assert.equal(selections[0].data.name,'Shared.scjson');
  assert.equal(calls.filter(c=>c[0]==='upload').length,0);
});
test('Save As revalidates revoked Create and discards account-stale selections',async()=>{
  const root={id:'1',spaceId:'1',owner:'bob',folder:true,capabilities:{read:true,create:true}};
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

test('refresh coalesces requests and preserves unchanged rows and every loaded page without restoring identity',async()=>{
  const {host,document,calls}=fixture();
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  const rows=[...document.getElementById('file-list').children],before=calls.length;
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));
  assert.deepEqual(document.getElementById('file-list').children,rows);
  assert.equal(document.getElementById('upload-file').disabled,false);
  assert.equal(document.getElementById('refresh-files').disabled,true);
  assert.equal(document.getElementById('file-area').getAttribute('aria-busy'),'false');
  await tick();
  assert.deepEqual(document.getElementById('file-list').children,rows);
  assert.deepEqual(calls.slice(before).map(([method,p])=>[method,p.cursor]),[['query',null],['query','second-page']]);
  assert.equal(calls.filter(([method])=>method==='restore').length,1);
  assert.equal(document.getElementById('upload-file').disabled,false);
});

test('refresh applies changed permissions and discards late replies after navigation',async()=>{
  let finish,hold=false;
  const {host,document}=fixture({drive(method,p){if(hold && method==='browse' && p.parent==='1')return new Promise(r=>{finish=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  hold=true;document.getElementById('refresh-files').dispatchEvent(new Event('click'));
  finish({folder:{id:'1',owner:'alice',folder:true,name:'My files',capabilities:{read:true,create:false}},breadcrumbs:[],entries:[],cursor:null});await tick();
  assert.equal(document.getElementById('file-drop-hint').hidden,true);
  assert.equal(document.getElementById('upload-file').disabled,true);
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));
  host.context.location.hash='#folder/3';host.window.dispatchEvent(new Event('hashchange'));await tick();
  finish({folder:{id:'1',owner:'alice',name:'Old reply',capabilities:{}},breadcrumbs:[],entries:[],cursor:null});await tick();
  assert.equal(document.getElementById('files-title').textContent,'Nested');
});

test('drop feedback survives child crossings and multiple files use the current shared Create-only folder',async()=>{
  const options={drive(method,p){if(method==='browse' && p.parent==='2')return {folder:{id:'2',owner:'bob',folder:true,name:'Shared artwork',capabilities:{read:true,create:true,edit:false}},breadcrumbs:[],entries:[],cursor:null};}};
  const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  const area=document.getElementById('file-area'),overlay=document.getElementById('file-drop-overlay');
  area.dispatchEvent(fileDrag('dragenter'));area.dispatchEvent(fileDrag('dragenter'));area.dispatchEvent(fileDrag('dragleave'));
  assert.equal(overlay.hidden,false);assert.equal(document.getElementById('file-drop-target').textContent,'Shared artwork');
  const over=fileDrag('dragover');area.dispatchEvent(over);assert.equal(over.dataTransfer.dropEffect,'copy');
  const files=['one.txt','two.scjson'].map(name=>({name,size:3,type:'text/plain',arrayBuffer:async()=>new ArrayBuffer(3)}));
  area.dispatchEvent(fileDrag('drop',files));assert.equal(overlay.hidden,true);await tick();
  const uploaded=calls.filter(([method])=>method==='upload').map(([,p])=>p);
  assert.equal(uploaded.length,2);assert.ok(uploaded.every(p=>p.parent==='2'));
  assert.equal(uploaded[1].craft.format_id,'grid');
});

test('drops cannot upload while signed out, read-only, at shared root, in trash, in pickers or with a stale account',async()=>{
  for(const options of [
    {account:{authenticated:false}}, {root:{capabilities:{read:true,create:false}}},
    {search:'#jagatud'}, {search:'#prugikast',drive(method){if(method==='trash')return {entries:[],cursor:null};}},
    {search:'?picker=save&app=gridcraft&channel=00000000-0000-0000-0000-000000000000#failid'}, {},
  ]) {
    const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
    if(!Object.keys(options).length)options.stale=true;
    const event=fileDrag('drop',[{name:'blocked.txt',size:1,arrayBuffer:async()=>new ArrayBuffer(1)}]);
    document.getElementById('file-area').dispatchEvent(event);await tick();
    assert.equal(event.defaultPrevented,true);assert.ok(!calls.some(([method])=>method==='upload'));
    assert.equal(document.getElementById('file-drop-overlay').hidden,true);
  }
});

test('drops preserve empty directories, leave text drags alone and prevent file navigation outside the area',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const area=document.getElementById('file-area');
  const textDrag=fileDrag('drop');textDrag.dataTransfer.types=['text/plain'];area.dispatchEvent(textDrag);
  assert.equal(textDrag.defaultPrevented,false);
  area.dispatchEvent(fileDrag('drop',[],[{kind:'file',webkitGetAsEntry:()=>({name:'Empty',isDirectory:true,createReader:()=>({readEntries:resolve=>resolve([])})})}]));
  await tick();assert.equal(calls.find(([method])=>method==='folder')[1].name,'Empty');
  const outside=fileDrag('drop');document.dispatchEvent(outside);assert.equal(outside.defaultPrevented,true);
  area.dispatchEvent(fileDrag('dragenter'));assert.equal(document.getElementById('file-drop-overlay').hidden,false);
  host.window.dispatchEvent(new Event('blur'));assert.equal(document.getElementById('file-drop-overlay').hidden,true);
  assert.ok(!calls.some(([method])=>method==='upload'));
});

test('active drags pause refresh and a newly opened folder blocks drops until validated',async()=>{
  let hold=false,finish;
  const {host,document,calls}=fixture({drive(method){if(hold && method==='browse')return new Promise(r=>{finish=r;});}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const area=document.getElementById('file-area'),refresh=document.getElementById('refresh-files');
  area.dispatchEvent(fileDrag('dragenter'));const before=calls.length;
  refresh.dispatchEvent(new Event('click'));assert.equal(calls.length,before);
  area.dispatchEvent(fileDrag('dragleave'));hold=true;
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));
  area.dispatchEvent(fileDrag('drop',[{name:'blocked.txt',size:1,arrayBuffer:async()=>new ArrayBuffer(1)}]));
  assert.ok(!calls.some(([method])=>method==='upload'));
  finish({folder:{id:'2',owner:'alice',folder:true,name:'Nested',capabilities:{read:true,create:true}},breadcrumbs:[],entries:[],cursor:null});await tick();
  assert.equal(document.getElementById('upload-file').disabled,false);
});

test('returning focus gives the initiating click precedence over automatic refresh',async()=>{
  let now=10000,resume;
  const {host,document,calls}=fixture();
  host.context.Date=class extends Date {static now(){return now;}};
  host.context.setTimeout=fn=>{resume=fn;return 1;};host.context.clearTimeout=()=>{};
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  now+=6000;const before=calls.length;host.window.dispatchEvent(new Event('focus'));
  assert.equal(calls.length,before);assert.equal(document.getElementById('new-folder').disabled,false);
  document.getElementById('new-folder').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('drive-dialog').open,true);
  resume();await tick();assert.equal(calls.length,before);
});

function deferred() {let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};}
function resourceFixture(overrides={}) {
  const root={id:'1',spaceId:'1',owner:'alice',folder:true,name:'My files',capabilities:{read:true,create:true,edit:true,delete:true}};
  const state={items:['Alpha.txt','Bravo.txt','Charlie.txt'].map((name,index)=>({...trashEntry(String(index+2)),parent:'1',name,version:'1',revision:'44',capabilities:{read:true,edit:true,delete:true}})),trash:[],root};
  const page=p=>({folder:{...root,id:p.parent},breadcrumbs:[root],entries:state.items.filter(item=>item.parent===p.parent).slice(p.cursor ? 2 : 0,p.cursor ? 4 : 2).map(item=>({...item})),cursor:p.cursor || state.items.filter(item=>item.parent===p.parent).length<=2 ? null : 'next'});
  const options={...overrides,drive(method,p){
    const result=overrides.drive?.(method,p,state,page);if(result!==undefined)return result;
    if(method==='browse')return page(p);
    if(method==='get')return state.items.find(item=>item.id===p.id);
    if(method==='trash')return {entries:state.trash.map(item=>({...item})),cursor:null};
    if(method==='list')return {entries:[root,{...root,id:'9',name:'Destination'}],cursor:null};
    if(method==='folder') {const item={...root,id:'8',parent:p.parent,name:p.name};state.items.push(item);return item;}
    if(method==='organize') {
      const item=[...state.items,...state.trash].find(item=>item.id===p.id);item.version=String(Number(item.version)+1);
      if(p.change==='rename')item.name=p.name;
      if(p.change==='move')item.parent=p.parent;
      if(p.change==='trash'){state.items=state.items.filter(entry=>entry!==item);state.trash.push(item);}
      if(p.change==='restore'){state.trash=state.trash.filter(entry=>entry!==item);state.items.push(item);}
      return {id:item.id,version:item.version};
    }
  }};
  const result=fixture(options),{document}=result;
  const row=id=>document.getElementById('file-list').children.find(row=>row.dataset.fileId===id);
  const click=(id,label)=>findButton(row(id),label).dispatchEvent(new Event('click'));
  const submit=()=>document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));
  const rename=(id,name)=>{click(id,'Nimeta ümber');document.getElementById('dialog-body').children[0].children[0].value=name;submit();};
  return {...result,state,options,row,click,submit,rename};
}

test('rename keeps other rows, an open menu, focus and loaded pages while only its resource is busy',async()=>{
  const commit=deferred(),verify=deferred();let mutation=false;
  const {host,document,calls,row,rename,state}=resourceFixture({drive(method,p,state,page){
    if(method==='organize') {mutation=true;return commit.promise.then(()=>{state.items[0]={...state.items[0],name:p.name,version:'2'};return {id:p.id,version:'2'};});}
    if(method==='browse' && mutation && !p.cursor)return verify.promise.then(()=>page(p));
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  const first=row('2'),second=row('3'),third=row('4');
  rename('2','Zulu.txt');
  assert.equal(document.getElementById('drive-dialog').open,false);
  assert.equal(first.getAttribute('aria-busy'),'true');assert.equal(second.getAttribute('aria-busy'),'false');
  assert.equal(document.getElementById('file-area').getAttribute('aria-busy'),'false');
  assert.equal(findButton(second,'Nimeta ümber').disabled,false);assert.equal(document.getElementById('new-folder').disabled,false);
  second.fileMenu.open=true;document.activeElement=second.fileMenu.children[0];
  host.i18n.setLanguage('en');assert.equal(first.operationStatus.textContent,'Renaming…');
  commit.resolve();await tick();assert.equal(row('2'),first);assert.equal(first.getAttribute('aria-busy'),'true');
  verify.resolve();await tick();
  assert.notEqual(row('2'),first);assert.equal(row('3'),second);assert.equal(row('4'),third);
  assert.equal(second.fileMenu.open,true);assert.equal(document.activeElement,second.fileMenu.children[0]);
  assert.equal(row('2').getAttribute('aria-busy'),'false');assert.equal(state.items[0].name,'Zulu.txt');
  assert.equal(document.getElementById('file-list').children.length,3);
  assert.equal(calls.filter(([method])=>method==='restore').length,1);
});

test('overlapping mutations reconcile even when one fails, and a previous completion leaves a newer dialog intact',async()=>{
  const first=deferred(),second=deferred(),oldRead=deferred();let holdRead=false;
  const f=resourceFixture({drive(method,p,state,page){
    if(method==='organize')return (p.id==='2' ? first : second).promise.then(()=>{const file=state.items.find(file=>file.id===p.id);file.name=p.name;file.version='2';return {id:p.id,version:'2'};});
    if(method==='browse' && holdRead){holdRead=false;return oldRead.promise.then(()=>page(p));}
  }});
  const {host,document,row,rename}=f;await vm.runInContext(`(async()=>{${source}})()`,host.context);
  rename('2','Alpha changed.txt');holdRead=true;first.resolve();await tick();
  rename('3','Bravo changed.txt');assert.equal(row('3').getAttribute('aria-busy'),'true');
  second.reject(new Error('Temporary rename failure'));await tick();
  assert.match(row('2').textContent,/Alpha changed.txt/);assert.match(row('3').textContent,/Bravo.txt/);
  assert.equal(document.getElementById('drive-dialog').open,true);
  assert.equal(document.getElementById('dialog-error').textContent,'Temporary rename failure');
  oldRead.resolve();await tick();
  assert.equal(document.getElementById('drive-dialog').open,true);assert.equal(document.getElementById('dialog-error').textContent,'Temporary rename failure');
  assert.equal(row('2').getAttribute('aria-busy'),'false');assert.equal(row('3').getAttribute('aria-busy'),'false');
  document.getElementById('dialog-cancel').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('upload-file').disabled,false);
});

test('a late refresh cannot undo an acknowledged rename, and a network error retains the current list',async()=>{
  const pending=deferred();let hold=false,fail=false;
  const {host,document,row,rename}=resourceFixture({drive(method,p,state,page){
    if(method==='browse' && hold){hold=false;const before=page(p);return pending.promise.then(()=>before);}
    if(method==='browse' && fail)throw new Error('Network unavailable');
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  hold=true;document.getElementById('refresh-files').dispatchEvent(new Event('click'));
  rename('2','Updated.txt');await tick();const updated=row('2'),unrelated=row('3');
  assert.match(updated.textContent,/Updated.txt/);
  pending.resolve();await tick();assert.equal(row('2'),updated);assert.equal(row('3'),unrelated);
  fail=true;document.getElementById('refresh-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(row('2'),updated);assert.equal(row('3'),unrelated);
  assert.equal(document.getElementById('file-area').getAttribute('aria-busy'),'false');
  assert.equal(document.getElementById('account-message').textContent,'Network unavailable');
});

test('move, trash and restore update only affected rows without a session reload',async()=>{
  const {host,document,calls,row,click,submit}=resourceFixture();
  await vm.runInContext(`(async()=>{${source}})()`,host.context);const unchanged=row('3');
  click('2','Liiguta');await tick();findButton(document.getElementById('dialog-body'),'Destination').dispatchEvent(new Event('click'));await tick();submit();await tick();
  assert.equal(row('2'),undefined);assert.equal(row('3'),unchanged);assert.ok(row('4'));
  click('4','Prügikasti');submit();await tick();assert.equal(row('4'),undefined);assert.equal(row('3'),unchanged);
  host.context.location.hash='#prugikast';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.ok(row('4'));click('4','Taasta');await tick();assert.equal(row('4'),undefined);
  assert.equal(document.getElementById('file-status').textContent,'Prügikast on tühi.');
  assert.equal(calls.filter(([method])=>method==='restore').length,1);
});

test('new folders and cross-tab file updates merge into the existing list without restoring identity',async()=>{
  const {host,document,calls,row,state,submit}=resourceFixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const existing=row('2');document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  document.getElementById('new-folder').dispatchEvent(new Event('click'));
  document.getElementById('dialog-body').children[0].children[0].value='New folder';submit();await tick();
  assert.equal(row('2'),existing);assert.ok(row('8'));assert.equal(document.getElementById('new-folder').disabled,false);
  state.items.find(file=>file.id==='3').name='Changed elsewhere.txt';
  const event=new Event('storage');event.key='subnet.files.changed';host.window.dispatchEvent(event);await tick();
  assert.match(row('3').textContent,/Changed elsewhere/);assert.equal(row('2'),existing);
  assert.equal(calls.filter(([method])=>method==='restore').length,1);
});

test('pending mutations cannot publish into a switched account or navigate back to their original folder',async()=>{
  for(const switchAccount of [false,true]) {
    const commit=deferred();const {host,document,calls,row,rename,options}=resourceFixture({drive(method){if(method==='organize')return commit.promise;}});
    await vm.runInContext(`(async()=>{${source}})()`,host.context);rename('2','Old account rename.txt');
    if(switchAccount){options.account={principal:'bob',usedBytes:4321};host.window.dispatchEvent(new Event('storage'));await tick();}
    else {host.context.location.hash='#folder/9';host.window.dispatchEvent(new Event('hashchange'));await tick();}
    const before=calls.length;commit.resolve({id:'2',version:'2'});await tick();
    assert.equal(document.getElementById('drive-dialog').open,false);
    if(switchAccount){assert.equal(calls.length,before);assert.equal(document.getElementById('account-principal').textContent,'bob');}
    else {assert.equal(host.context.location.hash,'#folder/9');assert.equal(row('2'),undefined);}
  }
});

test('loading the next page leaves files usable and a stale page cannot append after a mutation',async()=>{
  let hold=false;const pending=deferred();
  const {host,document,row,rename}=resourceFixture({drive(method,p,state,page){
    if(method==='browse' && p.cursor && hold){hold=false;const before=page(p);return pending.promise.then(()=>before);}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  hold=true;document.getElementById('more-files').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('more-files').disabled,true);
  assert.equal(document.getElementById('file-area').getAttribute('aria-busy'),'false');
  assert.equal(findButton(row('2'),'Nimeta ümber').disabled,false);
  rename('2','Renamed.txt');await tick();pending.resolve();await tick();
  assert.match(row('2').textContent,/Renamed.txt/);
  assert.equal(document.getElementById('file-list').children.length,2);
  assert.equal(document.getElementById('more-files').disabled,false);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('file-list').children.length,3);
});

function clickEvent(type='click',detail=1) {const event=new Event(type,{cancelable:true});event.detail=detail;return event;}
function keyEvent(key,composing=false) {const event=new Event('keydown',{cancelable:true});event.key=key;event.isComposing=composing;return event;}
test('single click selects, F2 selects the basename and Enter saves only that resource',async()=>{
  const {host,document,calls,row}=resourceFixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const original=row('2'),other=row('3');original.dispatchEvent(clickEvent());
  assert.equal(original.getAttribute('aria-selected'),'true');assert.equal(original.querySelector('input.file-name-input'),null);
  original.dispatchEvent(keyEvent('F2'));const input=original.querySelector('input.file-name-input');
  assert.equal(input.value,'Alpha.txt');assert.equal(input.selectionEnd,5);
  input.value='New name.txt';input.dispatchEvent(new Event('input'));host.i18n.setLanguage('en');
  assert.equal(input.value,'New name.txt');assert.equal(input.getAttribute('aria-label'),'File name');
  input.dispatchEvent(keyEvent('Enter',true));assert.equal(calls.some(([method])=>method==='organize'),false);
  input.dispatchEvent(keyEvent('Enter'));input.dispatchEvent(new Event('blur'));await tick();
  assert.equal(calls.filter(([method])=>method==='organize').length,1);
  assert.match(row('2').textContent,/New name.txt/);assert.equal(row('3'),other);
});

test('double click opens and records Recent without renaming',async()=>{
  const {host,document,calls,row}=resourceFixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  row('2').dispatchEvent(clickEvent());row('2').dispatchEvent(clickEvent('dblclick',2));await tick();
  assert.equal(row('2').querySelector('input.file-name-input'),null);assert.equal(document.getElementById('preview-dialog').open,true);
  assert.equal(calls.filter(([method])=>method==='get').length,1);
  assert.equal(calls.filter(([method])=>method==='download').length,1);assert.equal(calls.some(([method])=>method==='organize'),false);
});

test('inline rename preserves drafts on error and supports Escape and retry',async()=>{
  let fail=true;const {host,document,calls,row}=resourceFixture({drive(method){if(method==='organize' && fail)throw new Error('Rename temporarily unavailable');}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const begin=()=>{row('2').dispatchEvent(keyEvent('F2'));return row('2').querySelector('input.file-name-input');};
  let input=begin();input.value='Cancelled.txt';input.dispatchEvent(keyEvent('Escape'));input.dispatchEvent(new Event('blur'));await tick();
  assert.equal(row('2').querySelector('input.file-name-input'),null);assert.equal(calls.some(([method])=>method==='organize'),false);
  assert.equal(document.activeElement,row('2'));
  input=begin();input.value='';input.dispatchEvent(keyEvent('Enter'));await tick();assert.equal(calls.some(([method])=>method==='organize'),false);
  input.value='Retry.txt';input.dispatchEvent(new Event('input'));input.dispatchEvent(new Event('blur'));await tick();
  assert.equal(row('2').querySelector('input.file-name-input'),input);assert.equal(input.value,'Retry.txt');assert.equal(input.disabled,false);
  fail=false;input.dispatchEvent(keyEvent('Enter'));await tick();
  const requests=calls.filter(([method])=>method==='organize');assert.equal(requests.length,2);assert.deepEqual(requests[0],requests[1]);assert.match(row('2').textContent,/Retry.txt/);
});

test('read-only files, Trash and pickers cannot inline rename',async()=>{
  for(const options of [{file:{capabilities:{read:true}}},{search:pickerSearch('open')},{search:'#prugikast',drive(method){if(method==='trash')return {entries:[trashEntry('7')],cursor:null};}}]) {
    const {host,document}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
    const row=document.getElementById('file-list').children[0];row.dispatchEvent(keyEvent('F2'));assert.equal(row.querySelector('input.file-name-input'),null);
  }
});

test('folder F2 selects the whole name and double click navigates without recording a file open',async()=>{
  const {host,document,calls,row,state}=resourceFixture();state.items[0]={...state.items[0],folder:true,name:'Project.2026'};
  await vm.runInContext(`(async()=>{${source}})()`,host.context);row('2').dispatchEvent(keyEvent('F2'));
  const input=row('2').querySelector('input.file-name-input');assert.equal(input.selectionEnd,'Project.2026'.length);
  input.dispatchEvent(keyEvent('Escape'));row('2').dispatchEvent(clickEvent('dblclick',2));
  assert.equal(host.context.location.hash,'#folder/2');host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(calls.filter(([m])=>m==='query').at(-1)[0],'query');assert.equal(calls.filter(([m])=>m==='query').at(-1)[1].parent,'2');assert.equal(calls.some(([method])=>method==='open'),false);
});

test('range, toggle, select-all-loaded and Escape share the visible order',async()=>{
  const {host,document,row}=resourceFixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  row('2').dispatchEvent(clickEvent());let event=clickEvent();event.shiftKey=true;row('3').dispatchEvent(event);
  assert.equal(row('2').getAttribute('aria-selected'),'true');assert.equal(row('3').getAttribute('aria-selected'),'true');
  event=clickEvent();event.metaKey=true;row('2').dispatchEvent(event);assert.equal(row('2').getAttribute('aria-selected'),'false');
  event=keyEvent('a');event.ctrlKey=true;row('3').dispatchEvent(event);assert.equal(document.getElementById('selection-count').textContent,'2 valitud');
  row('3').dispatchEvent(keyEvent('Escape'));assert.equal(document.getElementById('selection-toolbar').hidden,true);
});

test('switching list/grid starts and clears thumbnails even when metadata is unchanged',async()=>{
  const {host,document,calls}=fixture({file:{name:'picture.png',format:{media_type:'image/png'},bytes:200}});
  const observers=[];
  host.context.IntersectionObserver=class {
    constructor(callback){this.callback=callback;observers.push(this);}
    observe(){}
    disconnect(){this.disconnected=true;}
  };
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(observers.length,0);
  document.getElementById('view-grid').dispatchEvent(new Event('click'));await tick();
  assert.equal(observers.length,1);
  observers[0].callback([{isIntersecting:true}]);await tick();
  assert.equal(calls.filter(([method])=>method==='download').length,1);
  assert.equal(document.getElementById('file-list').querySelectorAll('.file-thumbnail').length,1);
  document.getElementById('view-list').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('file-list').querySelectorAll('.file-thumbnail').length,0);
});

test('search routes, private views and preferences use the server query and survive language changes',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('search-text').value='Report';document.getElementById('search-type').value='text';document.getElementById('search-owner').value='mine';
  document.getElementById('drive-search').dispatchEvent(new Event('submit',{cancelable:true}));host.window.dispatchEvent(new Event('hashchange'));await tick();
  const q=calls.filter(([method])=>method==='query').at(-1)[1];assert.equal(q.text,'Report');assert.equal(q.fileType,'text');assert.equal(q.owner,'alice');assert.equal(q.scope,'all');
  host.i18n.setLanguage('en');assert.equal(document.getElementById('files-title').textContent,'Search results');
  document.getElementById('view-grid').dispatchEvent(new Event('click'));await tick();assert.equal(document.getElementById('view-grid').getAttribute('aria-pressed'),'true');
  assert.equal(calls.find(([method])=>method==='preferences')[1].view,'grid');
  document.getElementById('sort-field').value='size';document.getElementById('sort-field').dispatchEvent(new Event('change'));await tick();
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].sort,'size');
  for(const [hash,scope] of [['#tarniga','starred'],['#hiljutised','recent']]) {
    host.context.location.hash=hash;host.window.dispatchEvent(new Event('hashchange'));await tick();assert.equal(calls.at(-1)[1].scope,scope);
  }
  assert.equal(calls.at(-1)[1].sort,'opened');assert.equal(calls.at(-1)[1].descending,true);
});

test('bulk stars report partial failure and preserve other item outcomes',async()=>{
  const {host,document,calls,row}=resourceFixture({drive(method,p){if(method==='star'){if(p.id==='3')throw new Error('Access revoked');return null;}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-star').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([method])=>method==='star').length,2);assert.match(document.getElementById('account-message').textContent,/Bravo.txt: Access revoked/);
  assert.equal(row('2').getAttribute('aria-selected'),'true');
});

test('changing view and sort preserves selection, while navigation clears it',async()=>{
  const {host,document,row}=resourceFixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  row('2').dispatchEvent(clickEvent());
  document.getElementById('view-grid').dispatchEvent(new Event('click'));await tick();
  assert.equal(row('2').getAttribute('aria-selected'),'true');
  document.getElementById('sort-field').value='modified';document.getElementById('sort-field').dispatchEvent(new Event('change'));await tick();
  assert.equal(row('2').getAttribute('aria-selected'),'true');
  host.context.location.hash='#folder/9';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(document.getElementById('selection-toolbar').hidden,true);
});

test('a late search response cannot replace a different personal view',async()=>{
  let finish;
  const {host,document}=fixture({query(p){if(p.scope==='all')return new Promise(resolve=>{finish=resolve;});return {entries:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  host.context.location.hash='#search?q=private';host.window.dispatchEvent(new Event('hashchange'));await tick();
  host.context.location.hash='#tarniga';host.window.dispatchEvent(new Event('hashchange'));await tick();
  finish({entries:[trashEntry('late')],cursor:null});await tick();
  assert.equal(document.getElementById('files-title').textContent,'Tärniga');assert.equal(document.getElementById('file-list').children.length,0);
});

test('rapid sort preference writes settle in order and only the latest choice reloads',async()=>{
  const first=deferred();let writes=0;
  const {host,document,calls}=fixture({drive(method){if(method==='preferences' && ++writes===1)return first.promise;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const select=document.getElementById('sort-field');select.value='size';select.dispatchEvent(new Event('change'));await tick();
  select.value='modified';select.dispatchEvent(new Event('change'));first.resolve(null);await tick();await tick();
  assert.deepEqual(calls.filter(([method])=>method==='preferences').map(([,p])=>p.sort),['size','modified']);
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].sort,'modified');
});

test('a layout change while sorting is saved applies both choices',async()=>{
  const first=deferred();let writes=0;
  const {host,document,calls}=fixture({drive(method){if(method==='preferences' && ++writes===1)return first.promise;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const select=document.getElementById('sort-field');select.value='size';select.dispatchEvent(new Event('change'));await tick();
  document.getElementById('view-grid').dispatchEvent(new Event('click'));
  assert.equal(select.value,'size');
  first.resolve(null);await tick();await tick();
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].sort,'size');
  assert.equal(document.getElementById('view-grid').getAttribute('aria-pressed'),'true');
});

test('selection toolbar replaces controls and only offers actions valid for the whole selection',async()=>{
  const {host,document,row,state}=resourceFixture();
  state.items[1].owner='bob';state.items[1].capabilities={read:true};
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const first=row('2');first.dispatchEvent(clickEvent());
  assert.equal(document.getElementById('selection-rename').disabled,false);
  assert.equal(document.getElementById('selection-share').disabled,false);
  assert.equal(document.getElementById('browse-controls').inert,true);
  const event=clickEvent();event.metaKey=true;row('3').dispatchEvent(event);
  for(const action of ['rename','share','move','trash'])assert.equal(document.getElementById('selection-'+action).disabled,true);
  assert.equal(document.getElementById('selection-download').disabled,false);
  document.getElementById('selection-trash').dispatchEvent(new Event('click'));
  assert.equal(!!document.getElementById('drive-dialog').open,false);
  assert.equal(row('2'),first);
  document.getElementById('selection-clear').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('browse-controls').inert,false);
  assert.equal(document.getElementById('selection-toolbar').hidden,true);
});

test('bulk Trash retries only failed items, then restores the selected items',async()=>{
  let fail=true;
  const {host,document,calls,row,submit,state}=resourceFixture({drive(method,p){if(method==='organize' && p.id==='3' && fail)throw new Error('Try again');}});
  let token=0;host.context.crypto.randomUUID=()=>`batch-request-${++token}`;
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-trash').dispatchEvent(new Event('click'));
  assert.match(document.getElementById('dialog-body').textContent,/Alpha.txt.*Bravo.txt/);
  assert.equal(calls.some(([method])=>method==='organize'),false);
  submit();await tick();
  assert.equal(row('2'),undefined);assert.equal(row('3').getAttribute('aria-selected'),'true');
  assert.match(document.getElementById('dialog-error').textContent,/Bravo.txt: Try again/);
  fail=false;submit();await tick();
  const requests=calls.filter(([method])=>method==='organize').map(([,payload])=>payload);
  assert.deepEqual(requests.map(p=>p.id),['2','3','3']);assert.deepEqual(requests[1],requests[2]);
  assert.notEqual(requests[0].requestId,requests[1].requestId);
  host.context.location.hash='#prugikast';host.window.dispatchEvent(new Event('hashchange'));await tick();
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('selection-restore').hidden,false);
  document.getElementById('selection-restore').dispatchEvent(new Event('click'));await tick();
  assert.equal(state.trash.length,0);assert.equal(state.items.length,3);
});

test('bulk moves use one destination and preserve optimistic versions per file',async()=>{
  const {host,document,calls,submit,state}=resourceFixture();
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-move').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('dialog-submit').disabled,true);
  findButton(document.getElementById('dialog-body'),'Destination').dispatchEvent(new Event('click'));await tick();submit();await tick();
  const moves=calls.filter(([method])=>method==='organize').map(([,p])=>p);
  assert.deepEqual(moves.map(p=>[p.id,p.parent,p.version]),[['2','9','1'],['3','9','1']]);
  assert.equal(state.items.filter(file=>file.parent==='9').length,2);
});

test('selected ancestors cover selected descendants through unloaded intermediate folders',async()=>{
  const {host}=fixture();const roots=vm.runInContext('selectionRoots',host.context);
  const files=[{id:'child',parent:'middle'},{id:'folder',parent:'private'},{id:'other',parent:null}];const reads=[];
  const plan=await roots(files,async id=>{reads.push(id);return id==='middle'?{id,parent:'folder'}:null;});
  assert.deepEqual(JSON.parse(JSON.stringify(plan)),[{file:files[1],ids:['child','folder']},{file:files[2],ids:['other']}]);
  assert.equal(reads.filter(id=>id==='middle').length,1);
});

test('sharing offers the same two usable roles for accounts and email, preserving existing custom grants',async()=>{
  const custom={id:'9',root:'first',version:'3',subject:'bob',active:true,expiresAt:null,capabilities:{read:true,edit:true,create:false,delete:false,propose:false}};
  const {host,document,calls}=fixture({drive(method){if(method==='grants')return [custom];if(method==='grant')return {};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Jaga').dispatchEvent(new Event('click'));await tick();
  const body=document.getElementById('dialog-body'),roles=body.querySelectorAll('.sharing-role');
  assert.deepEqual(roles[0].children.map(o=>o.value),['viewer','editor']);
  assert.equal(body.querySelectorAll('input').some(input=>input.type==='checkbox'),false);
  assert.equal(roles[1].value,'advanced');assert.equal(roles[1].children[0].disabled,true);
  assert.equal(calls.some(([m])=>m==='grant'),false);
  roles[1].value='viewer';roles[1].dispatchEvent(new Event('change'));await tick();
  const grant=calls.find(([m])=>m==='grant')[1];
  assert.deepEqual(grant.capabilities,{read:true,propose:false,create:false,edit:false,delete:false});
});

test('bulk share applies the entered principal and capabilities to each selected object',async()=>{
  const {host,document,calls,submit,state}=resourceFixture({drive(method){if(method==='grant')return {};}});
  state.items[0].folder=true;
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-share').dispatchEvent(new Event('click'));
  const inputs=document.getElementById('dialog-body').querySelectorAll('input');
  inputs[0].value='recipient';const role=document.getElementById('dialog-body').querySelector('select');role.value='editor';role.dispatchEvent(new Event('change'));submit();await tick();
  const grants=calls.filter(([method])=>method==='grant').map(([,p])=>p);
  assert.deepEqual(grants.map(p=>[p.root,p.subject,p.capabilities.create,p.capabilities.edit]),[['2','recipient',true,true],['3','recipient',false,true]]);
});

test('changing accounts during a batch prevents any remaining operations',async()=>{
  const pending=deferred();
  const {host,document,calls,options}=resourceFixture({drive(method){if(method==='star')return pending.promise;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-star').dispatchEvent(new Event('click'));await tick();
  options.account={principal:'bob'};host.window.dispatchEvent(new Event('storage'));await tick();
  pending.resolve(null);await tick();
  assert.equal(calls.filter(([method])=>method==='star').length,1);
});

test('a ZIP download keeps per-file errors and releases resource locks',async()=>{
  const {host,document,calls,urls}=resourceFixture({download(id){if(id==='3')throw new Error('Download revoked');return new Uint8Array([1,2]);}});
  const create=document.createElement;document.createElement=tag=>{const node=create(tag);node.click=()=>node.dispatchEvent(new Event('click'));return node;};
  host.context.setTimeout=(fn,ms)=>{const timer=setTimeout(fn,ms);timer.unref();return timer;};
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-download').dispatchEvent(new Event('click'));await tick();
  assert.deepEqual(calls.filter(([method])=>method==='download').map(([,id])=>id),['2','3']);
  assert.equal(urls.filter(([method])=>method==='create').length,1);
  assert.equal(urls.find(([method])=>method==='create')[1].type,'application/zip');
  assert.match(document.getElementById('account-message').textContent,/Bravo.txt: Download revoked/);
  assert.equal(document.getElementById('selection-download').disabled,false);
});

test('an account change cancels a pending archive and prevents subsequent downloads',async()=>{
  const pending=deferred(),options={download:()=>pending.promise};
  const {host,document,calls,urls}=resourceFixture(options);
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('selection-download').dispatchEvent(new Event('click'));await tick();
  options.account={principal:'bob'};host.window.dispatchEvent(new Event('storage'));await tick();
  pending.resolve(new Uint8Array([1,2]));await tick();
  assert.equal(calls.filter(([method])=>method==='download').length,1);
  assert.equal(urls.length,0);
});

test('the context menu on a selected group keeps the group and single-file double click still opens',async()=>{
  const {host,document,calls,row}=resourceFixture();
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  row('2').dispatchEvent(clickEvent());let event=clickEvent();event.shiftKey=true;row('3').dispatchEvent(event);
  row('3').dispatchEvent(new Event('contextmenu',{cancelable:true}));
  assert.equal(document.getElementById('selection-more').open,true);
  assert.equal(document.getElementById('selection-count').textContent,'2 valitud');
  row('2').dispatchEvent(clickEvent());row('2').dispatchEvent(clickEvent('dblclick',2));await tick();
  assert.equal(calls.filter(([method])=>method==='get').length,1);
});

test('Home is distinct from Recent and displays service suggestions and truthful reasons',async()=>{
  const recent={...trashEntry('recent'),name:'Opened.txt',openedMs:1700000001000};
  const {host,document,calls}=fixture({search:'#kodu',suggestions(){
    return {entries:[{...recent,reason:'opened'}],quickFolders:[{...trashEntry('folder'),folder:true,name:'Projects',reason:'starred'}],cursor:null};
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('files-title').textContent,'Tere tulemast pilveruumi');
  assert.equal(document.getElementById('home-folder-list').children[0].href,'/pilveruum/#folder/folder');
  assert.match(document.getElementById('file-list').textContent,/Opened.txt/);
  assert.equal(calls.some(([method])=>method==='suggestions'),true);
  assert.equal(calls.some(([method,p])=>method==='query' && p.scope==='recent'),false);
  assert.match(document.getElementById('home-folder-list').textContent,/Tärniga/);
  host.i18n.setLanguage('en');assert.equal(document.getElementById('files-title').textContent,'Welcome to Cloud Space');
  host.context.location.hash='#failid';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(document.getElementById('home-folders').hidden,true);assert.equal(document.getElementById('files-title').textContent,'My files');
  assert.equal(vm.runInContext("browseRoute('', '1', 'alice', defaultPreferences()).view",host.context),'home');
});

test('New on Home creates in the account root and exposes the registered apps',async()=>{
  const {host,document,calls}=fixture({search:'#kodu',query(){return {entries:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('new-menu-folder').disabled,false);
  document.getElementById('new-menu-folder').dispatchEvent(new Event('click'));
  document.getElementById('dialog-body').querySelector('input').value='Home folder';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(calls.find(([method])=>method==='folder')[1].parent,'1');
  assert.equal(document.getElementById('new-apps').children[0].textContent,'GridCraft');
  assert.match(document.getElementById('new-apps').children[0].href,/\/apps\/gridcraft\//);
});

test('late Home results do not replace a folder after navigation',async()=>{
  const pending=deferred();
  const {host,document}=fixture({search:'#kodu',suggestions(){return pending.promise;},query(){return {entries:[],cursor:null};}});
  const startup=vm.runInContext(`(async()=>{${source}})()`,host.context);await tick();
  host.context.location.hash='#failid';host.window.dispatchEvent(new Event('hashchange'));await tick();
  pending.resolve({entries:[],quickFolders:[{...trashEntry('late'),folder:true}],cursor:null});await startup;
  assert.equal(document.getElementById('home-folders').hidden,true);
  assert.equal(document.getElementById('home-folder-list').children.length,0);
});

test('column sorting and filter chips invoke the canonical query without losing typed search text on refresh',async()=>{
  const {host,document,calls}=fixture();await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('column-size').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].sort,'size');
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].descending,true);
  document.getElementById('search-text').value='Planning';
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('search-text').value,'Planning');
  document.getElementById('search-type').value='text';document.getElementById('search-type').dispatchEvent(new Event('change'));
  host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].text,'Planning');
  assert.equal(calls.filter(([method])=>method==='query').at(-1)[1].fileType,'text');
});

test('failed and cancelled previews never acknowledge a successful open; summary failures do not block a document',async()=>{
  for(const failure of ['download','decode','summary']) {
    const {host,document,calls}=fixture({file:previewFile,
      download:async()=>{if(failure==='download')throw new Error('Network failed');return failure==='decode'?new Uint8Array([255]):new TextEncoder().encode('Opened safely');},
      drive(method){if(method==='acknowledge')throw new Error('Summary unavailable');},
    });
    await vm.runInContext(`(async()=>{${source}})()`,host.context);openPreview(document);await tick();
    assert.equal(calls.filter(([m])=>m==='acknowledge').length,failure==='summary'?1:0);
    if(failure==='summary')assert.equal(document.getElementById('preview-content').children[0].textContent,'Opened safely');
    else assert.equal(document.getElementById('preview-content').children[0].classList.contains('is-error'),true);
  }
});

test('folder visits acknowledge after the listing succeeds, but background refresh does not',async()=>{
  const {host,document,calls}=fixture({search:'#folder/2'});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);await tick();
  assert.deepEqual(calls.filter(([m])=>m==='acknowledge'),[['acknowledge',{id:'2',revision:null}]]);
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([m])=>m==='acknowledge').length,1);
});

test('Suggested Home revalidates without reordering surviving rows, and dismissal does not delete or unstar',async()=>{
  let items=['first','second'].map(id=>({...trashEntry(id),reason:'starred',starred:true}));
  let folders=['a','b'].map(id=>({...trashEntry(id),folder:true,reason:'starred'})),backgroundRefresh;
  const {host,document,calls}=fixture({search:'#kodu',setInterval(fn,ms){if(ms===30000)backgroundRefresh=fn;},suggestions:()=>({entries:items,quickFolders:folders,cursor:null}),drive(method,p){if(method==='dismiss'){items=items.filter(item=>item.id!==p.id);return null;}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const fileIds=()=>document.getElementById('file-list').children.map(row=>row.dataset.fileId);
  const folderLinks=()=>document.getElementById('home-folder-list').children.map(link=>link.href);
  items=[items[1],items[0]];folders=[folders[1],folders[0]];backgroundRefresh();await tick();
  assert.deepEqual(fileIds(),['first','second']);assert.deepEqual(folderLinks(),['/pilveruum/#folder/a','/pilveruum/#folder/b']);
  document.getElementById('refresh-files').dispatchEvent(new Event('click'));await tick();
  assert.deepEqual(fileIds(),['second','first']);assert.deepEqual(folderLinks(),['/pilveruum/#folder/b','/pilveruum/#folder/a']);
  findButton(document.getElementById('file-list').children[0],'Peida seitsmeks päevaks').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('file-list').children.length,1);
  assert.equal(calls.filter(([m])=>m==='dismiss').length,1);
  assert.equal(calls.some(([m])=>['organize','star'].includes(m)),false);
  host.i18n.setLanguage('en');assert.match(document.getElementById('file-list').textContent,/Starred/);
});

test('copy uses verified source bytes, destination access, an explicit name and the same retry request',async()=>{
  let fail=true;
  const copied=[];
  const {host,document,calls}=fixture({file:{...previewFile,capabilities:{read:true,edit:false,delete:false}},
    drive(method){if(method==='list')return {entries:[],cursor:null};},
    upload:async(payload,bytes)=>{copied.push({payload:JSON.parse(payload),bytes:[...bytes]});if(fail)throw new Error('Retry upload');},
  });
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Tee koopia').dispatchEvent(new Event('click'));await tick();
  const name=document.getElementById('dialog-body').querySelector('input');name.value='My copy.txt';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(document.getElementById('drive-dialog').open,false);
  assert.match(document.getElementById('transfer-items').textContent,/Ebaõnnestus/);
  assert.equal(copied[0].payload.parent,'1');assert.equal(copied[0].payload.name,'My copy.txt');
  assert.equal(copied[0].payload.file,undefined);assert.equal(copied[0].payload.capabilities,undefined);
  fail=false;findButton(document.getElementById('transfer-items'),'Proovi uuesti').dispatchEvent(new Event('click'));await tick();
  assert.deepEqual(copied[0],copied[1]);assert.equal(calls.filter(([m])=>m==='download').length,2);
  assert.equal(document.getElementById('drive-dialog').open,false);
  assert.equal(calls.filter(([m])=>m==='account').length,2,'copy retries reconcile storage usage too');
});

test('restricted file link resolves the item, preserves sign-in destination and never creates a grant',async()=>{
  const copied=[];
  const options={search:'#file/42',account:{authenticated:false},copy:value=>copied.push(value),query:p=>({entries:[{...trashEntry('42'),format:previewFile.format}],cursor:null})};
  const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(calls.some(([m])=>m==='query'),false);assert.equal(host.context.location.hash,'#file/42');
  options.account={authenticated:true};host.window.dispatchEvent(new Event('storage'));await tick();
  assert.equal(calls.filter(([m])=>m==='query').at(-1)[1].scope,'item');
  assert.equal(calls.filter(([m])=>m==='query').at(-1)[1].id,'42');
  findButton(document.getElementById('file-list'),'Kopeeri link').dispatchEvent(new Event('click'));await tick();
  assert.match(copied[0],/\/pilveruum\/#file\/42$/);assert.equal(calls.some(([m])=>m==='grant'),false);
});

test('sharing role changes preserve grant identity/version/expiry and inherited grants stay read-only',async()=>{
  const grant={id:'9',root:'first',version:'3',subject:'bob',active:true,expiresAt:'9999999999999999999',capabilities:{read:true,edit:false,create:false,delete:false,propose:false}};
  const {host,document,calls}=fixture({file:previewFile,drive(method){if(method==='grants')return [grant,{...grant,id:'10',root:'parent'}];if(method==='grant')return null;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Jaga').dispatchEvent(new Event('click'));await tick();
  const roles=document.getElementById('dialog-body').querySelectorAll('select');
  assert.equal(roles[0].value,'viewer');assert.equal(roles[1].value,'viewer');assert.equal(roles[2].disabled,true);
  roles[1].value='editor';roles[1].dispatchEvent(new Event('change'));await tick();
  const request=calls.find(([m])=>m==='grant')[1];
  assert.equal(request.id,'9');assert.equal(request.version,'3');assert.equal(request.expiresAt,grant.expiresAt);
  assert.deepEqual(request.capabilities,{read:true,propose:false,create:false,edit:true,delete:true});
});

test('move previews access changes, binds catalog version, and retries an uncertain commit without replanning',async()=>{
  let fail=true;const destination={id:'2',spaceId:'1',owner:'alice',name:'Destination',folder:true,capabilities:{read:true,create:true}};
  const {host,document,calls}=fixture({file:{parent:'1'},drive(method,p){
    if(method==='list')return {entries:p.parent?[]:[destination],cursor:null};
    if(method==='movePreview')return {version:'1',treeVersion:'7',affectedObjects:1,affectedSubjects:1,changes:[{subject:'bob',gains:true,loses:false,objects:1}]};
    if(method==='organize'){if(fail)throw new Error('lost response');return {id:p.id,version:'2'};}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Liiguta').dispatchEvent(new Event('click'));await tick();
  findButton(document.getElementById('dialog-body'),'Destination').dispatchEvent(new Event('click'));await tick();
  assert.match(document.getElementById('dialog-body').textContent,/bob/);assert.match(document.getElementById('dialog-body').textContent,/Ligipääs laieneb/);
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(document.getElementById('drive-dialog').open,true);fail=false;
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  const sent=calls.filter(([m])=>m==='organize');assert.equal(sent.length,2);assert.deepEqual(sent[0][1],sent[1][1]);assert.equal(sent[0][1].treeVersion,'7');
  assert.equal(calls.filter(([m])=>m==='movePreview').length,2);
});

test('folder upload shows an explicit collision choice and keeps unrelated siblings after skipping a parent',async()=>{
  const {host,document,calls}=fixture({drive(method,p){if(method==='list')return {entries:p.parent==='1'?[{id:'10',name:'Root',folder:true,capabilities:{create:true}}]:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const input=document.getElementById('upload-input');input.files=[
    {name:'nested.txt',webkitRelativePath:'Root/nested.txt',size:1,arrayBuffer:async()=>new Uint8Array([1])},
    {name:'sibling.txt',size:1,arrayBuffer:async()=>new Uint8Array([2])},
  ];input.dispatchEvent(new Event('change'));await tick();
  assert.equal(document.getElementById('dialog-title').textContent,'Sama nimega üksus on juba olemas');
  findButton(document.getElementById('dialog-body'),'Jäta vahele').dispatchEvent(new Event('click'));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='upload').map(([,p])=>p.name),['sibling.txt']);
  assert.match(document.getElementById('transfer-items').textContent,/Vahele jäetud/);
});

test('multi-file copy uploads distinct immutable revisions to the chosen folder',async()=>{
  const a={...previewFile,id:'a',spaceId:'1',owner:'alice',name:'a.txt',revision:'11',version:'1',bytes:2,parent:'1',capabilities:{read:true,edit:true,delete:true}};
  const b={...a,id:'b',name:'b.txt',revision:'12'};
  const {host,document,calls}=fixture({query:()=>({folder:{id:'1',spaceId:'1',owner:'alice',name:'Root',folder:true,capabilities:{read:true,create:true}},breadcrumbs:[],entries:[a,b],cursor:null}),drive(method){if(method==='list')return {entries:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('selection-copy').disabled,false);
  document.getElementById('selection-copy').dispatchEvent(new Event('click'));await tick();
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='download').map(c=>c.slice(1)),[['a','11'],['b','12']]);
  assert.deepEqual(calls.filter(([m])=>m==='upload').map(([,p])=>p.name),['a.txt','b.txt']);
});

test('clipboard menu remains account-scoped and paste starts in the current folder',async()=>{
  const options={drive(method){if(method==='list')return {entries:[],cursor:null};}};
  const {host,document,calls}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  document.getElementById('clipboard-copy').dispatchEvent(new Event('click'));
  host.context.location.hash='#folder/2';host.window.dispatchEvent(new Event('hashchange'));await tick();
  assert.equal(document.getElementById('clipboard-paste').disabled,false);
  document.getElementById('clipboard-paste').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([m])=>m==='list').at(-1)[1].parent,'2');
  document.getElementById('dialog-close').dispatchEvent(new Event('click'));
  options.account={principal:'bob'};host.window.dispatchEvent(new Event('storage'));await tick();
  assert.equal(document.getElementById('clipboard-paste').disabled,true);
});

test('new Craft launch and save picker preserve the destination and expose its owner',async()=>{
  const {host,document}=fixture({search:'#folder/2'});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const link=document.getElementById('new-apps').children[0];link.dispatchEvent(new Event('click'));
  assert.match(link.href,/cloud_space=1&cloud_parent=2/);
  const picker=fixture({search:'?picker=save&app=gridcraft&channel=aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa&space=1&parent=2&name=Example#folder/2'});
  await vm.runInContext(`(async()=>{${source}})()`,picker.host.context);
  assert.equal(picker.calls.find(([m])=>m==='query')[1].parent,'2');
  assert.match(picker.document.getElementById('picker-owner').textContent,/Sihtkausta omanik: mina/);
});

test('browser clipboard events support system copy/paste and reject unrelated clipboard text',async()=>{
  const {host,document}=fixture({drive(method){if(method==='list')return {entries:[],cursor:null};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('select-all-files').dispatchEvent(new Event('click'));
  let clipboard='';const event=type=>{const e=new Event(type,{cancelable:true});e.clipboardData={setData:(_,v)=>{clipboard=v;},getData:()=>clipboard};return e;};
  const copy=event('copy');document.dispatchEvent(copy);assert.equal(copy.defaultPrevented,true);assert.match(clipboard,/#file\/first$/);
  const saved=clipboard;clipboard='unrelated text';document.dispatchEvent(event('paste'));assert.notEqual(document.getElementById('drive-dialog').open,true);
  clipboard=saved;const paste=event('paste');document.dispatchEvent(paste);await tick();assert.equal(paste.defaultPrevented,true);assert.equal(document.getElementById('drive-dialog').open,true);
});

test('downloading a directly shared file does not require access to its private parent',async()=>{
  const {host,document,calls,urls}=fixture({file:{...previewFile,parent:'private-parent',owner:'bob'},drive(method,p){if(method==='get' && p.id==='private-parent')throw new Error('Not found or access unavailable');}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.createElement=(create=>tag=>{const node=create(tag);if(tag==='a')node.click=()=>{};return node;})(document.createElement);
  findButton(document.getElementById('file-list'),'Laadi alla').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([m])=>m==='download').length,1);assert.equal(urls.filter(([kind])=>kind==='create').length,1);
});

test('email settings show multiple contacts and verification does not replace the login session',async()=>{
  let pending=null;const emails=['first@example.com','second@example.com'];
  const {host,document,calls}=fixture({drive(method,p){
    if(method==='emailProfile')return {enabled:true,emails:[...emails],pending};
    if(method==='emailRequest'){pending={id:'20',email:p.email,delivery:'Accepted'};return {enabled:true,emails:[...emails],pending};}
    if(method==='emailVerify'){emails.push(pending.email);pending=null;return 'Added';}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('account-email').dispatchEvent(new Event('click'));await tick();
  const body=document.getElementById('dialog-body');assert.match(body.textContent,/first@example.com/);assert.match(body.textContent,/second@example.com/);
  findButton(body,'Lisa e-post').dispatchEvent(new Event('click'));
  body.querySelector('input').value='third@example.com';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.match(body.textContent,/third@example.com/);assert.equal(document.getElementById('dialog-submit').textContent,'Kinnita kood');
  body.querySelector('input').value='12345678';document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();await tick();
  assert.match(body.textContent,/third@example.com/);assert.equal(calls.filter(([m])=>m==='emailVerify').length,1);
  assert.equal(calls.some(([m])=>m==='sign_in'||m==='sign_out'),false);
});

test('a proved email on another principal requires an explicit transfer action',async()=>{
  let moved=false;
  const {host,document,calls}=fixture({drive(method){
    if(method==='emailProfile')return {enabled:true,emails:moved?['shared@example.com']:[],pending:moved?null:{id:'25',email:'shared@example.com',delivery:'Accepted'}};
    if(method==='emailVerify')return 'TransferRequired';
    if(method==='emailConfirmTransfer'){moved=true;return 'Added';}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('account-email').dispatchEvent(new Event('click'));await tick();
  const body=document.getElementById('dialog-body');findButton(body,'Jätka e-posti kinnitamist').dispatchEvent(new Event('click'));
  body.querySelector('input').value='12345678';document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.match(body.textContent,/Vana konto failid/);assert.match(body.textContent,/Ootel kutseid ei kanta üle/);
  assert.equal(calls.some(([m])=>m==='emailConfirmTransfer'),false);
  host.i18n.setLanguage('en');assert.equal(document.getElementById('dialog-submit').textContent,'Move email');
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();await tick();
  assert.equal(calls.filter(([m])=>m==='emailConfirmTransfer').length,1);assert.match(body.textContent,/shared@example.com/);
});

test('invitation link alone neither grants access nor reveals the destination',async()=>{
  const {host,document,calls}=fixture({search:'#invite/29',drive(method){if(method==='emailAccept')throw new Error('Invitation is not available for this account');}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('dialog-title').textContent,'Jagamiskutse');
  assert.equal(calls.some(([m])=>m==='emailAccept'),false);
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(calls.filter(([m])=>m==='emailAccept').length,1);
  assert.equal(host.window.location.hash,'#invite/29');assert.match(document.getElementById('dialog-error').textContent,/ei ole sellele kontole/);
});

test('email profile response cannot render after an account switch',async()=>{
  let resolve;const options={drive(method){if(method==='emailProfile')return new Promise(r=>{resolve=r;});}};
  const {host,document}=fixture(options);await vm.runInContext(`(async()=>{${source}})()`,host.context);
  document.getElementById('account-email').dispatchEvent(new Event('click'));await tick();options.stale=true;
  resolve({enabled:true,emails:['private@example.com'],pending:null});await tick();
  assert.doesNotMatch(document.getElementById('dialog-body').textContent,/private@example.com/);
});


test('email deep link waits for login and then resumes the pending code on the same route',async()=>{
  const options={search:'#email',account:{authenticated:false},drive(method){
    if(method==='emailProfile')return {enabled:true,emails:[],pending:{id:'31',email:'mine@example.com',delivery:'Accepted'}};
  }};
  const {host,document,calls}=fixture(options);
  options.signIn=()=>{options.account={authenticated:true};};
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(calls.some(([m])=>m==='emailProfile'),false);
  assert.equal(host.window.location.hash,'#email');
  document.getElementById('sign-in').dispatchEvent(new Event('click'));await tick();await tick();
  assert.equal(document.getElementById('dialog-title').textContent,'Minu e-postid');
  findButton(document.getElementById('dialog-body'),'Jätka e-posti kinnitamist').dispatchEvent(new Event('click'));
  assert.equal(document.getElementById('dialog-submit').textContent,'Kinnita kood');
  assert.match(document.getElementById('dialog-body').textContent,/mine@example.com/);
  assert.equal(host.window.location.hash,'#email');
});

test('invitation can resume after verifying a contact and opens only the accepted object',async()=>{
  let pending=null,verified=false;
  const {host,document,calls}=fixture({search:'#invite/29',drive(method,p){
    if(method==='emailProfile')return {enabled:true,emails:verified?['reader@example.com']:[],pending};
    if(method==='emailRequest'){pending={id:'32',email:p.email,delivery:'Accepted'};return {enabled:true,emails:[],pending};}
    if(method==='emailVerify'){verified=true;pending=null;return 'Added';}
    if(method==='emailAccept')return {id:'2'};
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const body=document.getElementById('dialog-body'),form=document.getElementById('dialog-form');
  findButton(body,'Minu e-postid').dispatchEvent(new Event('click'));await tick();
  findButton(body,'Lisa e-post').dispatchEvent(new Event('click'));body.querySelector('input').value='reader@example.com';
  form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  body.querySelector('input').value='12345678';form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();await tick();
  assert.equal(calls.some(([m])=>m==='emailAccept'),false);
  findButton(body,'Tagasi kutse juurde').dispatchEvent(new Event('click'));
  form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();await tick();
  assert.equal(host.window.location.hash,'#folder/2');
  assert.equal(document.getElementById('drive-dialog').open,false);
});

test('email sharing uses the selected role and only retryable deliveries offer retry',async()=>{
  const {host,document,calls}=fixture({drive(method){
    if(method==='grants')return [];
    if(method==='emailInvitations')return ['Failed','Expired','Unknown'].map((delivery,i)=>({id:String(50+i),email:`recipient${i}@example.com`,active:true,accepted:false,expiresAt:String(BigInt(Date.now()+3600000)*1000000n),delivery}));
    if(method==='emailShare')return {id:'60',delivery:'Accepted'};
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Jaga').dispatchEvent(new Event('click'));await tick();await tick();
  const body=document.getElementById('dialog-body');
  const rows=body.querySelectorAll('.share-row');
  assert.equal(findButton(rows[0],'Proovi saatmist uuesti'),undefined);
  assert.equal(findButton(rows[1],'Proovi saatmist uuesti'),undefined);
  assert.ok(findButton(rows[2],'Proovi saatmist uuesti'));
  body.querySelector('input').value='reader@example.com';body.querySelector('select').value='editor';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();await tick();
  const sent=calls.filter(([m])=>m==='emailShare');assert.equal(sent.length,1);
  assert.equal(sent[0][1].email,'reader@example.com');assert.equal(sent[0][1].role,'editor');
  assert.equal(calls.some(([m])=>m==='grant'),false);
});

test('folder tree loads only expanded folders, follows cursors and resets on account change',async()=>{
  const {host,document,calls}=fixture({query(p){
    if(p.fileType!=='folder')return {entries:[],cursor:null};
    const child={id:p.cursor?'3':'2',parent:p.parent,spaceId:'1',folder:true,name:p.cursor?'Second folder':'First folder',capabilities:{read:true}};
    return {entries:p.parent==='1'?[child]:[],cursor:p.parent==='1'&&!p.cursor?'next':null};
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(calls.filter(([m,p])=>m==='query'&&p.fileType==='folder').length,0);
  document.getElementById('folder-tree-toggle').dispatchEvent(new Event('click'));await tick();
  const tree=document.getElementById('folder-tree');assert.match(tree.textContent,/First folder/);assert.doesNotMatch(tree.textContent,/Second folder/);
  findButton(tree,'Näita rohkem kaustu').dispatchEvent(new Event('click'));await tick();assert.match(tree.textContent,/Second folder/);
  const first=tree.querySelector('[role="treeitem"]');first.dispatchEvent(keyEvent('ArrowRight'));await tick();assert.match(tree.textContent,/Alamkaustu pole/);
  assert.deepEqual(calls.filter(([m,p])=>m==='query'&&p.fileType==='folder').map(([,p])=>[p.parent,p.cursor]),[['1',null],['1','next'],['2',null]]);
  host.window.dispatchEvent(new Event('storage'));await tick();assert.equal(tree.hidden,true);assert.equal(tree.textContent,'');
});

test('details display caller access, preserve names and discard responses after closing',async()=>{
  const pending=deferred();let second=false;
  const f={id:'2',version:'1',spaceId:'1',parent:'1',owner:'bob',name:'<b>Private.txt</b>',folder:false,createdMs:1700000000000,updatedMs:1700000000000,bytes:50,format:{media_type:'text/plain'},capabilities:{read:true}};
  const {host,document}=fixture({file:f,query(){return {entries:[f],cursor:null};},drive(method){if(method==='details')return second?pending.promise:{entry:f,locations:[],access:[{inherited:false,location:{id:'2',name:f.name}}],versions:3};}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Faili teave').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('details-title').textContent,f.name);
  const body=document.getElementById('details-body');assert.match(body.textContent,/Vaataja/);assert.match(body.textContent,/Jagatud otse/);assert.match(body.textContent,/Ülemkausta asukoht pole saadaval/);assert.equal(findButton(body,'Jaga'),undefined);
  document.getElementById('details-close').dispatchEvent(new Event('click'));second=true;
  findButton(document.getElementById('file-list'),'Faili teave').dispatchEvent(new Event('click'));await tick();
  document.getElementById('details-close').dispatchEvent(new Event('click'));pending.resolve({entry:f,locations:[],access:[],versions:3});await tick();
  assert.equal(document.getElementById('file-details').hidden,true);assert.equal(body.textContent,'');
});

test('rename Undo submits the receipt version and preserves another edit on conflict',async()=>{
  let reject=false;
  const {host,document,calls,rename,state,submit}=resourceFixture({drive(method,p){if(method==='organize'&&reject)throw new Error('This item changed; refresh and try again');}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  rename('2','Renamed.txt');await tick();assert.equal(document.getElementById('undo-notice').hidden,false);
  document.getElementById('undo-action').dispatchEvent(new Event('click'));await tick();
  const operations=calls.filter(([m])=>m==='organize');assert.equal(operations[1][1].version,'2');assert.equal(operations[1][1].name,'Alpha.txt');assert.equal(state.items[0].name,'Alpha.txt');
  assert.equal(document.getElementById('undo-notice').hidden,true);
  rename('2','Second rename.txt');await tick();state.items[0].name='Someone else.txt';reject=true;
  document.getElementById('undo-action').dispatchEvent(new Event('click'));await tick();
  assert.match(document.getElementById('account-message').textContent,/vahepeal muutunud/);assert.equal(state.items[0].name,'Someone else.txt');
});

test('alternative restore checks collision before confirmation and reuses the request on lost response',async()=>{
  let requests=[],fail=true;
  const trashed={...trashEntry('7'),parent:'44',name:'Occupied.txt'};
  const {host,document,calls}=fixture({search:'#prugikast',drive(method,p){
    if(method==='trash')return {entries:[trashed],cursor:null};
    if(method==='browse')return {entries:[],cursor:null};
    if(method==='restorePreview'){if(p.name==='Occupied.txt')throw new Error('An item with this name already exists');return {version:'2',treeVersion:'8',affectedObjects:0,affectedSubjects:0,changes:[]};}
    if(method==='organize'){requests.push(p);if(fail)throw new Error('Lost response');return {id:'7',version:'3'};}
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Taasta mujale…').dispatchEvent(new Event('click'));await tick();
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(requests.length,0);assert.match(document.getElementById('dialog-error').textContent,/juba olemas/);
  document.getElementById('dialog-body').querySelector('input').value='Recovered.txt';form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(document.getElementById('dialog-title').textContent,'Kinnita taastamine');assert.equal(requests.length,0);
  form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();fail=false;form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(requests.length,2);assert.deepEqual(requests[0],requests[1]);assert.equal(requests[0].change,'restoreTo');assert.equal(requests[0].name,'Recovered.txt');assert.equal(requests[0].treeVersion,'8');
});

test('partial Undo retains uncertain request IDs and never replays successful items',async()=>{
  let lose=true;const inverseCalls=[];const saved=new Map();
  const {host,document,calls,row,submit,state}=resourceFixture({drive(method,p,state){
    if(method==='restorePreview'){const item=state.trash.find(f=>f.id===p.id);if(!item)throw new Error('Already restored');return {version:item.version,treeVersion:'7',affectedSubjects:0,affectedObjects:0,changes:[]};}
    if(method==='organize' && p.change==='restoreTo'){
      inverseCalls.push({...p});if(saved.has(p.requestId))return saved.get(p.requestId);
      const item=state.trash.find(f=>f.id===p.id);state.trash=state.trash.filter(f=>f!==item);state.items.push(item);item.version=String(Number(item.version)+1);
      const receipt={id:item.id,version:item.version};saved.set(p.requestId,receipt);
      if(item.id==='3' && lose){lose=false;throw new Error('Lost response');}return receipt;
    }
  }});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  row('2').dispatchEvent(clickEvent());const add=clickEvent();add.metaKey=true;row('3').dispatchEvent(add);
  document.getElementById('selection-trash').dispatchEvent(new Event('click'));submit();await tick();
  assert.equal(state.trash.length,2);document.getElementById('undo-action').dispatchEvent(new Event('click'));await tick();
  assert.equal(state.trash.length,0);assert.equal(document.getElementById('undo-notice').hidden,false);
  const previewCount=calls.filter(([m])=>m==='restorePreview').length;
  document.getElementById('undo-action').dispatchEvent(new Event('click'));await tick();
  assert.equal(document.getElementById('undo-notice').hidden,true);assert.equal(inverseCalls.length,3);
  assert.deepEqual(inverseCalls[0],inverseCalls[2]);assert.equal(inverseCalls.filter(p=>p.id==='2').length,1);
  assert.equal(calls.filter(([m])=>m==='restorePreview').length,previewCount);
});

test('details revalidation removes stale metadata after access is revoked',async()=>{
  let denied=false;
  const f={id:'2',version:'1',spaceId:'1',owner:'bob',name:'Shared document',folder:false,bytes:5,createdMs:10,updatedMs:20,capabilities:{read:true}};
  const {host,document}=fixture({query(){return {entries:denied?[]:[f],cursor:null};},drive(method){if(method==='details'){if(denied)throw new Error('Not found or access unavailable');return {entry:f,locations:[{id:'44',name:'Shared folder'}],access:[],versions:1};}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Faili teave').dispatchEvent(new Event('click'));await tick();
  assert.match(document.getElementById('details-body').textContent,/Shared folder/);
  host.i18n.setLanguage('en');assert.match(document.getElementById('details-body').textContent,/My access/);
  denied=true;document.getElementById('refresh-files').dispatchEvent(new Event('click'));await tick();
  assert.doesNotMatch(document.getElementById('details-body').textContent,/Shared folder/);assert.match(document.getElementById('details-body').textContent,/unavailable/);
});

test('storage projection separates categories and opens current version management',async()=>{
  const {host,document,calls}=fixture({search:'#salvestusruum',drive(method){
    if(method==='storage')return {currentBytes:100,historyBytes:40,trashBytes:20,pendingBytes:10,usedBytes:170,quotaBytes:1000,files:[{id:'42',name:'Budget',bytes:100,historyBytes:40,versions:2,trashed:false}]};
    if(method==='history')return [{id:'44',bytes:100,updatedMs:1700000000000,author:'alice',label:'Release',keep:true}];
  }});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('failid').hidden,true);assert.equal(document.getElementById('insights-view').hidden,false);
  assert.match(document.getElementById('insights-body').textContent,/Vanad versioonid/);assert.match(document.getElementById('insights-body').textContent,/140 B/);
  assert.equal(calls.some(([m])=>m==='query'),false);
  findButton(document.getElementById('insights-body'),'Versioonid').dispatchEvent(new Event('click'));await tick();
  assert.match(document.getElementById('dialog-body').textContent,/Release/);assert.match(document.getElementById('dialog-body').textContent,/Säilib püsivalt/);
  host.i18n.setLanguage('en');assert.match(document.getElementById('dialog-body').textContent,/Kept permanently/);
});
test('activity pages preserve server cursor and discard responses after account change',async()=>{
  let finish,delayed=false;const {host,document,calls}=fixture({search:'#tegevused',drive(method,p){
    if(method==='activity'){if(delayed)return new Promise(resolve=>{finish=resolve;});return {items:[{id:'8',object:'42',name:'Current name',kind:'Renamed',atMs:1700000000000,actor:'alice'}],before:'8'};}
  }});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.match(document.getElementById('insights-body').textContent,/Current name/);assert.match(document.getElementById('insights-body').textContent,/Muudeti nime/);
  delayed=true;findButton(document.getElementById('insights-body'),'Näita rohkem').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([m])=>m==='activity').at(-1)[1].before,'8');
  host.context.location.hash='#failid';host.window.dispatchEvent(new Event('hashchange'));await tick();
  finish({items:[{id:'7',object:'43',name:'Stale secret',kind:'Created',atMs:1700000000000,actor:'alice'}],before:null});await tick();
  assert.equal(document.getElementById('insights-view').hidden,true);assert.doesNotMatch(document.getElementById('insights-body').textContent,/Stale secret/);
});
test('named versions are protected and deletion confirms one exact version with safe retries',async()=>{
  let fail=true;const requests=[];const {host,document,calls}=fixture({drive(method,p){
    if(method==='history')return [{id:'10',bytes:3,updatedMs:1700000000000,author:'alice',propertiesVersion:'2'},{id:'11',bytes:7,updatedMs:1700000000000,author:'alice',propertiesVersion:'3',label:'Keep',keep:true},{id:'44',bytes:100,updatedMs:1700000000000,author:'alice'}];
    if(method==='revisionDelete'){requests.push(p);if(fail)throw new Error('lost response');return {id:p.id,version:'2'};}
  }});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Versioonid').dispatchEvent(new Event('click'));await tick();
  const buttons=document.getElementById('dialog-body').querySelectorAll('button').filter(b=>b.textContent==='Kustuta versioon');
  assert.equal(buttons.length,2);assert.equal(buttons[0].disabled,true);
  buttons[1].dispatchEvent(new Event('click'));assert.match(document.getElementById('dialog-body').textContent,/jäädavalt/);assert.equal(requests.length,0);
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();assert.equal(document.getElementById('drive-dialog').open,true);
  fail=false;form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();assert.equal(requests.length,2);assert.deepEqual(requests[0],requests[1]);
  assert.equal(requests[0].revision,'10');assert.equal(requests[0].propertiesVersion,'2');assert.equal(calls.some(([m])=>m==='account'),true);
});

test('shortcut creation retains identity on retry and opening resolves current target authority',async()=>{
  let fail=true;const requests=[];const f={id:'7',folder:false,name:'Shared.txt',owner:'bob',spaceId:'9',capabilities:{read:true}};
  const {host,document,calls}=fixture({query(){return {entries:[f],cursor:null};},drive(method,p){
    if(method==='browse')return {folder:{id:'1',spaceId:'1',owner:'alice',folder:true,capabilities:{create:true}},breadcrumbs:[],entries:[],cursor:null};
    if(method==='shortcut'){requests.push(p);if(fail)throw new Error('Lost response');return {id:'20'};}
  }});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Lisa otsetee').dispatchEvent(new Event('click'));await tick();
  assert.match(document.getElementById('dialog-body').textContent,/ei anna sihtfailile ligipääsu/);
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();fail=false;form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(requests.length,2);assert.deepEqual(requests[0],requests[1]);assert.equal(requests[0].target,'7');assert.equal(requests[0].parent,'1');
  const shortcut={...f,id:'20',shortcut:'7',name:'Alias'};
  const second=fixture({query(){return {entries:[shortcut],cursor:null};},drive(method){if(method==='shortcutTarget')throw new Error('Not found or access unavailable');}});
  await vm.runInContext(`(async()=>{${source}})()`,second.host.context);
  second.document.getElementById('file-list').children[0].dispatchEvent(new Event('dblclick'));await tick();
  assert.equal(second.calls.filter(([m])=>m==='shortcutTarget').length,1);assert.equal(second.calls.some(([m])=>m==='download'),false);
  assert.match(second.document.getElementById('account-message').textContent,/kättesaadav|ligipääs/);
});
test('folder color is an explicit private mutation and default clears it',async()=>{
  const folder={id:'2',folder:true,name:'Personal folder',owner:'alice',capabilities:{read:true,edit:true},color:'gold'};
  const {host,document,calls}=fixture({query(){return {entries:[folder],cursor:null};},drive(method){if(method==='folderColor')return null;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Kausta värv').dispatchEvent(new Event('click'));
  findButton(document.getElementById('dialog-body'),'Vaikimisi').dispatchEvent(new Event('click'));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='folderColor').map(([,p])=>p),[{id:'2',color:null}]);
});
test('inaccessible direct links offer a request without guessing owner or granting access',async()=>{
  const {host,document,calls}=fixture({search:'#file/777',query(){throw new Error('Not found or access unavailable');},drive(method){if(method==='requestAccess')return null;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-status'),'Küsi ligipääsu').dispatchEvent(new Event('click'));
  document.getElementById('dialog-body').querySelector('input').value='Please share';
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(document.getElementById('dialog-title').textContent,'Taotlus saadetud');
  assert.deepEqual(calls.filter(([m])=>m==='requestAccess').map(([,p])=>p),[{id:'777',message:'Please share'}]);assert.equal(calls.some(([m])=>m==='grant'),false);
});
test('inbox approval is explicit, role-scoped and preserves the decision on retry',async()=>{
  const decisions=[];let fail=true;
  const {host,document}=fixture({search:'#teavitused',drive(method,p){
    if(method==='inbox')return {notifications:true,items:[{id:'1',object:'7',sender:'bob',kind:'AccessRequested',at:'123456789',atMs:1700000000000,name:'Plan',message:'Please',canRespond:true}]};
    if(method==='decideAccess'){decisions.push(p);if(fail)throw new Error('Lost response');return null;}
  }});await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('people-body'),'Vaata taotlust').dispatchEvent(new Event('click'));
  assert.equal(decisions.length,0);const body=document.getElementById('dialog-body');body.querySelectorAll('select')[1].value='editor';
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();fail=false;form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(decisions.length,2);assert.deepEqual(decisions[0],decisions[1]);assert.equal(decisions[0].role,'editor');assert.equal(decisions[0].requestedAt,'123456789');
});
test('multi-recipient sharing retries only failed recipients with the original expiry',async()=>{
  const sent=[];let fail=true;
  const {host,document}=fixture({drive(method,p){if(method==='grants'||method==='contacts')return [];if(method==='grant'){sent.push(p);if(p.subject==='bob'&&fail)throw new Error('Lost response');return null;}}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Jaga').dispatchEvent(new Event('click'));await tick();
  const body=document.getElementById('dialog-body'),inputs=body.querySelectorAll('input');inputs[0].value='ann, bob';inputs[1].value='2099-01-01T10:00';
  const form=document.getElementById('dialog-form');form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();fail=false;
  form.dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.equal(sent.length,3);assert.equal(sent.filter(p=>p.subject==='ann').length,1);assert.deepEqual(sent[1],sent[2]);assert.ok(sent[1].expiresAt);
});

test('blocking and reporting require distinct explicit actions and never delete shared data',async()=>{
  const shared={id:'8',name:'Shared file',owner:'bob',folder:false,capabilities:{read:true}};
  const {host,document,calls}=fixture({query(){return {entries:[shared],cursor:null};},drive(method){if(['block','report','hideShared'].includes(method))return null;}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  findButton(document.getElementById('file-list'),'Blokeeri omanik').dispatchEvent(new Event('click'));
  assert.equal(calls.some(([m])=>m==='block'),false);assert.match(document.getElementById('dialog-body').textContent,/üksteise jagatud faile/);
  document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='block').map(([,p])=>p),[{principal:'bob',blocked:true}]);
  findButton(document.getElementById('file-list'),'Raporteeri jagamist').dispatchEvent(new Event('click'));
  document.getElementById('dialog-body').querySelector('input').value='Unwanted content';document.getElementById('dialog-form').dispatchEvent(new Event('submit',{cancelable:true}));await tick();
  assert.deepEqual(calls.filter(([m])=>m==='report').map(([,p])=>p),[{id:'8',reason:'Unwanted content'}]);assert.equal(calls.some(([m])=>m==='organize'||m==='purge'),false);
});
test('inbox response cannot populate a different account or route',async()=>{
  let finish;const {host,document}=fixture({search:'#teavitused',drive(method){if(method==='inbox')return new Promise(r=>{finish=r;});}});
  const loading=vm.runInContext(`(async()=>{${source}})()`,host.context);await tick();
  host.window.location.hash='#failid';host.window.dispatchEvent(new Event('hashchange'));await tick();
  finish({notifications:true,items:[{id:'1',object:'7',sender:'bob',kind:'Shared',name:'Old private name',atMs:1700000000000}]});await loading;await tick();
  assert.equal(document.getElementById('people-view').hidden,true);assert.doesNotMatch(document.getElementById('people-body').textContent,/Old private name/);
});


test('switching previews keeps the dialog and a delayed close cannot erase the new file',async()=>{
  const f=id=>({id,spaceId:'1',owner:'alice',name:id+'.txt',folder:false,bytes:5,revision:'1',format:{media_type:'text/plain'},capabilities:{read:true}});
  const {host,document}=fixture({query(){return {entries:[f('first'),f('second')],cursor:null};},drive(method,p){if(method==='get')return f(p.id);}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  const dialog=document.getElementById('preview-dialog');let closes=0;
  dialog.close=()=>{dialog.open=false;closes++;setImmediate(()=>dialog.dispatchEvent(new Event('close')));};
  const row=document.getElementById('file-list').children[0];row.dispatchEvent(new Event('dblclick'));await tick();await tick();
  document.getElementById('preview-next').dispatchEvent(new Event('click'));await tick();await tick();
  assert.equal(document.getElementById('preview-name').textContent,'second.txt');assert.equal(dialog.open,true);assert.equal(closes,0);
  dialog.dispatchEvent(new Event('close'));
  assert.equal(document.getElementById('preview-name').textContent,'second.txt');
  assert.match(document.getElementById('preview-content').textContent,/Example text/);
});

test('an empty bounded search page keeps continuation and never claims the search is complete',async()=>{
  const {host,document,calls}=fixture({search:'#search?q=target',query:p=>p.cursor
    ? {entries:[{...trashEntry('found'),name:'target.txt'}],cursor:null}
    : {entries:[],cursor:'opaque-continuation'}});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(document.getElementById('more-files').hidden,false);
  assert.match(document.getElementById('file-status').textContent,/järgmisi tulemusi/);
  document.getElementById('more-files').dispatchEvent(new Event('click'));await tick();
  assert.equal(calls.filter(([m])=>m==='query').at(-1)[1].cursor,'opaque-continuation');
  assert.equal(document.getElementById('file-list').children[0].dataset.fileId,'found');
});

test('the shared save picker accepts UUID space and destination without editor-specific translation',async()=>{
  const space='01234567-89ab-4cde-8fab-0123456789ab',parent='aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee';
  const {host,calls}=fixture({search:`?picker=save&app=gridcraft&channel=aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa&space=${space}&parent=${parent}&name=Draft`});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  assert.equal(calls.find(([m])=>m==='query')[1].parent,parent);
  assert.equal(host.window.location.pathname,`/pilveruum/folder/${parent}`);
});


test('preview arrow navigation preserves media controls, text scrolling and modified shortcuts',async()=>{
  const {host,document}=fixture({file:previewFile});
  await vm.runInContext(`(async()=>{${source}})()`,host.context);
  openPreview(document);await tick();
  const dialog=document.getElementById('preview-dialog'),next=document.getElementById('preview-next');
  let advances=0;next.click=()=>{advances++;};next.disabled=false;
  function key(target, modifiers={}) {
    const event=new Event('keydown',{cancelable:true});event.key='ArrowRight';
    Object.defineProperty(event,'target',{value:target});Object.assign(event,modifiers);dialog.dispatchEvent(event);return event;
  }
  for(const tag of ['audio','video','iframe','input','textarea','select'])key(document.createElement(tag));
  for(const className of ['preview-text','office-preview']){const node=document.createElement('div');node.className=className;key(node);}
  key(dialog,{altKey:true});key(dialog,{metaKey:true});assert.equal(advances,0);
  const event=key(dialog);assert.equal(advances,1);assert.equal(event.defaultPrevented,true);
  next.disabled=true;assert.equal(key(dialog).defaultPrevented,false);assert.equal(advances,1);
});
