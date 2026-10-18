// Optional account contacts. The backend alone verifies codes, binds addresses and grants access.
export function createAccountEmail(ui) {
  const {call, element, button, field, dialog, body, error, current} = ui;
  let revision=0;
  const pendingLabel=status=>({Accepted:'Kinnituskiri on saatmiseks vastu võetud.',Sending:'Kirja saatmine on pooleli.',Queued:'Kiri ootab saatmist.',Retryable:'Kirja saatmine vajab korduskatset.',Unknown:'Saatmise tulemus ei ole veel teada.',Failed:'Kirja saatmine ebaõnnestus.',Expired:'Kinnituskood on aegunud.'})[status] || 'Kirja saatmine on pooleli.';
  function page(title, submit, action) {
    const mine=++revision;dialog(title,submit,action);
    const active=current();
    return ()=>mine===revision && active();
  }
  async function action(alive, work, done) {
    if(!alive())return;
    const controls=body().querySelectorAll('input,button,select');
    for(const node of controls)node.disabled=true;
    try {const result=await work();if(alive())await done(result);}
    catch(e) {if(alive())error(e);}
    finally {if(alive())for(const node of controls)node.disabled=false;}
  }
  function transfer(pending) {
    const alive=page('Too e-post sellele kontole','Too e-post üle',()=>action(alive,()=>call('emailConfirmTransfer',{id:pending.id}),()=>open()));
    const address=element('strong');address.textContent=pending.email;body().append(address);
    body().append(element('p','','See aadress on seotud teise Subneti kontoga. Ületoomine suunab tulevased e-postiga jagamised sellele kontole.'));
    body().append(element('p','','Vana konto failid ja juba antud õigused jäävad vanale kontole. Ootel kutseid ei kanta üle. Internet Identity kontot ei muudeta.'));
  }
  function code(pending) {
    if(!pending){open();return;}
    if(pending.transferRequired){transfer(pending);return;}
    let input;
    const alive=page('Kinnita e-post','Kinnita kood',()=>action(alive,()=>call('emailVerify',{id:pending.id,code:input.value.trim()}),result=>result==='TransferRequired'?transfer(pending):open()));
    const address=element('p');address.textContent=pending.email;body().append(address,element('p','muted',pendingLabel(pending.delivery)));
    input=field('8-kohaline kinnituskood');input.maxLength=8;input.inputMode='numeric';input.autocomplete='one-time-code';input.pattern='[0-9]{8}';
    body().append(element('p','muted','Sisesta 10 minuti jooksul kood samal Subneti kontol. Koodi saab proovida kuni viis korda.'));
    if(['Unknown','Retryable','Sending','Queued'].includes(pending.delivery))body().append(button('Proovi saatmist uuesti',()=>action(alive,()=>call('emailRetry',{id:pending.id}),()=>open())));
    body().append(button('Saada uus kood',()=>add(pending.email)),button('Tagasi e-postide juurde',()=>open()));input.focus();
  }
  function add(value='') {
    let input;let request=null;
    const alive=page('Lisa e-post','Saada kinnituskood',()=>action(alive,()=>{
      if(!request)request={email:input.value.trim(),requestId:crypto.randomUUID()};
      return call('emailRequest',request);
    },profile=>code(profile.pending)));
    input=field('E-posti aadress',value);input.type='email';input.maxLength=254;input.autocomplete='email';
    input.addEventListener('input',()=>{request=null;});
    body().append(element('p','muted','E-post seotakse sinu subnet kontoga.'));input.focus();
  }
  async function open() {
    const alive=page('Minu e-postid',null);body().append(element('p','','Laadin…'));
    await action(alive,()=>call('emailProfile'),profile=>{
      body().replaceChildren(element('p','muted','Ühel kontol võib olla mitu kinnitatud e-posti. Iga aadress saab kuuluda korraga ühele Subneti kontole.'));
      for(const email of profile.emails) {
        const row=element('div','share-row'),address=element('span');address.textContent=email;
        row.append(address,button('Eemalda e-post',()=>{
          const yes=page('Eemalda e-post','Eemalda e-post',()=>action(yes,()=>call('emailRemove',{email}),()=>open()));
          const name=element('strong');name.textContent=email;body().append(name,element('p','','See lõpetab tulevased jagamised selle aadressi kaudu. Failid ja juba antud õigused säilivad.'));
        }));body().append(row);
      }
      if(profile.pending)body().append(button('Jätka e-posti kinnitamist',()=>code(profile.pending)));
      if(profile.enabled)body().append(button('Lisa e-post',()=>add()));
      else body().append(element('p','','E-posti saatmine ei ole veel seadistatud.'));
      if(ui.invitationId())body().append(button('Tagasi kutse juurde',()=>invite(ui.invitationId())));
    });
  }
  function invite(id) {
    const alive=page('Jagamiskutse','Võta kutse vastu',()=>action(alive,()=>call('emailAccept',{id}),result=>ui.accepted(result.id)));
    body().append(element('p','','Kutse vastuvõtmiseks kinnita sellel Subneti kontol aadress, millele kutse saadeti. Link ise ligipääsu ei anna.'));
    body().append(button('Minu e-postid',()=>open()));
  }
  return {open,invite,pendingLabel};
}
