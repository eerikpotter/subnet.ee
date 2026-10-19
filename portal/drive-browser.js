// Object routes share one canonical UUID contract with the Rust SDK.
export const OBJECT_UUID='[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}';
export function driveRoute(location) {
  if(location.hash)return location.hash;
  const match=new RegExp(`^/(?:en/)?pilveruum/(file|folder)/(${OBJECT_UUID})/?$`,'i').exec(location.pathname);
  return match?`#${match[1]}/${match[2].toLowerCase()}`:'';
}
export function driveHref(route,pathname=globalThis.location?.pathname||'/pilveruum/') {
  const base=pathname.startsWith('/en/')?'/en/pilveruum/':'/pilveruum/';
  const match=new RegExp(`^#(file|folder)/(${OBJECT_UUID})$`,'i').exec(route);
  return match?`${base}${match[1]}/${match[2].toLowerCase()}`:base+route;
}
export function canonicalDriveLocation(location) {
  const [path,hash]=driveHref(driveRoute(location),location.pathname).split('#');
  return path+(location.search||'')+(hash?'#'+hash:'');
}
// Presentation state only. Permissions and result ordering belong to the service.
export function menuPosition(anchor,menu,viewport) {
  const margin=12,width=Math.min(menu.width,viewport.width-2*margin),height=Math.min(menu.height,viewport.height-2*margin);
  const below=anchor.bottom+4,above=anchor.top-height-4;
  return {left:Math.max(margin,Math.min(anchor.right-width,viewport.width-width-margin)),
    top:Math.max(margin,Math.min(below+height<=viewport.height-margin?below:above,viewport.height-height-margin))};
}
export class DriveSelection {
  constructor() { this.ids=new Set(); this.anchor=null; this.focus=null; }
  clear() { this.ids.clear(); this.anchor=null; this.focus=null; }
  reconcile(order) {
    const visible=new Set(order);this.ids=new Set([...this.ids].filter(id=>visible.has(id)));
    if(!visible.has(this.anchor))this.anchor=null;
    if(!visible.has(this.focus))this.focus=order[0]||null;
  }
  select(id,order,{toggle=false,range=false}={}) {
    if(!order.includes(id))return;
    if(range && this.anchor && order.includes(this.anchor)) {
      if(!toggle)this.ids.clear();
      const [a,b]=[order.indexOf(this.anchor),order.indexOf(id)].sort((a,b)=>a-b);
      for(const item of order.slice(a,b+1))this.ids.add(item);
    } else if(toggle) {
      if(this.ids.has(id))this.ids.delete(id);else this.ids.add(id);
      this.anchor=id;
    } else {this.ids=new Set([id]);this.anchor=id;}
    this.focus=id;
  }
  all(order) {this.ids=new Set(order);this.anchor=order[0]||null;this.focus=this.focus||this.anchor;}
  step(order,delta,{range=false,focusOnly=false}={}) {
    if(!order.length)return null;
    const index=Math.max(0,order.indexOf(this.focus));
    const id=order[Math.max(0,Math.min(order.length-1,index+delta))];
    if(focusOnly)this.focus=id;else this.select(id,order,{range});
    return id;
  }
}
export const canManage=(file,principal)=>file?.spaceAccess?.manage ?? (file?.owner===principal);
export const defaultPreferences=()=>({view:'list',sort:'name',descending:false});
export function contentLocation(location,t) {
  if(location==='text')return t('Tekst');
  return location.split('/').map(part=>{
    const colon=part.indexOf(':');if(colon<0)return '';
    const kind=part.slice(0,colon),value=part.slice(colon+1);
    const label={page:'Lehekülg',slide:'Slaid',sheet:'Leht',paragraph:'Lõik',row:'Rida',cell:'Lahter',table:'Tabel'}[kind];
    return label?`${t(label)} ${value}`:'';
  }).filter(Boolean).join(' · ');
}
// UI affordances only; every operation is still authorized by the canister.
export function selectionCapabilities(files,principal,trash=false) {
  const all=predicate=>files.length>0 && files.every(predicate);
  return {
    open:!trash && files.length===1,
    link:!trash && files.length===1,
    copy:!trash && all(file=>!file.folder && !file.shortcut && file.capabilities.read),
    rename:!trash && files.length===1 && all(file=>file.capabilities.edit),
    download:!trash && all(file=>!file.shortcut && file.capabilities.read),
    move:!trash && all(file=>file.capabilities.edit && file.capabilities.delete && file.spaceId===files[0].spaceId),
    share:!trash && all(file=>canManage(file,principal)),
    trash:!trash && all(file=>file.capabilities.delete),
    restore:trash && all(file=>file.capabilities.delete),
    star:!trash && files.some(file=>!file.starred),
    unstar:!trash && files.some(file=>file.starred),
  };
}
// A folder action already includes its descendants. Resolve only ancestry needed
// to avoid moving/trashing a selected child twice, including in search results.
export async function selectionRoots(files,get) {
  const selected=new Set(files.map(file=>file.id)), cache=new Map(files.map(file=>[file.id,file]));
  const groups=new Map();
  for(const file of files) {
    let root=file, parent=file.parent;const seen=new Set([file.id]);
    while(parent && !seen.has(parent)) {
      seen.add(parent);
      if(seen.size>10_000)throw new Error('Selection ancestry limit reached');
      let ancestor=cache.get(parent);
      if(!ancestor) {ancestor=await get(parent);cache.set(parent,ancestor);}
      if(!ancestor)break; // A private ancestor is not exposed to this viewer.
      if(selected.has(parent))root=ancestor;
      parent=ancestor.parent;
    }
    if(!groups.has(root.id))groups.set(root.id,{file:root,ids:[]});
    groups.get(root.id).ids.push(file.id);
  }
  return [...groups.values()];
}
export function browseRoute(hash,root,principal,preferences) {
  const query={text:'',fileType:'',owner:null,afterMs:null,beforeMs:null,sort:preferences.sort,descending:preferences.descending,parent:null};
  let view='files';
  if(!hash || hash==='#kodu') {view='home';query.scope='suggested';}
  else if(hash.startsWith('#file/')) {view='link';query.scope='item';query.id=hash.slice(6);}
  else if(hash.startsWith('#folder/')) {query.scope='folder';query.parent=hash.slice(8);}
  else if(hash.startsWith('#content?')) {view='search';const p=new URLSearchParams(hash.slice(9));query.scope='content';query.text=p.get('q')||'';query.parent=p.get('in')||null;}
  else if(hash.startsWith('#search?')) {
    view='search';const p=new URLSearchParams(hash.slice(8));
    query.text=p.get('q')||'';query.fileType=p.get('type')||'';query.owner=p.get('owner')==='mine'?principal:null;
    for(const [param,key] of [['after','afterMs'],['before','beforeMs']]) {
      const value=p.get(param);if(value && /^\d{4}-\d{2}-\d{2}$/.test(value)) {const ms=Date.parse(value+'T00:00:00Z');if(Number.isFinite(ms))query[key]=ms;}
    }
    query.parent=p.get('in')||null;query.scope=query.parent?'subtree':'all';
  } else if(hash==='#jagatud') {view='shared';query.scope='shared';}
  else if(hash==='#hiljutised') {view='recent';query.scope='recent';query.sort=preferences.recentSort||'opened';query.descending=preferences.recentDescending??true;}
  else if(hash==='#tarniga') {view='starred';query.scope='starred';}
  else if(hash==='#prugikast') {view='trash';query.scope='trash';}
  else if(hash==='#rakendused' || hash==='#ai') {view=hash==='#ai'?'ai':'apps';}
  else {query.scope='folder';query.parent=root;}
  return {view,query};
}

// Roles are UI presets over the canonical capabilities, never new authority.
export function roleCapabilities(role,folder=false) {
  return {read:true,propose:false,create:role==='editor' && folder,edit:role==='editor',delete:role==='editor'};
}
export function sharingRole(capabilities,folder=false) {
  for(const role of ['viewer','editor']) {
    const expected=roleCapabilities(role,folder);
    if(Object.keys(expected).every(key=>!!capabilities[key]===expected[key]))return role;
  }
  return 'advanced';
}
export function stableSuggestionOrder(previous,current) {
  const next=new Map(current.map(item=>[item.id,item]));
  return [...previous.filter(item=>next.has(item.id)).map(item=>{const value=next.get(item.id);next.delete(item.id);return value;}),...next.values()];
}
