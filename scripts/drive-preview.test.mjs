import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const moduleUrl=source=>'data:text/javascript;base64,'+Buffer.from(source).toString('base64');
const archiveUrl=moduleUrl(readFileSync(new URL('../portal/vendor/fflate/browser.js',import.meta.url),'utf8'));
const {zipSync,strToU8}=await import(archiveUrl);
const {previewFormat,readOfficeParts,createThumbnails}=await import(moduleUrl(readFileSync(new URL('../portal/drive-preview.js',import.meta.url),'utf8').replace("'./vendor/fflate/browser.js'",JSON.stringify(archiveUrl))));
test('Office preview only reads explicit XML parts and orders numbered slides naturally',async()=>{
 const bytes=zipSync({'ppt/slides/slide10.xml':strToU8('<p/>'),'ppt/slides/slide2.xml':strToU8('<p/>'),'ppt/media/image.png':new Uint8Array(3_000_000),'ppt/vbaProject.bin':strToU8('untrusted')});
 assert.deepEqual((await readOfficeParts(bytes,'pptx')).map(p=>p.name),['ppt/slides/slide2.xml','ppt/slides/slide10.xml']);
});
test('Office preview rejects XML entities, oversized parts, invalid ZIP and unrelated content',async()=>{
 await assert.rejects(readOfficeParts(zipSync({'word/document.xml':strToU8('<!DOCTYPE foo [<!ENTITY x SYSTEM "file:///secret">]><p>&x;</p>')}),'docx'));
 await assert.rejects(readOfficeParts(zipSync({'word/document.xml':new Uint8Array(2*1024*1024+1)}),'docx'));
 await assert.rejects(readOfficeParts(new Uint8Array([1,2,3]),'xlsx'));
 await assert.rejects(readOfficeParts(zipSync({'outside.xml':strToU8('<p/>')}),'docx'));
});
test('preview support distinguishes inert browser formats from native Craft documents',()=>{
 assert.equal(previewFormat({name:'file.pdf',format:{craft:{}}}).kind,'pdf');
 assert.equal(previewFormat({name:'doc.docx'}).kind,'office');
 assert.equal(previewFormat({name:'photo.pcraft',format:{craft:{}}}),null);
 assert.equal(previewFormat({name:'page.html'}).kind,'text');
});

test('thumbnail work is bounded and account changes discard in-flight image bytes',async()=>{
 const previous=globalThis.IntersectionObserver,observers=[],pending=[];let downloads=0;
 globalThis.IntersectionObserver=class {constructor(callback){this.callback=callback;observers.push(this);}observe(){}disconnect(){}};
 try{
  const files=Array.from({length:30},(_,i)=>({id:String(i),name:'image.png',revision:'1',bytes:100}));
  const rows=new Map(files.map(f=>[f.id,{row:{querySelector:()=>({querySelector:()=>null})}}]));
  const thumbs=createThumbnails({download:()=>{downloads++;return new Promise(resolve=>pending.push(resolve));},current:()=>false,element:()=>{throw new Error('A stale image must not render');}});
  thumbs.update(files,rows,true,{principal:'a'});assert.equal(observers.length,16);
  for(const observer of observers)observer.callback([{isIntersecting:true}]);assert.equal(downloads,2);
  thumbs.clear();for(const resolve of pending)resolve(new Uint8Array([1]));
  await new Promise(resolve=>setImmediate(resolve));assert.equal(downloads,2);
 }finally{globalThis.IntersectionObserver=previous;}
});
