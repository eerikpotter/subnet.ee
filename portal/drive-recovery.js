// One completed action, held only in the active account/tab. Receipts are
// expectations for fresh commands, never authority or snapshots to restore.
export class LastDriveAction {
  constructor(){this.revision=0;this.clear();}
  clear(){this.group=null;this.items=[];this.revision++;}
  record(group,file,request,receipt){
    if(!receipt?.version || receipt.version===file.version || !['rename','move','trash'].includes(request.change))return;
    if(this.group!==group){this.clear();this.group=group;}
    if(this.items.some(item=>item.file.id===file.id))return;
    const inverse=request.change==='rename'?{change:'rename',name:file.name}:request.change==='move'?{change:'move',parent:file.parent}:{change:'restoreTo',parent:file.parent,name:file.name};
    this.items.push({file:{...file,version:receipt.version},inverse});
    this.revision++;
  }
  complete(id){this.items=this.items.filter(item=>item.file.id!==id);this.revision++;}
}
export function impactKey(impact){return JSON.stringify([impact.version,impact.affectedObjects,impact.affectedSubjects,impact.changes]);}
export async function recoveryPreview(call,item){
  if(item.inverse.change==='rename')return null;
  const result=await call(item.inverse.change==='move'?'movePreview':'restorePreview',{id:item.file.id,...item.inverse});
  if(result.version!==item.file.version)throw new Error('This item changed; refresh and try again');
  return result;
}
