import { driveHref } from './drive-browser.js';
// Read-only account projections; every request is scoped and revalidated by the service.
const activityLabels={Created:'Loodi',ContentSaved:'Salvestati sisu',Renamed:'Muudeti nime',Moved:'Liigutati',Trashed:'Viidi prügikasti',Restored:'Taastati',SharingChanged:'Muudeti jagamist',RevisionLabeled:'Muudeti versiooni märgistust',RevisionDeleted:'Kustutati vana versioon'};
export function createDriveInsights({host,title,body,call,element,button,bind,text,session,current,owner,size,date,error,open,history}) {
  let epoch=0,view=null,before=null,list=null,busy=false;
  function value(node,read){bind(node,()=>{node.textContent=read();});return node;}
  async function activity(more=false) {
    if(busy)return;busy=true;const mine=epoch,who=session();
    try {
      const page=await call('activity',{...(view.object?{id:view.object}:{}),before:more?before:null});
      if(mine!==epoch||!current(who))return;
      if(!more)list.replaceChildren();
      for(const item of page.items){
        const row=element('article','activity-row'),summary=element('div'),name=element('a');name.textContent=item.name;name.href=driveHref(item.trashed?'#prugikast':`#file/${item.object}`);
        name.addEventListener('click',e=>{if(!item.trashed){e.preventDefault();open(item.object);}});
        const action=element('span','',activityLabels[item.kind]||'Muudeti');summary.append(name,action);
        const detail=element('small','muted');value(detail,()=>`${item.actor===owner()?text('Mina'):item.actor} · ${date(item.atMs)}`);row.append(summary,detail);list.append(row);
      }
      before=page.before;
      if(!list.children.length)list.append(element('p','muted','Tegevusi pole veel.'));
      body.querySelector('.insight-more')?.remove();if(before)body.append(button('Näita rohkem',()=>activity(true),'button insight-more'));
    }catch(e){if(mine===epoch&&current(who)){list.append(element('p','is-error',error(e)),button('Proovi uuesti',()=>activity(more)));}}
    finally{if(mine===epoch)busy=false;}
  }
  async function storage() {
    const mine=epoch,who=session();
    try {
      const data=await call('storage');if(mine!==epoch||!current(who))return;body.replaceChildren();
      const stats=element('dl','storage-breakdown');
      for(const [label,key] of [['Praegune sisu','currentBytes'],['Vanad versioonid','historyBytes'],['Prügikast','trashBytes'],['Pooleliolevad üleslaadimised','pendingBytes']]){
        stats.append(element('dt','',label),value(element('dd'),()=>size(data[key])));
      }
      body.append(value(element('p','storage-total'),()=>text('{used} / {quota} kasutusel',{used:size(data.usedBytes),quota:size(data.quotaBytes)})),stats,
        element('h2','','Failid suuruse järjekorras'),element('p','muted','Suurus sisaldab kõiki säilitatud versioone.'));
      const files=element('div','storage-files');
      for(const file of data.files){
        const row=element('article','storage-row'),summary=element('div'),name=element('a');name.textContent=file.name;name.href=driveHref(file.trashed?'#prugikast':`#file/${file.id}`);
        name.addEventListener('click',e=>{if(!file.trashed){e.preventDefault();open(file.id);}});
        summary.append(name);if(file.trashed)summary.append(element('small','', 'Prügikast'));
        row.append(summary,value(element('span'),()=>size(file.bytes+file.historyBytes)));
        if(!file.trashed && file.versions)row.append(button('Versioonid',async()=>{try{const entry=await call('get',{id:file.id});if(current(who))history(entry);}catch(e){if(current(who))row.append(element('p','is-error',error(e)));}}));
        files.append(row);
      }
      if(!data.files.length)files.append(element('p','muted','Faile pole veel.'));body.append(files);
    }catch(e){if(mine===epoch&&current(who))body.replaceChildren(element('p','is-error',error(e)),button('Proovi uuesti',storage));}
  }
  function show(kind,object=null){epoch++;busy=false;view={kind,object};before=null;host.hidden=false;title.textContent='';bind(title,()=>{title.textContent=text(kind==='storage'?'Salvestusruum':'Tegevused');});body.replaceChildren(element('p','muted','Laadin…'));
    if(kind==='storage')return storage();
    body.replaceChildren(element('p','muted',object?'Selle faili või kausta muudatused.':'Sinu pilveruumi muudatused. Jagatud faili ajalugu avaneb faili teabest.'),element('p','muted','Iga faili kohta säilib kuni 100 viimast tegevust.'));
    list=element('div','activity-list');body.append(list);return activity();
  }
  return {show,hide(){epoch++;busy=false;view=null;host.hidden=true;body.replaceChildren();},refresh(){if(view)return show(view.kind,view.object);}};
}

export function createRevisionHistory({body,showDialog,dialogOpen,dialogEpoch,call,element,button,bind,text,field,session,current,owner,size,date,error,mutate,requestId,download,restore,picker,select,message}) {
  async function show(file) {
    showDialog('Versioonid',null);const epoch=dialogEpoch(),who=session(),route=location.hash;
    const valid=()=>dialogOpen()&&epoch===dialogEpoch()&&current(who);body.append(element('p','','Laadin…'));
    try {
      const revisions=await call('history',{id:file.id});if(!valid())return;body.replaceChildren();
      for(const revision of [...revisions].reverse()){
        const row=element('div','history-row'),info=element('div'),stamp=element('span');
        bind(stamp,()=>{stamp.textContent=`${date(revision.updatedMs)} · ${size(revision.bytes)}`;});
        if(revision.label){const name=element('strong');name.textContent=revision.label;info.append(name);}info.append(stamp);
        if(revision.id===file.revision)info.append(element('small','version-badge','Praegune versioon'));
        if(revision.keep)info.append(element('small','version-badge','Säilib püsivalt'));
        if(revision.automatic)info.append(element('small','version-badge','Automaatne salvestus'));
        const author=element('small','account-id');author.textContent=revision.author;info.append(author);row.append(info,picker()?button('Ava',()=>select(file,revision.id)):button('Laadi alla',()=>download(file,revision.id)));
        const token=requestId();
        if(!picker()&&file.capabilities.edit&&revision.id!==file.revision)row.append(button('Taasta versioon',()=>mutate(()=>restore(file,revision,token),{ids:[file.id],label:'Taastan…',quota:true})));
        if(!picker()&&file.owner===owner()){
          row.append(button('Nimeta / säilita',()=>label(file,revision)));
          if(revision.id!==file.revision){const remove=button('Kustuta versioon',()=>removeRevision(file,revision));remove.disabled=!!revision.keep||!!revision.label;row.append(remove);}
        }
        body.append(row);
      }
      if(!picker())body.append(element('p','muted','Taastamine lisab uue versiooni. Senine ajalugu säilib.'),element('p','muted','Nimega ja püsivaks märgitud versioonid säilivad. Käsitsi salvestusi automaatselt ei kustutata.'));
    }catch(e){if(valid())body.replaceChildren(element('p','is-error',error(e)));}
  }
  function label(file,revision){
    const token=requestId(),who=session(),route=location.hash;let input,keep;
    showDialog('Versiooni nimi ja säilitamine','Salvesta',async()=>{
      const ok=await mutate(()=>call('revisionLabel',{id:file.id,revision:revision.id,version:revision.propertiesVersion||'0',label:input.value,keep:keep.checked,requestId:token}),{ids:[file.id]});
      if(ok&&current(who)&&route===location.hash)show(file);
    });
    input=field('Versiooni nimi');input.value=revision.label||'';input.maxLength=160;
    const line=element('label','version-keep');keep=element('input');keep.type='checkbox';keep.checked=!!revision.keep;line.append(keep,element('span','','Säilita püsivalt'));body.append(line,element('p','muted','Nimega versioon säilib alati. Kustutamiseks eemalda nimi ja püsiv säilitamine.'));input.focus();
  }
  function removeRevision(file,revision){
    const token=requestId(),who=session(),route=location.hash;
    showDialog('Kustuta versioon','Kustuta jäädavalt',async()=>{
      const ok=await mutate(()=>call('revisionDelete',{id:file.id,revision:revision.id,version:file.version,propertiesVersion:revision.propertiesVersion||'0',requestId:token}),{ids:[file.id],quota:true});
      if(ok&&current(who)&&route===location.hash){try{const fresh=await call('get',{id:file.id});if(current(who)&&route===location.hash)show(fresh);}catch(e){if(current(who))message(error(e),true);}}
    });
    body.append(element('p','', 'See kustutab valitud vana versiooni jäädavalt. Praegune versioon jääb alles.'),element('p','',revision.label||''));
    const stamp=element('p');bind(stamp,()=>{stamp.textContent=`${date(revision.updatedMs)} · ${size(revision.bytes)}`;});body.append(stamp);
  }
  return {show};
}
