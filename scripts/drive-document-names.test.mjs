import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const {documentDisplayName,documentStorageName}=await import('data:text/javascript;base64,'+readFileSync(new URL('../portal/drive-browser.js',import.meta.url)).toString('base64'));
const app={cloudExtension:'wcraft.json'};
test('native document names hide the complete extension without changing stored identity',()=>{
 const file={id:'same-id',name:'Nimetu.wcraft.json'};
 assert.equal(documentDisplayName(file,app),'Nimetu');
 assert.equal(documentStorageName('Minu tekst',file,app),'Minu tekst.wcraft.json');
 assert.equal(documentStorageName('Minu tekst.WCRAFT.JSON',file,app),'Minu tekst.WCRAFT.JSON');
 assert.deepEqual(file,{id:'same-id',name:'Nimetu.wcraft.json'});
});
test('folders, unknown formats and dots in the title are preserved',()=>{
 assert.equal(documentDisplayName({name:'Notes.wcraft.json',folder:true},app),'Notes.wcraft.json');
 assert.equal(documentDisplayName({name:'data.json'},null),'data.json');
 assert.equal(documentDisplayName({name:'Version 1.2.wcraft.json'},app),'Version 1.2');
 assert.equal(documentStorageName('Folder',{folder:true},app),'Folder');
});
