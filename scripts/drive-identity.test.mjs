import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../portal/drive-browser.js',import.meta.url),'utf8');
const {driveRoute,driveHref,canonicalDriveLocation,browseRoute,defaultPreferences,menuPosition}=await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

const file='01234567-89ab-4cde-8fab-0123456789ab';
const folder='aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee';
test('UUID links retain identity on a fresh load, reload and language change',()=>{
  for(const locale of ['', '/en']) {
    const url=new URL(`https://subnet.ee${locale}/pilveruum/file/${file}`);
    assert.equal(driveRoute(url),`#file/${file}`);
    assert.equal(canonicalDriveLocation(url),url.pathname);
    assert.deepEqual(browseRoute(driveRoute(url),folder,'alice',defaultPreferences()).query.id,file);
    assert.equal(driveHref(`#folder/${folder}`,url.pathname),`${locale}/pilveruum/folder/${folder}`);
  }
});
test('view navigation leaves the file path and preserves picker parameters',()=>{
  const url=new URL(`https://subnet.ee/pilveruum/file/${file}?picker=open#hiljutised`);
  assert.equal(canonicalDriveLocation(url),'/pilveruum/?picker=open#hiljutised');
  url.hash=`#folder/${folder}`;
  assert.equal(canonicalDriveLocation(url),`/pilveruum/folder/${folder}?picker=open`);
});
test('copies and ownership are not encoded into the address',()=>{
  assert.equal(driveHref(`#file/${file}`,'/pilveruum/'),`/pilveruum/file/${file}`);
  const copied='fedcba98-7654-4321-8abc-fedcba987654';
  assert.notEqual(driveHref(`#file/${file}`),driveHref(`#file/${copied}`));
  assert.equal(driveRoute(new URL('https://subnet.ee/pilveruum/file/8')),'');
});


test('file menus remain inside desktop and phone viewports, including the last row',()=>{
 for(const viewport of [{width:1280,height:720},{width:390,height:844}])for(const top of [16,viewport.height-40]){
  const menu={width:220,height:600},anchor={right:viewport.width-16,top,bottom:top+32};
  const placed=menuPosition(anchor,menu,viewport);
  assert.ok(placed.left>=12 && placed.left+menu.width<=viewport.width-12);
  assert.ok(placed.top>=12 && placed.top+menu.height<=viewport.height-12);
 }
});
