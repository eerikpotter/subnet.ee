import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const { workspaceReturn, creationContext, editorContext, returnStateKey } = await import('data:text/javascript;base64,'+readFileSync(new URL('../portal/workspace-context.js',import.meta.url)).toString('base64'));
const origin='https://subnet.ee';
test('return paths preserve folder/search and reject off-site, non-workspace and picker destinations',()=>{
 for(const path of ['/#folder/abc','/en/#search?q=plans&sort=name','/file/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee']) assert.equal(workspaceReturn(path,origin),path);
 for(const path of ['https://evil.test/','//evil.test/','javascript:alert(1)','/apps/wordcraft/','/?picker=save','/../apps/wordcraft/']) assert.equal(workspaceReturn(path,origin),null);
});
test('editor context retains object addressing and account-specific restore keys',()=>{
 const location={origin,pathname:'/',search:'',hash:'#search?q=plans'};
 const url=new URL(editorContext('/apps/wordcraft/?cloud_file=abc&cloud_space=xyz',location),origin);
 assert.equal(url.searchParams.get('cloud_file'),'abc');assert.equal(url.searchParams.get('cloud_space'),'xyz');
 assert.equal(url.searchParams.get('workspace_return'),'/#search?q=plans');
 assert.notEqual(returnStateKey('alice',location),returnStateKey('bob',location));
});

test('new presentations use the declared blank startup while retaining destination',()=>{
 const apps=JSON.parse(readFileSync(new URL('../registry/apps.json',import.meta.url),'utf8'));
 const app=apps.find(app=>app.id==='deckcraft');
 const url=new URL(creationContext(app,{spaceId:'space',id:'folder'},{origin,pathname:'/',search:'',hash:'#folder/folder'}),origin);
 assert.equal(url.searchParams.get('blank'),'');
 assert.equal(url.searchParams.get('cloud_parent'),'folder');
 assert.equal(url.searchParams.get('cloud_space'),'space');
});
