// Team UI uses the same signed service and folder browser as personal files.
// Versions are returned by the service; a stale form never silently overwrites.
export const teamRoleLabel=role=>({Manager:'Haldaja',Editor:'Muutja',Viewer:'Vaataja'})[role]||role;
export function createTeamSpaces({host,body,call,element,button,text,bind,session,current,size,error,dialog,dialogBody,field,requestId,navigate,changed,dialogCurrent,mutate}) {
  let epoch=0;
  const fail=(target,e)=>target.append(element('p','is-error',error(e)));
  function selectRole(value) {
    const select=element('select');select.setAttribute('aria-label',text('Tiimi roll'));
    for(const role of ['Viewer','Editor','Manager']) { const option=element('option','',teamRoleLabel(role));option.value=role;select.append(option); }
    select.value=value;return select;
  }
  async function show() {
    const turn=++epoch,snapshot=session();host.hidden=false;body.replaceChildren();
    if(!current(snapshot)){body.append(element('p','','Logi sisse, et näha oma salvestatud faile.'));return;}
    body.append(button('Uus tiimiruum',create,'button button-primary'));
    const list=element('div','team-list');body.append(list,element('p','muted','Tiimi failid kuuluvad ruumile ja jäävad alles ka liikme lahkumisel.'));
    list.append(element('p','','Laadin…'));
    try {
      const spaces=await call('spaces');if(turn!==epoch||!current(snapshot))return;
      list.replaceChildren();
      for(const space of spaces) {
        const card=element('article','team-card'),name=element('h2');name.textContent=space.name;
        const usage=element('p','muted');bind(usage,()=>{usage.textContent=text('{role} · {used} kasutusel',{role:text(teamRoleLabel(space.role)),used:size(space.usedBytes)});});
        card.append(name,usage,button('Ava',()=>navigate(space.id)),button(space.role==='Manager'?'Halda tiimi':'Liikmed',()=>manage(space.id)));list.append(card);
      }
      if(!spaces.length)list.append(element('p','','Sa ei kuulu veel ühtegi tiimiruumi.'));
    }catch(e){if(turn===epoch&&current(snapshot)){list.replaceChildren();fail(list,e);}}
  }
  function create() {
    const snapshot=session(),token=requestId();let input,request;
    dialog('Uus tiimiruum','Loo tiimiruum',()=>mutate(async()=>{
      if(!current(snapshot))return;
      request??={name:input.value,requestId:token};input.disabled=true;
      const space=await call('spaceCreate',request);if(current(snapshot)){changed();navigate(space.id);}
    }));input=field('Tiimiruumi nimi');
  }
  async function manage(id) {
    const snapshot=session();dialog('Tiimiruumi haldamine',null);const active=dialogCurrent();const target=dialogBody();
    target.append(element('p','','Laadin…'));
    try {
      const view=await call('spaceGet',{id});if(!active()||!current(snapshot))return;
      target.replaceChildren();const title=element('h3');title.textContent=view.name;target.append(title);
      const usage=element('p','muted');bind(usage,()=>{usage.textContent=text('{used} kasutusel',{used:size(view.usedBytes)});});target.append(usage);
      if(view.role==='Manager')target.append(button('Ruumi seaded',()=>settings(view)),button('Lisa liige',()=>add(view)));
      target.append(button('Lahku tiimist',()=>remove(view,{principal:snapshot.principal})));
      const list=element('div','team-members');target.append(list);let next=null;
      do {
        const page=await call('spaceMembers',{id,after:next});if(!active()||!current(snapshot))return;
        // Keep one authority version across all loaded member pages.
        if(page.space.version!==view.version)throw new Error(text('Tiim on vahepeal muutunud. Ava haldusvaade uuesti.'));
        for(const member of page.members) {
          const row=element('div','share-row'),who=element('code','account-id');who.textContent=member.principal;row.append(who,element('span','',teamRoleLabel(member.role)));
          if(view.role==='Manager')row.append(button('Muuda rolli',()=>editMember(view,member)),button('Eemalda',()=>remove(view,member)));
          list.append(row);
        }
        next=page.next;
      } while(next);
    }catch(e){if(active()&&current(snapshot)){target.replaceChildren();fail(target,e);}}
  }
  function update(title,label,view,build) {
    const snapshot=session(),token=requestId();let request,read;
    dialog(title,label,()=>mutate(async()=>{
      if(!current(snapshot))return;
      request??={id:view.id,version:view.version,requestId:token,...read()};
      await call(request.operation,{...request,operation:undefined});
      if(current(snapshot)){changed();if(!host.hidden)await show();}
    }));read=build();
  }
  function add(view) {
    update('Lisa liige','Lisa',view,()=>{
      const principal=field('Konto ID'),role=selectRole('Viewer');dialogBody().append(role,element('p','muted','Lisa olemasoleva Subneti konto ID.'));
      return ()=>({operation:'spaceMember',principal:principal.value.trim(),role:role.value});
    });
  }
  function editMember(view,member) {
    update('Muuda rolli','Salvesta',view,()=>{
      const who=element('code','account-id');who.textContent=member.principal;
      const role=selectRole(member.role);dialogBody().append(who,role);
      return ()=>({operation:'spaceMember',principal:member.principal,role:role.value});
    });
  }
  function remove(view,member) {
    update('Eemalda tiimist','Eemalda',view,()=>{
      const who=element('code','account-id');who.textContent=member.principal;
      dialogBody().append(who,element('p','','Liikme tiimipõhine ligipääs lõpeb. Tiimi failid jäävad alles.'));
      return ()=>({operation:'spaceMember',principal:member.principal,role:null});
    });
  }
  function settings(view) {
    update('Ruumi seaded','Salvesta',view,()=>{
      const name=field('Tiimiruumi nimi',view.name),label=element('label','dialog-field'),external=element('input');external.type='checkbox';external.checked=view.externalSharing;
      label.append(external,element('span','','Luba jagamine väljaspool tiimi'));dialogBody().append(label,element('p','muted','Välise jagamise keelamine peatab väliste saajate ja avalike linkide ligipääsu.'));
      return ()=>({operation:'spaceSettings',name:name.value,externalSharing:external.checked});
    });
  }
  return {show,manage,hide(){++epoch;host.hidden=true;body.replaceChildren();}};
}
