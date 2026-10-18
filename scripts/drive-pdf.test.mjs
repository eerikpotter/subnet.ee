import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {Element} from './i18n-dom-helpers.mjs';
const {createPdfPreview}=await import('data:text/javascript;base64,'+Buffer.from(readFileSync(new URL('../portal/drive-pdf.js',import.meta.url))).toString('base64'));
const tick=()=>new Promise(resolve=>setImmediate(resolve));

function fixture(options={}) {
  const controls=new Map(),pages=[],renders=[];let destroyed=0,acknowledged=0,cancelled=0,parameters;
  const element=(tag,cls,value)=>{const e=new Element(tag);e.className=cls||'';e.textContent=value||'';e.getContext=()=>({});return e;};
  const host=element('div');host.clientWidth=1000;
  const pdf={GlobalWorkerOptions:{},getDocument(input){parameters=input;return {promise:options.loading||Promise.resolve({numPages:3,async getPage(number){pages.push(number);return {
    getViewport:({scale})=>({width:3000*scale,height:4000*scale}),cleanup(){},
    render(params){renders.push(params);return {promise:options.rendering||Promise.resolve(),cancel(){cancelled++;}};},
    async getTextContent(){return {items:[{str:'<script>inert PDF text</script>'}]};},
  };}}),destroy:async()=>{destroyed++;}};}};
  const viewer=createPdfPreview({host,element,button:(name,action)=>{const e=element('button','',name);e.click=action;controls.set(name,e);return e;},text:(key,params={})=>key.replace(/\{(\w+)\}/g,(_,k)=>params[k]),current:options.current||(()=>true),ready:()=>{acknowledged++;},error:failure=>{throw failure;},load:async()=>pdf});
  return {viewer,host,controls,pages,renders,read:()=>({destroyed,acknowledged,cancelled,parameters})};
}
test('PDF preview renders one bounded page at a time with inert text and local assets',async()=>{
  const f=fixture();await f.viewer.open(new Uint8Array([1]));
  assert.equal(f.read().acknowledged,1);assert.equal(f.controls.get('Eelmine lehekülg').disabled,true);
  assert.match(f.host.textContent,/<script>inert PDF text<\/script>/);
  const before=f.renders[0].viewport;
  await f.controls.get('Järgmine lehekülg').click();await tick();assert.deepEqual(f.pages,[1,2]);
  f.controls.get('Suurenda').click();await tick();assert.ok(f.renders.at(-1).viewport.width>before.width);
  assert.ok(f.renders.every(r=>r.viewport.width*r.viewport.height<=4_000_001));
  assert.equal(f.read().acknowledged,1);
  assert.equal(f.read().parameters.isEvalSupported,false);
  assert.match(f.read().parameters.standardFontDataUrl,/^\/vendor\//);
  f.viewer.close();assert.equal(f.read().destroyed,1);
});
test('closing a pending PDF destroys its worker and late parsing cannot repopulate the preview',async()=>{
  let finish;const f=fixture({loading:new Promise(resolve=>{finish=resolve;})});
  const pending=f.viewer.open(new Uint8Array([1]));await tick();f.viewer.close();
  finish({numPages:1,getPage(){throw new Error('A closed preview must not read a page');}});await pending;
  assert.equal(f.read().destroyed,1);assert.equal(f.host.children.length,0);assert.equal(f.read().acknowledged,0);
});
test('account changes discard a pending page render and never acknowledge another accounts preview',async()=>{
  let finish,valid=true;const f=fixture({current:()=>valid,rendering:new Promise(resolve=>{finish=resolve;})});
  const pending=f.viewer.open(new Uint8Array([1]));await tick();valid=false;f.viewer.close();finish();await pending;
  assert.equal(f.read().cancelled,1);assert.equal(f.read().acknowledged,0);
});
