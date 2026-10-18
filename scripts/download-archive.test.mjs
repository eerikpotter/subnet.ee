import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import vm from 'node:vm';

const context=vm.createContext({Blob,TextEncoder});
vm.runInContext(readFileSync(new URL('../portal/download-archive.js',import.meta.url),'utf8').replace(/^export /gm,''),context);
const archive=vm.runInContext('downloadArchive',context);
test('standard ZIP reader extracts original bytes, UTF-8, safe names, duplicates and empty files',async()=>{
  const bytes=new Uint8Array([0,255,7,3]),files=[
    {name:'Tõde.txt',bytes},{name:'Tõde.txt',bytes:new Uint8Array()},
    {name:'../escape.txt',bytes},{name:'..\\escape.txt',bytes},
  ];
  const blob=archive(files);assert.equal(blob.type,'application/zip');
  const result=spawnSync('python3',['-c',`
import io, sys, zipfile
z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read()))
assert z.testzip() is None
assert z.namelist() == ['Tõde.txt','Tõde (2).txt','_escape.txt','_escape (2).txt']
assert z.read('Tõde.txt') == bytes([0,255,7,3])
assert z.read('Tõde (2).txt') == b''
assert all(i.compress_type == zipfile.ZIP_STORED for i in z.infolist())
`],{input:Buffer.from(await blob.arrayBuffer())});
  assert.equal(result.status,0,result.stderr.toString());
});
test('archive bounds reject oversized buffers and names before encoding',()=>{
  assert.throws(()=>archive([{name:'large',bytes:{byteLength:256*1024*1024}}]),/Archive too large/);
  assert.throws(()=>archive([{name:'a'.repeat(65536),bytes:new Uint8Array()}]),/Archive too large/);
  assert.throws(()=>archive(Array(10001)),/Archive too large/);
});
test('independent ZIP reader preserves nested UTF-8 paths, empty folders and case-colliding root folders',async()=>{
  const blob=archive([
    {path:'Root',directory:true}, {path:'Root/Empty',directory:true},
    {path:'Root/Nested/Tõde.txt',bytes:new Uint8Array([42])},
    {path:'root',directory:true}, {path:'root/a.txt',bytes:new Uint8Array([7])},
  ]);
  const result=spawnSync('python3',['-c',`
import io, sys, zipfile
z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read()))
assert z.testzip() is None
assert z.getinfo('Root/Empty/').is_dir()
assert z.read('Root/Nested/Tõde.txt') == b'*'
assert z.read('root (2)/a.txt') == bytes([7])
`],{input:Buffer.from(await blob.arrayBuffer())});
  assert.equal(result.status,0,result.stderr.toString());
  for(const path of ['../escape','/absolute','Root/../escape','Root\\escape'])assert.throws(()=>archive([{path,bytes:new Uint8Array()}]),/Invalid archive path/);
});
