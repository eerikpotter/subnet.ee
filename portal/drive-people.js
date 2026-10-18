import { driveHref } from './drive-browser.js';
// Account communication has its own view lifecycle; none of these records grants access.
export function createDrivePeople({host,title,body,call,element,button,bind,text,session,current,error,showDialog,dialogBody,field,mutate,requestId,dialogEpoch}) {
  let epoch=0,view=null;
  const stamp=node=>bind(node,()=>{node.textContent=new Date(Number(node.dataset.time)).toLocaleString(window.SubnetI18n.locale);});
  async function show(kind='inbox') {
    view=kind;const mine=++epoch,who=session();host.hidden=false;
    bind(title,()=>{title.textContent=text(kind==='safety'?'Peidetud ja blokeeritud':kind==='contacts'?'Minu kontaktid':'Teavitused');});body.replaceChildren(element('p','muted','Laadin…'));
    const valid=()=>mine===epoch&&current(who);
    try {
      if(kind==='safety') {
        const data=await call('sharingSafety');if(!valid())return;body.replaceChildren(element('h2','','Blokeeritud kontod'));
        for(const principal of data.blocked){const row=element('article','contact-row'),id=element('code','account-id');id.textContent=principal;row.append(id,button('Eemalda blokeering',()=>block(principal,false)));body.append(row);}
        if(!data.blocked.length)body.append(element('p','muted','Blokeeritud kontosid pole.'));
        body.append(element('h2','','Peidetud üksused'));
        for(const item of data.hidden){const row=element('article','contact-row'),name=element('span');name.textContent=item.name||text('Üksus ei ole kättesaadav');row.append(name,button('Näita uuesti',async()=>{try{await call('hideShared',{id:item.id,hidden:false});if(valid())show(kind);}catch(e){if(valid())row.append(element('p','is-error',error(e)));}}));body.append(row);}
        if(!data.hidden.length)body.append(element('p','muted','Peidetud üksusi pole.'));return;
      }
      if(kind==='contacts') {
        const contacts=await call('contacts');if(!valid())return;
        body.replaceChildren(element('p','muted','Kontaktinimed on nähtavad ainult sulle. Õigused seotakse alati konto ID-ga.'),button('Lisa kontakt',()=>editContact()));
        for(const contact of contacts){const row=element('article','contact-row'),name=element('strong'),id=element('code','account-id');name.textContent=contact.label;id.textContent=contact.principal;row.append(name,id,button('Muuda',()=>editContact(contact)));body.append(row);}
        return;
      }
      const data=await call('inbox');if(!valid())return;body.replaceChildren();
      const settings=element('label','notification-setting'),enabled=element('input');enabled.type='checkbox';enabled.checked=data.notifications;
      settings.append(enabled,element('span','','Jagamise märguanded'));body.append(settings,element('p','muted','Ligipääsutaotlused kuvatakse alati. Säilib kuni 100 viimast teavitust.'));
      enabled.addEventListener('change',async()=>{enabled.disabled=true;try{await call('notifications',{enabled:enabled.checked});}catch(e){if(valid()){enabled.checked=!enabled.checked;body.append(element('p','is-error',error(e)));}}finally{if(valid())enabled.disabled=false;}});
      for(const item of data.items){
        const row=element('article','inbox-row');row.classList.toggle('is-unread',!item.read);
        const description=element('div'),name=element('strong');name.textContent=item.name||text('Ligipääsutaotlus');
        const labels={Shared:'Sinuga jagati faili või kausta',AccessRequested:'Soovib ligipääsu',AccessDeclined:'Ligipääsutaotlus lükati tagasi',Reminder:'Jagatud faili meeldetuletus'};
        description.append(name,element('p','',labels[item.kind]||'Teavitus'));const sender=element('code','account-id');sender.textContent=item.sender;description.append(sender);
        if(item.message){const note=element('p','request-message');note.textContent=item.message;description.append(note);}
        const date=element('small','muted');date.dataset.time=item.atMs;stamp(date);description.append(date);row.append(description);
        if(item.canRespond)row.append(button('Vaata taotlust',()=>decide(item)));
        else if(item.name){const link=element('a','button','Ava');link.href=driveHref(`#file/${item.object}`);row.append(link);}
        if(!item.read)row.append(button('Märgi loetuks',async()=>{try{await call('readNotice',{id:item.id});if(valid())show(kind);}catch(e){if(valid())row.append(element('p','is-error',error(e)));}}));
        body.append(row);
      }
      if(!data.items.length)body.append(element('p','muted','Teavitusi veel pole.'));
    } catch(e){if(valid())body.replaceChildren(element('p','is-error',error(e)),button('Proovi uuesti',()=>show(kind)));}
  }
  function editContact(contact={}) {
    let principal,label;
    showDialog('Kontakti nimi','Salvesta',()=>mutate(()=>call('contact',{principal:principal.value.trim(),label:label.value.trim()})));
    principal=field('Konto ID',contact.principal||'');principal.disabled=!!contact.principal;
    label=field('Kontakti nimi',contact.label||'');label.maxLength=100;label.required=false;
    dialogBody().append(element('p','muted','Tühi nimi eemaldab kontakti. Jagamisõigused jäävad alles.'));
  }
  function decide(item) {
    let choice,role,expiry;const token=requestId();let request;
    showDialog('Ligipääsutaotlus','Salvesta otsus',()=>{
      if(!request)request={id:item.object,requester:item.sender,requestedAt:item.at,approve:choice.value==='approve',role:role.value,expiresAt:expiry.value?String(BigInt(new Date(expiry.value).getTime())*1000000n):null,requestId:token};
      choice.disabled=role.disabled=expiry.disabled=true;return mutate(()=>call('decideAccess',request));
    });
    const name=element('strong');name.textContent=item.name;dialogBody().append(name);
    const who=element('code','account-id');who.textContent=item.sender;dialogBody().append(who);
    if(item.message){const message=element('p');message.textContent=item.message;dialogBody().append(message);}
    function select(label,options){const wrapper=element('label','dialog-field',label),select=element('select');for(const [value,title] of options){const option=element('option','',title);option.value=value;select.append(option);}select.value=options[0][0];wrapper.append(select);dialogBody().append(wrapper);return select;}
    choice=select('Otsus',[['approve','Anna ligipääs'],['decline','Lükka tagasi']]);role=select('Ligipääsu roll',[['viewer','Vaataja'],['editor','Muutja']]);
    expiry=field('Ligipääs kuni');expiry.type='datetime-local';expiry.required=false;
    choice.addEventListener('change',()=>{role.disabled=expiry.disabled=choice.value==='decline';});
  }
  function requestAccess(id) {
    let message,locked;
    showDialog('Küsi ligipääsu','Saada taotlus',async()=>{
      if(locked!==undefined)return;const who=session();locked=message.value;
      try{await call('requestAccess',{id,message:locked});if(current(who)&&mine===dialogEpoch()){showDialog('Taotlus saadetud',null);dialogBody().append(element('p','','Kui üksus on olemas ja taotlus on lubatud, näeb omanik seda oma teavitustes.'));}}
      catch(e){locked=undefined;if(current(who)&&mine===dialogEpoch())dialogBody().append(element('p','is-error',error(e)));}
    });
    const mine=dialogEpoch();
    message=field('Sõnum omanikule');message.required=false;message.maxLength=500;
    dialogBody().append(element('p','muted','Omanik näeb sinu konto ID-d ja sõnumit.'));
  }
  function block(principal,blocked=true) {
    showDialog(blocked?'Blokeeri konto':'Eemalda blokeering',blocked?'Blokeeri':'Eemalda blokeering',()=>mutate(()=>call('block',{principal,blocked})));
    const id=element('code','account-id');id.textContent=principal;dialogBody().append(id,element('p','',blocked?'Blokeerimise ajal ei saa te üksteise jagatud faile avada ega uusi jagamisi saada.':'Blokeeringu eemaldamisel kehtivad varasemad jagamisõigused uuesti.'));
  }
  function hideItem(file) {
    showDialog('Eemalda enda vaatest','Peida',()=>mutate(()=>call('hideShared',{id:file.id,hidden:true}),{ids:[file.id]}));
    const name=element('p');name.textContent=file.name;dialogBody().append(name,element('p','','Algfail jääb alles ja sinu ligipääs säilib. Peidetud üksused leiad konto menüüst.'));
  }
  function report(file) {
    let reason;showDialog('Raporteeri jagamist','Saada raport',()=>mutate(()=>call('report',{id:file.id,reason:reason.value}),{ids:[file.id]}));
    reason=field('Põhjus');reason.maxLength=500;dialogBody().append(element('p','muted','Raport on nähtav teenuse haldajale. Faili sisu ei lisata raportisse.'));
  }
  return {show,requestAccess,block,hideItem,report,hide(){epoch++;view=null;host.hidden=true;body.replaceChildren();},refresh(){if(view)return show(view);}};
}
