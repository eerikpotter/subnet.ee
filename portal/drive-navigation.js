import { canManage, driveHref } from './drive-browser.js';
// Transient, account-scoped views over the canonical browse/details API.
export function createFolderTree({host,toggle,call,element,button,text,session,current,navigate,active,error}) {
  let root=null,epoch=0,opened=false,focused=null;
  const nodes=new Map();
  function node(file) {if(!nodes.has(file.id))nodes.set(file.id,{file,items:[],cursor:null,loaded:false,expanded:false});else nodes.get(file.id).file=file;return nodes.get(file.id);}
  async function load(state,more=false) {
    if(state.busy)return;
    const mine=epoch,who=session();state.busy=true;state.error=null;render();
    try {
      const page=await call('query',{scope:'folder',parent:state.file.id,fileType:'folder',sort:'name',cursor:more?state.cursor:null});
      if(mine!==epoch || !current(who))return;
      state.items=more?[...state.items,...page.entries]:page.entries;
      state.cursor=page.cursor;state.loaded=true;
      for(const file of page.entries)node(file);
    } catch(e) {if(mine===epoch && current(who)){state.items=[];state.loaded=false;state.cursor=null;state.error=error(e);}}
    finally {if(mine===epoch && current(who)){state.busy=false;render();for(const file of (opened && (state.file.id===root?.id || state.expanded)?state.items:[])){const child=nodes.get(file.id);if(child.expanded && !child.loaded && !child.error)load(child);}}}
  }
  function expand(state,value) {state.expanded=value;if(value && !state.loaded)load(state);else render();}
  function render() {
    host.hidden=!root || !opened;toggle.hidden=!root;toggle.setAttribute('aria-expanded',String(!!root && opened));
    if(!root){host.replaceChildren();return;}
    const restoreFocus=host.contains(document.activeElement);host.replaceChildren();
    const visible=[];
    function branch(state,parent,level) {
      const item=element('div','folder-tree-item');item.setAttribute('role','treeitem');item.dataset.id=state.file.id;
      item.setAttribute('aria-level',String(level));item.setAttribute('aria-expanded',String(state.expanded));item.setAttribute('aria-selected',String(active()===state.file.id));
      item.tabIndex=focused===state.file.id || (!focused && !visible.length)?0:-1;
      const line=element('div','folder-tree-line');if(state.file.color)line.dataset.folderColor=state.file.color;
      const arrow=button('',()=>{focused=state.file.id;expand(state,!state.expanded);},'tree-arrow');arrow.textContent=state.expanded?'▾':'▸';arrow.tabIndex=-1;
      arrow.setAttribute('aria-label',state.expanded?text('Sulge kaust'):text('Ava alamkaustad'));
      const label=element('span','tree-name');label.textContent=state.file.name;line.append(arrow,label);item.append(line);parent.append(item);visible.push(item);
      line.addEventListener('click',event=>{if(event.target.closest('button'))return;focused=state.file.id;navigate(state.file.id);});
      item.addEventListener('focus',()=>{focused=state.file.id;});
      item.addEventListener('keydown',event=>{
        if(event.target!==item)return;
        const index=visible.indexOf(item);let next;
        if(event.key==='ArrowDown')next=visible[index+1];else if(event.key==='ArrowUp')next=visible[index-1];
        else if(event.key==='Home')next=visible[0];else if(event.key==='End')next=visible.at(-1);
        else if(event.key==='ArrowRight'){if(!state.expanded)expand(state,true);else next=visible.find(n=>n.dataset.id===state.items[0]?.id);}
        else if(event.key==='ArrowLeft'){if(state.expanded)expand(state,false);else next=visible.find(n=>n.dataset.id===state.file.parent);}
        else if(event.key==='Enter'){navigate(state.file.id);}else return;
        event.preventDefault();event.stopPropagation();if(next){focused=next.dataset.id;for(const n of visible)n.tabIndex=n===next?0:-1;next.focus();}
      });
      if(state.expanded){const group=element('div','folder-tree-group');group.setAttribute('role','group');item.append(group);children(state,group,level+1);}
    }
    function children(state,parent,level) {
      for(const file of state.items)branch(nodes.get(file.id),parent,level);
      if(state.busy)parent.append(element('span','tree-status','Laadin…'));
      else if(state.error){parent.append(element('span','tree-status is-error',state.error),button('Proovi uuesti',()=>load(state),'tree-more'));}
      else if(state.loaded && !state.items.length)parent.append(element('span','tree-status','Alamkaustu pole.'));
      if(state.cursor)parent.append(button('Näita rohkem kaustu',()=>load(state,true),'tree-more'));
    }
    children(nodes.get(root.id),host,1);
    if(!visible.some(n=>n.dataset.id===focused)){focused=visible[0]?.dataset.id||null;if(visible[0])visible[0].tabIndex=0;}
    if(restoreFocus)visible.find(n=>n.dataset.id===focused)?.focus({preventScroll:true});
  }
  toggle.addEventListener('click',()=>{opened=!opened;render();if(opened && root && !nodes.get(root.id).loaded)load(nodes.get(root.id));});
  return {
    setRoot(file){if(file?.id===root?.id)return;epoch++;nodes.clear();root=file;opened=false;focused=null;if(file)node(file);render();},
    reset(){epoch++;nodes.clear();root=null;opened=false;focused=null;render();},
    refresh(){epoch++;for(const s of nodes.values()){s.loaded=false;s.busy=false;s.error=null;s.items=[];s.cursor=null;}if(root && opened)load(nodes.get(root.id));else render();},
    render,
  };
}

export function createFileDetails({host,body,closeButton,title,call,element,button,text,bind,session,current,owner,size,date,error,history,share,role,activity}) {
  let epoch=0,selected=null,signature=null,returnFocus=null;
  async function show(file,focus=true) {
    const key=JSON.stringify([file.id,file.version]);if(!host.hidden && signature===key && !focus)return;
    const mine=++epoch,who=session();selected=file;signature=key;
    if(host.hidden)returnFocus=document.activeElement;
    host.hidden=false;title.textContent=file.name;body.replaceChildren(element('p','muted','Laadin…'));
    if(focus)closeButton.focus({preventScroll:true});
    try {
      const data=await call('details',{id:file.id});if(mine!==epoch || !current(who))return;
      const entry=data.entry;title.textContent=entry.name;body.replaceChildren();
      const list=element('dl','file-properties');
      const add=(name,value)=>{const dd=element('dd');bind(dd,()=>{dd.textContent=String(typeof value==='function'?value():value);});list.append(element('dt','',name),dd);};
      add('Tüüp',()=>entry.folder?text('Kaust'):entry.shortcut?text('Otsetee'):entry.format?.media_type||text('Fail'));
      add('Omanik',()=>entry.spaceAccess?.teamName||(entry.owner===owner()?text('Mina'):entry.owner));
      if(!entry.folder && !entry.shortcut){add('Suurus',()=>size(entry.bytes||0));add('Versioone',data.versions);}
      add('Loodud',()=>date(entry.createdMs));add('Muudetud',()=>date(entry.updatedMs));body.append(list);
      body.append(element('h3','','Asukoht'));
      const path=element('nav','details-location');bind(path,()=>path.setAttribute('aria-label',text('Kausta asukoht')));
      for(const p of data.locations){const link=element('a');if(p.id===entry.spaceId && entry.owner===owner())bind(link,()=>{link.textContent=text('Minu failid');});else link.textContent=p.name;link.href=driveHref(`#folder/${p.id}`);path.append(link);}
      if(!data.locations.length)path.append(element('p','muted','Ülemkausta asukoht pole saadaval.'));body.append(path);
      body.append(element('h3','','Minu ligipääs'));
      body.append(element('p','',entry.spaceAccess?.teamName?(canManage(entry,owner())?'Haldaja':role(entry)):entry.owner===owner()?'Omanik':role(entry)));
      if(!entry.spaceAccess?.teamName && entry.owner===owner())body.append(element('p','muted','Ligipääs tuleneb faili omandist.'));
      for(const access of data.access){
        const p=element('p','access-source');p.append(element('span','',access.inherited?'Päritud kaustast':'Jagatud otse'));
        if(access.inherited && access.location){const link=element('a');link.textContent=access.location.name;link.href=driveHref(`#folder/${access.location.id}`);p.append(link);}
        if(access.expiresMs){const expiry=element('span','muted');bind(expiry,()=>{expiry.textContent=text('Kehtib kuni')+' '+date(access.expiresMs);});p.append(expiry);}body.append(p);
      }
      const actions=element('div','details-actions');if(activity)actions.append(button('Tegevused',()=>activity(entry)));if(!entry.folder && !entry.shortcut && entry.capabilities.edit)actions.append(button('Versioonid',()=>history(entry)));
      if(!entry.folder&&!entry.shortcut) {
        const indexing=element('section','content-status');indexing.append(element('h3','','Sisuotsing'));body.append(indexing);
        let polls=0;
        const showIndex=async(rebuild=false)=>{try {
          const status=await call(rebuild?'contentRebuild':'contentStatus',{id:entry.id});if(mine!==epoch||!current(who))return;
          const labels={Pending:'Sisu indekseerimine on ootel.',Ready:'Sisu on otsitav.',Empty:'Failist ei leitud otsitavat teksti.',Partial:'Sisu on osaliselt indekseeritud.',Unsupported:'Selle vormingu sisuotsing pole veel toetatud.',TooLarge:'Fail ületab sisu indekseerimise 16 MB piiri.',Unreadable:'Faili teksti ei õnnestunud lugeda.',Failed:'Sisu indekseerimine ebaõnnestus.'};
          indexing.replaceChildren(element('h3','','Sisuotsing'),element('p','muted',labels[status.state]||'Sisu indekseerimine on ootel.'));
          if(status.state==='Failed'&&entry.capabilities.edit)indexing.append(button('Proovi uuesti',()=>showIndex(true)));
          if(status.state==='Pending'){indexing.append(button('Värskenda olekut',()=>showIndex()));if(polls++<20)setTimeout(()=>{if(mine===epoch&&current(who))showIndex();},3000);}
        }catch(e){if(mine===epoch&&current(who))indexing.replaceChildren(element('h3','','Sisuotsing'),element('p','muted',error(e)));}};
        showIndex();
      }
      if(canManage(entry,owner()))actions.append(button('Jaga',()=>share(entry)));body.append(actions);
    }catch(e){if(mine===epoch && current(who)){signature=null;body.replaceChildren(element('p','is-error',error(e)));}}
  }
  function close(focus=true){epoch++;selected=null;signature=null;host.hidden=true;body.replaceChildren();if(focus)returnFocus?.focus?.({preventScroll:true});}
  closeButton.addEventListener('click',()=>close());host.addEventListener('keydown',e=>{if(e.key==='Escape'){e.stopPropagation();close();}});
  return {show,close,refresh(){if(selected){signature=null;show(selected,false);}},selection(files){if(!host.hidden && files.length===1 && !files[0].trashed)show(files[0],false);}};
}
