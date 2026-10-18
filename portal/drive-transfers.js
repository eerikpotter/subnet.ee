// Ephemeral, account-scoped work. Catalog, permission and publication authority
// remain in the SDK/canister. A retry retains the original operation and target.
export const TRANSFER_ITEM_LIMIT = 1000;
export class TransferSkipped extends Error {}
export class TransferCancelled extends Error {}
export class TransferQueue {
  constructor({changed=()=>{}}={}) { this.jobs=[];this.changed=changed;this.running=false; }
  add(jobs) {
    if(this.jobs.length+jobs.length>TRANSFER_ITEM_LIMIT)throw new RangeError('Too many transfer items');
    for(const job of jobs)this.jobs.push({...job,status:'queued',progress:0,cancelled:false,error:null});
    this.changed(this);return this.drain();
  }
  cancel(job) {if(['queued','failed'].includes(job.status))job.status='cancelled';job.cancelled=true;job.onCancel?.();this.changed(this);}
  retry(job) {if(job.status!=='failed' || !job.valid())return;job.cancelled=false;job.status='queued';job.error=null;this.changed(this);return this.drain();}
  clear() {this.jobs=this.jobs.filter(j=>['queued','running'].includes(j.status));this.changed(this);}
  reset() {for(const job of this.jobs)this.cancel(job);this.jobs=[];this.changed(this);}
  async drain() {
    if(this.running)return;
    this.running=true;
    try {
      let job;
      while((job=this.jobs.find(j=>j.status==='queued'))) {
        if(!job.valid()){job.status='cancelled';continue;}
        job.status='running';this.changed(this);
        const control={
          check:()=>{if(job.cancelled || !job.valid())throw new TransferCancelled();},
          progress:(done,total)=>{job.progress=total?done/total:0;this.changed(this);return !job.cancelled && job.valid();},
        };
        try {await job.run(control);job.status='done';job.progress=1;}
        catch(error) {
          job.status=error instanceof TransferSkipped?'skipped':error instanceof TransferCancelled || (job.cancelled && String(error).includes('Upload cancelled'))?'cancelled':'failed';
          job.error=error;
        }
        this.changed(this);
      }
    } finally {this.running=false;this.changed(this);}
  }
}
export function relativePath(value) {
  const parts=String(value).split('/');
  if(!parts.length || parts.length>63 || parts.some(part=>!part || part!==part.trim() || part==='.' || part==='..' || /[\\\x00-\x1f\x7f]/.test(part) || new TextEncoder().encode(part.normalize('NFC')).length>160))throw new Error('Invalid folder path');
  return parts.map(part=>part.normalize('NFC'));
}
export function localFiles(files) {
  const result=new Map();
  for(const file of files) {
    const path=relativePath(file.webkitRelativePath||file.name);
    for(let i=1;i<path.length;i++) {
      const key=path.slice(0,i).join('/');if(result.has(key) && !result.get(key).folder)throw new Error('Duplicate upload path');
      result.set(key,{path:path.slice(0,i),folder:true});
    }
    const key=path.join('/');if(result.has(key))throw new Error('Duplicate upload path');
    result.set(key,{path,folder:false,file});
    if(result.size>TRANSFER_ITEM_LIMIT)throw new RangeError('Too many transfer items');
  }
  return [...result.values()];
}
// Capture DataTransfer entries synchronously: the browser's drop store is transient.
export function droppedFiles(data) {
  const entries=Array.from(data.items||[]).filter(item=>item.kind==='file').map(item=>item.webkitGetAsEntry?.());
  const fallback=Array.from(data.files||[]);
  return (async()=>{
    if(!entries.length || entries.some(entry=>!entry))return localFiles(fallback);
    const result=[];
    async function visit(entry,parent=[]) {
      const path=relativePath([...parent,entry.name].join('/'));
      if(result.length>=TRANSFER_ITEM_LIMIT)throw new RangeError('Too many transfer items');
      if(entry.isFile) {
        const file=await new Promise((resolve,reject)=>entry.file(resolve,reject));result.push({path,file,folder:false});
      } else if(entry.isDirectory) {
        result.push({path,folder:true});const reader=entry.createReader();
        while(true) {
          const batch=await new Promise((resolve,reject)=>reader.readEntries(resolve,reject));
          if(!batch.length)break;
          for(const child of batch)await visit(child,path);
        }
      } else throw new Error('Unsupported dropped item');
    }
    for(const entry of entries)await visit(entry);
    return result;
  })();
}
export function keepBothName(name,existing) {
  const used=new Set(existing.map(item=>item.name.normalize('NFC'))),dot=name.lastIndexOf('.');
  const stem=dot>0?name.slice(0,dot):name,ext=dot>0?name.slice(dot):'';
  let candidate=name,n=1;
  while(used.has(candidate))candidate=`${stem} (${++n})${ext}`;
  relativePath(candidate);return candidate;
}
export async function listChildren(list,parent,check=()=>{}) {
  const items=[],cursors=new Set();let cursor=null;
  do {
    check();const page=await list(parent,cursor);check();items.push(...page.entries);cursor=page.cursor;
    if(items.length>10000 || (cursor && cursors.has(cursor)))throw new RangeError('Folder listing too large');
    if(cursor)cursors.add(cursor);
  }while(cursor);
  return items;
}
// A folder is created once, including after a lost response. Successful siblings
// are not replayed. Source File objects stay in this tab; no file bytes are persisted.
export function uploadJobs(items,parent,api) {
  const directories=new Map(),records=new Map();
  for(const item of items) {
    let key=item.path.join('/');if(records.has(key)) {if(item.folder || item.path.length>1)throw new Error('Duplicate upload path');key+=':'+records.size;}
    records.set(key,{...item,requestId:api.id()});
  }
  function destination(path) {
    if(!path.length)return parent;
    // Parent jobs precede their children. A child must not silently retry a
    // failed parent or recreate a cancelled folder under a different job's control.
    for(let i=1;i<=path.length;i++) {
      const record=records.get(path.slice(0,i).join('/'));
      if(!record?.folder)throw new Error('Missing parent folder');
      if(record.cancelled)throw new TransferCancelled();
      if(record.skipped)throw new TransferSkipped();
    }
    const key=path.join('/');if(directories.has(key))return directories.get(key);
    throw new Error('Parent folder is not ready. Retry it first.');
  }
  async function run(item,control) {
    control.check();if(item.cancelled)throw new TransferCancelled();if(item.skipped)throw new TransferSkipped();
    const target=destination(item.path.slice(0,-1));
    if(item.folder && directories.has(item.path.join('/')))return directories.get(item.path.join('/'));
    if(!item.target) {
      const children=await listChildren(api.list,target,control.check),name=item.path.at(-1),existing=children.find(child=>child.name===name);
      let choice=existing?await api.collision(item,existing):'new';control.check();
      if(choice==='skip'){item.skipped=true;throw new TransferSkipped();}
      if(choice==='merge' && item.folder && existing?.folder) {directories.set(item.path.join('/'),existing.id);return existing.id;}
      const nextName=choice==='keep'?keepBothName(name,children):name;
      if(choice==='replace' && (item.folder || existing?.folder || !existing?.capabilities.edit))throw new Error('Cannot replace this item');
      item.target=choice==='replace'?{file:existing.id,revision:existing.revision,requestId:item.requestId}:{parent:target,name:nextName,requestId:item.requestId};
    }
    if(item.folder) {
      const entry=await api.folder(item.target);directories.set(item.path.join('/'),entry.id);return entry.id;
    }
    if(item.file.size>64*1024*1024)throw new Error('Files must be at most 64 MiB');
    const bytes=new Uint8Array(await item.file.arrayBuffer());control.check();
    const result=await api.upload({...api.format(item.file),...item.target},bytes,control.progress);
    return result;
  }
  return [...records.values()].sort((a,b)=>a.path.length-b.path.length || Number(b.folder)-Number(a.folder)).map(item=>({name:item.path.join('/'),valid:api.valid,onCancel:()=>{item.cancelled=true;},run:control=>run(item,control)}));
}
// Bounded traversal, shared by hierarchical download. Parent/space checks reject
// stale or malformed catalogs before constructing paths; the SDK verifies bytes.
export async function walkFiles(roots,list,check=()=>{}) {
  const result=[],seen=new Set();
  async function visit(item,path,depth) {
    check();if(depth>63 || seen.has(item.id) || result.length>=10000)throw new RangeError('Folder traversal too large');
    seen.add(item.id);relativePath(path.join('/'));result.push({...item,path:path.join('/')});
    if(item.folder)for(const child of await listChildren(list,item.id,check)) {
      if(child.parent!==item.id || child.spaceId!==item.spaceId)throw new Error('Folder changed; refresh and retry');
      await visit(child,[...path,child.name],depth+1);
    }
  }
  for(const item of roots)await visit(item,[item.name],0);
  return result;
}
