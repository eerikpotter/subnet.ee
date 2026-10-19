// Public reading has its own minimal projection; it cannot call account APIs.
export function createPublicLinks({host,body,read,download,call,element,button,text,bind,error,dialog,dialogBody,submit,field,requestId,mutate,dialogCurrent,href,preview,size,canRequest,requestAccess}) {
  let epoch=0;
  const fail=(target,e)=>target.append(element('p','is-error',error(e)));
  async function settings(file) {
    let execute;dialog('Üldine ligipääs','Salvesta',()=>execute?.());const active=dialogCurrent(),target=dialogBody();
    submit.disabled=true;
    target.append(element('p','','Laadin…'));
    try {
      const state=await call('publicLink',{id:file.id});if(!active())return;
      target.replaceChildren();
      const label=element('label','dialog-field'),mode=element('select');label.append(element('span','','Üldine ligipääs'),mode);target.append(label);
      for(const [value,title] of [['restricted','Piiratud'],['public','Kõik lingiga']]) {const option=element('option','',title);option.value=value;option.disabled=value==='public'&&!state.allowed;mode.append(option);}
      mode.value=state.enabled?'public':'restricted';
      if(!state.allowed)target.append(element('p','muted','Selle tiimi väline jagamine on keelatud.'));
      if(state.inherited)target.append(element('p','muted','Avalik ligipääs on päritud kaustast. Selle lõpetamiseks muuda ülemkausta jagamist.'));
      target.append(element('p','muted','Kõik lingiga saavad vaadata ja alla laadida ilma kontota. Muutmine vajab nimelist õigust.'));
      const expiry=field('Ligipääs kuni');expiry.type='datetime-local';expiry.required=false;
      if(state.expiresAt){const d=new Date(Number(state.expiresAt)/1e6);expiry.value=new Date(d.getTime()-d.getTimezoneOffset()*60000).toISOString().slice(0,16);}
      target.append(element('p','muted','Tühi kuupäev tähendab tähtajatut ligipääsu.'));
      const link=element('a');link.textContent=file.name;link.href=href(file);target.append(link);
      const token=requestId();let request;
      // Replace the pending submit callback only after policy/version have arrived.
      execute=()=>{
        if(!request)request={id:file.id,enabled:mode.value==='public',expiresAt:expiry.value?String(BigInt(new Date(expiry.value).getTime())*1000000n):null,version:state.version,requestId:token};
        mode.disabled=expiry.disabled=true;return mutate(()=>call('publicLinkSet',request),{ids:[file.id]});
      };
      submit.disabled=false;
    }catch(e){if(active()){target.replaceChildren();fail(target,e);}}
  }
  async function save(file) {
    const turn=epoch;
    try {
      const bytes=await download(file.id);if(turn!==epoch)return;
      const url=URL.createObjectURL(new Blob([bytes],{type:'application/octet-stream'}));
      const link=element('a');link.href=url;link.download=file.name;document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),30000);
    }catch(e){if(turn===epoch)fail(body,e);}
  }
  async function show(id) {
    const turn=++epoch;host.hidden=false;body.replaceChildren(element('p','','Laadin…'));
    try {
      const item=await read('get',{id});if(turn!==epoch)return;
      body.replaceChildren();const title=element('h1');title.textContent=item.name;
      body.append(title,element('p','muted','Jagatud vaatamiseks. Muutmine vajab nimelist õigust.'));
      if(!item.folder){body.append(button('Eelvaade',()=>preview(item)),button('Laadi alla',()=>save(item)));await preview(item);return;}
      const list=element('div','public-file-list');body.append(list);let cursor=null;
      const more=button('Näita rohkem',load);body.append(more);
      async function load(){more.disabled=true;try {
        const page=await read('list',{id,cursor});if(turn!==epoch)return;
        for(const file of page.entries){const row=element('div','public-file-row'),name=element('a');name.textContent=file.name;name.href=href(file);row.append(name);if(!file.folder)row.append(element('span','muted',size(file.bytes||0)),button('Laadi alla',()=>save(file)));list.append(row);}
        cursor=page.cursor;more.hidden=!cursor;if(!list.children.length&&!cursor)list.append(element('p','','Kaust on tühi.'));
      }catch(e){if(turn===epoch)fail(list,e);}finally{if(turn===epoch)more.disabled=false;}}
      await load();
    }catch(e){if(turn===epoch){body.replaceChildren(element('h1','','Link ei ole kättesaadav'),element('p','muted','Ligipääs võib olla piiratud, aegunud või lõpetatud. Võid sisse logida oma kontoga.'));if(canRequest())body.append(button('Küsi ligipääsu',()=>requestAccess(id)));}}
  }
  return {show,settings,save,hide(){epoch++;host.hidden=true;body.replaceChildren();}};
}
