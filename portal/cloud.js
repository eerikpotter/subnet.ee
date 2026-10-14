import init, { CloudPortal } from "__CLOUD_BINDINGS__";
const { t, text, bind, href: localizedHref } = window.SubnetI18n;
const $ = id => document.getElementById(id);
let cloud, account, root, folder, cursor, registry = [];
let generation = 0, loading = false, restoring = false, busy = false, lastRefresh = 0;
let entries = [], trail = [], currentView = 'files', modalAction, replaceFile, loginAttempt = 0, dialogGeneration = 0;
let previewState = null, previewQueue = Promise.resolve();
let fileMenus = [];
let picker = null, pickerSelection = null;
function closeFileMenus(except=null) { for(const menu of fileMenus) if(menu!==except)menu.open=false; }
const call = (operation, payload = {}) => cloud.drive(operation, JSON.stringify(payload));
const requestId = () => crypto.randomUUID();
// Metadata only, memory only, scoped to this signed-in session. A cached view
// never grants authority: mutations remain disabled until revalidation finishes.
const views = new Map(), catalog = new Map(), completeFolders = new Map();
function clearCache() { views.clear(); catalog.clear(); completeFolders.clear(); }
function dropCachedTree(id) {
  const pending=[id], seen=new Set();
  while(pending.length) {
    const current=pending.pop();if(seen.has(current))continue;seen.add(current);
    for(const [child,{entry}] of catalog) if(entry.parent===current)pending.push(child);
    catalog.delete(current);completeFolders.delete(current);views.delete(`folder/${current}`);
  }
}
function rememberCatalog(snapshot) {
  if(!snapshot) return;
  const fresh=new Set(snapshot.entries.map(entry=>entry.id)), complete=new Set(snapshot.completeFolders), at=Date.now();
  for(const [id,{entry}] of catalog) if(complete.has(entry.parent) && !fresh.has(id)) dropCachedTree(id);
  for(const entry of snapshot.entries) {catalog.delete(entry.id);catalog.set(entry.id,{entry,at});}
  for(const id of complete) completeFolders.set(id,at);
  while(catalog.size>400) {const id=catalog.keys().next().value;catalog.delete(id);completeFolders.delete(id);}
}
function prefetchedPage(parent) {
  const found=catalog.get(parent);
  if(!found || !found.entry.folder || Date.now()-found.at>=30000) return null;
  const children=[...catalog.values()].filter(v=>v.entry.parent===parent && Date.now()-v.at<30000).map(v=>v.entry);
  if(!children.length && Date.now()-(completeFolders.get(parent)||0)>=30000) return null;
  const breadcrumbs=[];let entry=found.entry;
  for(let depth=0;entry && depth<64;depth++) {breadcrumbs.unshift(entry);entry=catalog.get(entry.parent)?.entry;}
  return {folder:found.entry,breadcrumbs,entries:children.sort((a,b)=>BigInt(a.id)<BigInt(b.id)?-1:1).slice(0,50),cursor:null};
}
function remember(key, value) {
  views.delete(key); views.set(key, {value, at:Date.now()});
  if(views.size > 12) views.delete(views.keys().next().value);
}
function transfer(label, values, fraction = null) {
  $('transfer-status').hidden = false;
  text($('transfer-label'), label, values);
  if(fraction === null) $('transfer-progress').removeAttribute('value');
  else $('transfer-progress').value = fraction;
}
function element(tag, className, source) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (source) text(node, source);
  return node;
}
function button(source, action, className = 'button') {
  const node = element('button', className, source); node.type = 'button'; node.addEventListener('click', action); return node;
}
function message(source = '', error = false) {
  text($('account-message'), source); $('account-message').hidden = !source; $('account-message').classList.toggle('is-error', error);
}
function errorMessage(error) {
  const source = error?.message || String(error);
  return ({'Not found or access unavailable':'Fail või kaust ei ole kättesaadav.',
    'This item changed; refresh and try again':'Fail või kaust on vahepeal muutunud. Värskenda ja proovi uuesti.',
    'An item with this name already exists':'Selle nimega fail või kaust on juba olemas.',
    'Cloud storage quota reached':'Pilveruumi maht on täis.',
    'Revision limit reached':'Faili versioonide piir on täis.',
    'This operation expired; start again':'Üleslaadimine aegus. Laadi fail uuesti üles.'})[source] || source;
}
function size(bytes) {
  if (bytes < 1024) return new Intl.NumberFormat(window.SubnetI18n.locale).format(bytes) + ' B';
  const unit = bytes < 1024 * 1024 ? 'KB' : 'MB';
  return new Intl.NumberFormat(window.SubnetI18n.locale, { maximumFractionDigits: 1 }).format(bytes / (unit === 'KB' ? 1024 : 1024 * 1024)) + ' ' + unit;
}
function updateAccount() {
  const signed = !!account?.authenticated;
  $('sign-in').hidden = signed; $('sign-out').hidden = !signed;
  $('sign-in').disabled = !cloud || restoring; $('sign-out').disabled = restoring || busy;
  $('refresh-files').disabled = !signed || loading || restoring || busy;
  $('new-folder').disabled = $('upload-file').disabled = !signed || !folder?.capabilities.create || loading || busy || currentView === 'trash';
  $('new-folder').hidden = $('upload-file').hidden = ['shared', 'trash'].includes(currentView) && !folder;
  $('copy-account').hidden = !signed;
  text($('account-status'), signed ? 'Minu konto' : 'Sinu pilveruum');
  $('account-status').title = signed ? account.principal : '';
  updatePicker();
  bind($('storage-usage'), () => { $('storage-usage').textContent = signed ? t('{used} / {quota} kasutusel', { used: size(account.usedBytes), quota: size(account.quotaBytes) }) : t('Sinu failid on seotud sinu kontoga.'); });
}
function heading() {
  const label = currentView === 'apps' ? 'Rakendused' : currentView === 'trash' ? 'Prügikast' : currentView === 'shared' ? 'Minuga jagatud' : 'Minu failid';
  text($('page-name'), label);
  if (folder && folder.id !== root?.id) { $('files-title').textContent = folder.name; $('files-title').removeAttribute('data-i18n-dynamic'); }
  else text($('files-title'), label);
  text($('drive-description'), currentView === 'trash' ? 'Kustutatud failid ja kaustad. Taasta need algsesse asukohta.' : currentView === 'shared' ? 'Failid ja kaustad, mille teised on sinuga jaganud.' : 'Sinu failid. Sinu inimesed. Sinu AI.');
  $('apps-view').hidden = currentView !== 'apps'; $('failid').hidden = currentView === 'apps';
  for (const [id, view] of [['files-nav','files'],['shared-nav','shared'],['trash-nav','trash'],['apps-nav','apps']]) {
    $(id).classList.toggle('nav-active', currentView === view);
    if (currentView === view) $(id).setAttribute('aria-current','page'); else $(id).removeAttribute('aria-current');
  }
  $('breadcrumbs').replaceChildren();
  if (folder) {
    const start = element('a', '', currentView === 'shared' ? 'Minuga jagatud' : 'Minu failid'); start.href = currentView === 'shared' ? '#jagatud' : '#failid'; $('breadcrumbs').append(start);
    for (const item of trail.filter(item => item.id !== root?.id)) {
      const separator = element('span'); separator.textContent = '/';
      const link = element('a'); link.textContent = item.name; link.href = `#folder/${item.id}`;
      if (item.id === folder.id) link.setAttribute('aria-current','page');
      $('breadcrumbs').append(separator, link);
    }
  }
}
function clearFiles(source) {
  entries = []; cursor = null; $('file-list').replaceChildren(); $('more-files').hidden = true;
  text($('file-status'), source); $('file-status').hidden = false;
}
function appFor(file) { return registry.find(app => app.cloud && app.openFormats?.some(format => format.id === file.format?.craft?.format_id && format.version === file.format?.craft?.format_version)); }
function editorHref(file) {
  const app = appFor(file);
  return app && file.capabilities.read && /^[a-z]+$/.test(app.id)
    ? localizedHref(`/apps/${app.id}/?${new URLSearchParams({cloud_service:'cloud',cloud_space:file.spaceId,cloud_file:file.id})}`) : null;
}
function setupPicker() {
  const params=new URLSearchParams(location.search), mode=params.get('picker');
  if(!mode) return;
  const app=registry.find(app=>app.id===params.get('app') && app.cloud), channel=params.get('channel');
  if(!['open','save'].includes(mode) || !app || !/^[a-f0-9-]{36}$/.test(channel||'') || window.parent===window) throw new Error(t('Failivalijat ei saa siin avada.'));
  picker={mode,app,channel,done:false};
  document.body.classList.add('cloud-picker');
  if(params.get('theme')==='dark') document.body.classList.add('picker-dark');
  $('picker-footer').hidden=false;$('picker-name-field').hidden=mode!=='save';
  $('picker-name').value=params.get('name')||'';
  $('picker-name').addEventListener('input',updatePicker);
  $('picker-form').addEventListener('submit',event=>{event.preventDefault();if(mode==='save')selectPickerFolder();else if(pickerSelection)selectPickerFile(pickerSelection);});
  $('picker-cancel').addEventListener('click',()=>postSelection({action:'cancel'}));
  bind($('picker-heading'),()=>{$('picker-heading').textContent=t(mode==='save'?'Salvesta pilveruumi':'Ava pilveruumist')+' · '+app.name;});
  updatePicker();
}
function pickerSupports(file) {return !!picker && appFor(file)?.id===picker.app.id && file.capabilities.read && !!file.revision;}
function updatePicker() {
  if(!picker)return;
  const ready=account?.authenticated && !loading && !restoring && !busy && !picker.done && !cloud?.needs_refresh();
  const canCreate=!!folder?.capabilities.create;
  $('picker-submit').disabled=!ready || (picker.mode==='save' ? !canCreate || !$('picker-name').value.trim() : !pickerSelection);
  text($('picker-submit'),picker.mode==='save'?'Salvesta siia':'Ava');
  text($('picker-hint'),picker.mode==='save' ? (folder && !canCreate ? 'Selles kaustas puudub loomise õigus.' : 'Vali kaust ja faili nimi. Olemasolevaid faile ei kirjutata üle.') : 'Vali fail avamiseks. Ainult loetavast failist saad salvestada oma koopia.');
}
function postSelection(value) {
  if(!picker || picker.done)return;
  picker.done=true;updatePicker();
  window.parent.postMessage({type:'subnet-cloud-selection',channel:picker.channel,principal:account?.principal||'',...value},location.origin);
}
async function selectPickerFile(file, revision=null) {
  if(!picker || picker.mode!=='open' || !pickerSupports(file) || loading || busy || restoring || cloud.needs_refresh())return;
  const epoch=generation;busy=true;updateAccount();
  try {
    const fresh=await call('get',{id:file.id});
    if(epoch!==generation || cloud.needs_refresh() || !pickerSupports(fresh))return;
    postSelection({action:'open',space:fresh.spaceId,file:fresh.id,revision:revision||fresh.revision});
  } catch(error){message(errorMessage(error),true);}finally{busy=false;updateAccount();}
}
async function selectPickerFolder() {
  if(!picker || picker.mode!=='save' || !folder?.capabilities.create || loading || busy || restoring || cloud.needs_refresh())return;
  const suffix='.'+picker.app.cloudExtension, raw=$('picker-name').value.trim();
  const name=raw.toLowerCase().endsWith(suffix)?raw:raw+suffix;
  if(!raw || new TextEncoder().encode(name).length>160 || /[\/\\]/.test(name)) {message('Sisesta sobiv faili nimi (kuni 160 baiti).',true);return;}
  const epoch=generation,parent=folder.id;busy=true;updateAccount();
  try {
    const fresh=await call('get',{id:parent});
    if(epoch!==generation || cloud.needs_refresh())return;
    if(!fresh.folder || !fresh.capabilities.create)throw new Error(t('Selles kaustas puudub loomise õigus.'));
    postSelection({action:'save',parent,name});
  } catch(error){message(errorMessage(error),true);}finally{busy=false;updateAccount();}
}
function sharingBadge(file) {
  if(!file.sharing?.direct && !file.sharing?.inherited) return null;
  const badge=element('span','sharing-badge');badge.setAttribute('role','img');
  bind(badge,()=>{
    const label=t(file.sharing.direct && file.sharing.inherited ? 'Jagatud otse ja ülemkausta kaudu' : file.sharing.direct ? 'Jagatud' : 'Jagatud ülemkausta kaudu');
    badge.title=label;badge.setAttribute('aria-label',label);
  });
  return badge;
}
function renderFiles() {
  fileMenus=[];
  $('file-list').replaceChildren();
  const sorted = [...entries].sort((a,b) => Number(b.folder)-Number(a.folder) || a.name.localeCompare(b.name, window.SubnetI18n.locale));
  for (const file of sorted) {
    const row = element('article', 'file-row'); row.dataset.fileId = file.id;
    const icon = element('span', `file-icon${file.folder ? ' folder-icon' : ''}`); icon.textContent = file.folder ? '▱' : (appFor(file)?.symbol || '↥'); icon.setAttribute('aria-hidden','true');
    const detail = element('div','file-detail'); const title = element('h2');
    if (file.folder && currentView !== 'trash') { const link = element('a','folder-link'); link.textContent = file.name; link.href = `#folder/${file.id}`; title.append(link); }
    else title.textContent = file.name;
    const meta = element('p'); bind(meta, () => { meta.textContent = `${file.folder ? t('Kaust') : size(file.bytes || 0)} · ${new Date(file.updatedMs).toLocaleDateString(window.SubnetI18n.locale, {dateStyle:'medium'})}`; });
    const name=element('div','file-name-line');name.append(title);
    const badge=currentView==='trash' ? null : sharingBadge(file);if(badge)name.append(badge);
    detail.append(name, meta); row.append(icon, detail);
    if(picker && !file.folder) {
      const supported=pickerSupports(file);
      row.classList.toggle('picker-unsupported',!supported);
      if(supported && picker.mode==='open') {
        row.tabIndex=0;row.setAttribute('aria-label',file.name);
        row.addEventListener('click',()=>{pickerSelection=file;for(const item of $('file-list').children)item.classList.toggle('picker-selected',item.dataset.fileId===file.id);updatePicker();});
        row.addEventListener('dblclick',()=>selectPickerFile(file));
        row.addEventListener('keydown',event=>{if(event.key==='Enter')selectPickerFile(file);});
        row.classList.toggle('picker-selected',pickerSelection?.id===file.id);
        row.append(button('Ava',()=>selectPickerFile(file)));
        if(file.capabilities.edit)row.append(button('Versioonid',()=>history(file)));
      }
      if(!file.capabilities.edit) detail.append(element('p','picker-access','Ainult lugemiseks'));
      $('file-list').append(row);continue;
    }
    if (currentView === 'trash') row.append(button('Taasta', () => organize(file, 'restore')));
    else {
      const href = picker ? null : editorHref(file);
      if (href) {
        const link = element('a','button','Ava'); link.href = href; row.append(link);
      } else if(!file.folder) row.append(button('Ava',()=>openFile(file)));
      row.tabIndex=0;
      row.addEventListener('dblclick',event=>{if(!event.target.closest('a, button, details'))openFile(file);});
      row.addEventListener('keydown',event=>{
        if(event.target===row && event.key==='Enter') {event.preventDefault();openFile(file);}
      });
      if(picker) {$('file-list').append(row);continue;}
      const menu = element('details','file-menu'); const summary = element('summary','button'); summary.textContent = '···'; bind(summary, () => summary.setAttribute('aria-label', t('Failitoimingud'))); menu.append(summary);
      fileMenus.push(menu);summary.addEventListener('click',()=>closeFileMenus(menu));
      const actions = element('div','file-menu-items');
      const action = (label, fn) => actions.append(button(label, () => { if(loading || busy) return; menu.open = false; fn(); }));
      if (!file.folder) { actions.append(button('Ava',()=>{menu.open=false;openFile(file);})); action('Laadi alla', () => download(file)); if(file.capabilities.edit) action('Versioonid', () => history(file)); }
      if (file.capabilities.edit) { action('Nimeta ümber', () => rename(file)); if (!file.folder) action('Laadi uus versioon', () => { replaceFile = file; $('replace-input').click(); }); }
      if (file.capabilities.edit && file.capabilities.delete) action('Liiguta', () => move(file));
      if (file.owner === account.principal) action('Jaga', () => share(file));
      if (file.capabilities.delete) action('Prügikasti', () => trash(file));
      if (actions.children.length) { menu.append(actions); row.append(menu); }
    }
    $('file-list').append(row);
  }
  $('more-files').hidden = !cursor; $('file-status').hidden = entries.length > 0;
  text($('file-status'), currentView === 'trash' ? 'Prügikast on tühi.' : currentView === 'shared' && !folder ? 'Sinuga pole veel midagi jagatud. Anna jagajale oma konto ID.' : 'Siin on veel tühi. Loo kaust või laadi esimene fail üles.');
}
async function route(initialPage = null) {
  const epoch = ++generation;
  pickerSelection=null;
  text($('view-status'),'');
  const hash = picker && ['#rakendused','#prugikast'].includes(location.hash) ? '#failid' : location.hash;
  currentView = hash === '#rakendused' ? 'apps' : hash === '#jagatud' ? 'shared' : hash === '#prugikast' ? 'trash' : 'files';
  const parent = hash.startsWith('#folder/') ? hash.slice(8) : currentView === 'files' ? root?.id : null;
  const key = parent ? `folder/${parent}` : currentView;
  folder = null; trail = [];
  if (!account?.authenticated || !cloud) { loading=false; clearFiles('Logi sisse, et näha oma salvestatud faile.'); heading(); updateAccount(); return; }
  if (currentView === 'apps') { loading=false; heading(); updateAccount(); return; }
  if(initialPage?.folder?.id === parent && !cloud.needs_refresh()) {
    applyPage(initialPage,false); remember(key,initialPage); loading=false; updateAccount(); return;
  }
  const prefetched = parent ? prefetchedPage(parent) : null;
  const cached = prefetched ? {value:prefetched,at:Date.now()} : views.get(key);
  if(cached && Date.now()-cached.at < 30000 && !cloud.needs_refresh()) applyPage(cached.value, false);
  else clearFiles('Laadin sinu faile…');
  loading = true; heading(); updateAccount(); text($('view-status'),'Värskendan…');
  try {
    const page = await call(currentView === 'trash' ? 'trash' : parent ? 'browse' : 'list', {parent,cursor:null});
    if(epoch !== generation || cloud.needs_refresh()) return;
    applyPage(page, false); remember(key,page); lastRefresh=Date.now();
  } catch(error) {
    if(epoch === generation) { clearCache(); folder=null; trail=[]; clearFiles('Kaust ei ole kättesaadav. Värskenda või ava Minu failid.'); message(errorMessage(error),true); heading(); }
  } finally { if(epoch === generation) { loading=false; text($('view-status'),''); updateAccount(); } }
}
function applyPage(page, append) {
  rememberCatalog(page.catalog);
  if(page.folder) { folder=page.folder; trail=page.breadcrumbs; currentView=folder.owner===account.principal ? 'files' : 'shared'; }
  const visible = currentView==='shared' && !folder ? page.entries.filter(e=>e.owner!==account.principal) : page.entries;
  entries=append ? [...entries,...visible] : visible; cursor=page.cursor; heading(); renderFiles();
}
async function loadFiles(epoch, append = false) {
  if (!account?.authenticated) return;
  loading = true; updateAccount(); $('more-files').disabled = true;
  try {
    const page = await call(currentView === 'trash' ? 'trash' : folder ? 'browse' : 'list', {parent:folder?.id || null,cursor:append ? cursor : null});
    if (epoch !== generation || cloud.needs_refresh()) return;
    applyPage(page,append);
  } catch(error) { if(epoch === generation) { clearCache(); message(errorMessage(error),true); } }
  finally { if(epoch === generation) { loading=false; $('more-files').disabled=false; updateAccount(); } }
}
async function refresh(full = false) {
  if(cloud?.needs_refresh()) closePreview();
  if($('preview-dialog').open) return;
  if (!cloud || restoring || busy || $('drive-dialog').open) return;
  if(!full && account?.authenticated && !cloud.needs_refresh()) { await route(); return; }
  restoring=true; clearCache(); ++generation; clearFiles('Kontrollin sisselogimist…'); updateAccount();
  try {
    const state=await cloud.restore(); account=state; root=null; folder=null;
    if(state.authenticated) {
      root=state.home?.folder || await call('initialize');
      if(state.home) {remember(`folder/${root.id}`,state.home);rememberCatalog(state.home.catalog);}
    }
    message(); lastRefresh=Date.now(); await route(state.home);
  } catch(error) { account=null; clearCache(); clearFiles('Kontoga ühendamine ei õnnestunud. Proovi uuesti.'); message(errorMessage(error),true); }
  finally { restoring=false; updateAccount(); }
}
function showDialog(title, submit, action) {
  ++dialogGeneration;
  text($('dialog-title'),title); $('dialog-body').replaceChildren(); $('dialog-error').hidden = true;
  $('dialog-submit').hidden = !submit; $('dialog-submit').disabled = false;
  if (submit) text($('dialog-submit'),submit);
  modalAction = action; $('drive-dialog').showModal();
}
function field(label, value = '') {
  const wrapper = element('label','dialog-field',label); const input = element('input'); input.value = value; input.required = true; input.maxLength = 160; wrapper.append(input); $('dialog-body').append(wrapper); return input;
}
function closeDialog() { if (!busy) { ++dialogGeneration; $('drive-dialog').close(); } }
async function mutate(action, success) {
  if (busy || loading) return; let completed = false; busy = true; updateAccount(); const epoch = generation;
  try { await action(); completed = true; if(epoch !== generation || cloud.needs_refresh()) return; $('drive-dialog').close(); if(success) message(success); }
  catch(error) { if ($('drive-dialog').open) { text($('dialog-error'),errorMessage(error)); $('dialog-error').hidden = false; } else message(errorMessage(error),true); }
  finally { busy = false; updateAccount(); }
  if (!$('drive-dialog').open && epoch === generation) { if (completed) { clearCache(); await refresh(); } else await route(); }
}
function newFolder() {
  const parent = folder?.id; if(!parent) return; const token = requestId(); let input;
  showDialog('Uus kaust','Loo kaust',() => mutate(() => call('folder',{parent,name:input.value,requestId:token})));
  input = field('Kausta nimi'); input.focus();
}
function rename(file) {
  const token = requestId(); let input;
  showDialog('Nimeta ümber','Salvesta',() => mutate(() => call('organize',{id:file.id,version:file.version,change:'rename',name:input.value,requestId:token})));
  input = field('Nimi',file.name); input.focus(); input.select();
}
function organize(file, change, extra = {}) { return mutate(() => call('organize',{id:file.id,version:file.version,change,requestId:requestId(),...extra})); }
function trash(file) {
  const token = requestId(); showDialog('Prügikasti','Prügikasti',() => mutate(() => call('organize',{id:file.id,version:file.version,change:'trash',requestId:token})));
  const p = element('p'); p.textContent = file.name; $('dialog-body').append(p, element('p','muted','Faili või kausta saab hiljem prügikastist taastada.'));
}
async function download(file, revision = file.revision) {
  if(busy) return; busy = true; updateAccount(); const epoch = generation;
  try {
    message('Laadin faili alla…'); const bytes = await cloud.download(file.id, revision);
    if(epoch !== generation || cloud.needs_refresh()) return;
    const url = URL.createObjectURL(new Blob([bytes],{type:'application/octet-stream'}));
    const link = element('a'); link.href = url; link.download = file.name; document.body.append(link); link.click(); link.remove(); setTimeout(() => URL.revokeObjectURL(url),30000); message();
  } catch(error) { message(errorMessage(error),true); } finally { busy = false; updateAccount(); }
}
// Rendering is deliberately restricted to inert image, media and plain-text
// contexts. Never put uploaded HTML/SVG markup into this document or an iframe.
function previewFormat(file) {
  if(file.format?.craft) return null;
  const types={png:'image/png',jpg:'image/jpeg',jpeg:'image/jpeg',gif:'image/gif',webp:'image/webp',avif:'image/avif',bmp:'image/bmp',svg:'image/svg+xml',
    mp3:'audio/mpeg',wav:'audio/wav',ogg:'audio/ogg',oga:'audio/ogg',m4a:'audio/mp4',flac:'audio/flac',aac:'audio/aac',opus:'audio/ogg',
    mp4:'video/mp4',m4v:'video/mp4',webm:'video/webm',ogv:'video/ogg',mov:'video/quicktime'};
  const supplied=(file.format?.media_type||'').split(';')[0].trim().toLowerCase();
  const extension=file.name.split('.').pop().toLowerCase();
  const mime=Object.values(types).includes(supplied) ? supplied : types[extension];
  if(mime) return {kind:mime.split('/')[0],mime};
  if(supplied.startsWith('text/') || /^application\/(json|xml|javascript|yaml|toml)$/.test(supplied)
    || ['txt','md','csv','tsv','json','xml','html','htm','css','js','ts','yaml','yml','toml','log','rs','py','sh'].includes(extension)) return {kind:'text',mime:'text/plain'};
  return null;
}
function releasePreview() {
  const state=previewState;previewState=null;
  if(state?.media) {state.media.pause();state.media.removeAttribute('src');state.media.load();}
  if(state?.url) URL.revokeObjectURL(state.url);
  $('preview-content').replaceChildren();$('preview-content').setAttribute('aria-busy','false');
  $('preview-name').textContent='';$('preview-meta').textContent='';
}
function closePreview() { releasePreview();if($('preview-dialog').open)$('preview-dialog').close(); }
function previewNotice(source, error=false) {
  const notice=element('p',`preview-notice${error ? ' is-error' : ''}`,source);
  notice.setAttribute('role',error?'alert':'status');$('preview-content').replaceChildren(notice);
  $('preview-content').setAttribute('aria-busy','false');
}
function openFile(file) {
  if(busy || !account?.authenticated || cloud.needs_refresh())return;
  closeFileMenus();
  if(picker && !file.folder) {selectPickerFile(file);return;}
  if(file.folder) {location.hash=`#folder/${file.id}`;return;}
  const href=editorHref(file);if(href) {location.href=href;return;}
  closePreview();
  const state={file,principal:account.principal};previewState=state;
  const current=()=>previewState===state && account?.principal===state.principal && !cloud.needs_refresh();
  $('preview-name').textContent=file.name;$('preview-meta').textContent=size(file.bytes||0);
  $('preview-dialog').showModal();
  const format=previewFormat(file);
  if(!format) {previewNotice('Selle failitüübi eelvaade pole veel saadaval. Faili saad alla laadida.');return;}
  if(format.kind==='text' && file.bytes>1024*1024) {previewNotice('Teksti eelvaade on saadaval kuni 1 MB failidele. Suurema faili saad alla laadida.');return;}
  previewNotice('Laadin eelvaadet…');$('preview-content').setAttribute('aria-busy','true');
  // At most one verified download in flight. Closing discards the result; queued
  // stale requests never start, even when users open/close files rapidly.
  previewQueue=previewQueue.then(async()=>{
    if(!current()) return;
    try {
      const bytes=await cloud.download(file.id,file.revision);
      if(!current())return;
      if(format.kind==='text') {
        if(bytes.length>1024*1024)throw new Error(t('Teksti eelvaade on saadaval kuni 1 MB failidele. Suurema faili saad alla laadida.'));
        const encoding=bytes[0]===255 && bytes[1]===254 ? 'utf-16le' : bytes[0]===254 && bytes[1]===255 ? 'utf-16be' : 'utf-8';
        let value;try {value=new TextDecoder(encoding,{fatal:true}).decode(bytes);} catch {throw new Error(t('Selle teksti kodeeringut ei saa eelvaates lugeda. Laadi fail alla.'));}
        const pre=element('pre','preview-text');pre.textContent=value || t('Fail on tühi.');pre.tabIndex=0;
        $('preview-content').replaceChildren(pre);$('preview-content').setAttribute('aria-busy','false');return;
      }
      state.url=URL.createObjectURL(new Blob([bytes],{type:format.mime}));
      const viewer=element(format.kind==='image'?'img':format.kind,`preview-${format.kind}`);
      const ready=()=>{if(current()){$('preview-content').setAttribute('aria-busy','false');$('preview-content').querySelector('.preview-notice')?.remove();}};
      viewer.addEventListener('error',()=>{if(current())previewNotice('Brauser ei saa seda faili kuvada või esitada. Laadi fail alla.',true);});
      if(format.kind==='image') {viewer.alt=file.name;viewer.addEventListener('load',ready);}
      else {state.media=viewer;viewer.controls=true;viewer.preload='metadata';viewer.playsInline=true;viewer.setAttribute('aria-label',file.name);viewer.addEventListener('loadedmetadata',ready);}
      viewer.src=state.url;$('preview-content').append(viewer);
    } catch(error) {if(current())previewNotice(errorMessage(error),true);}
  });
}
async function uploadFiles(files, replacement) {
  if(busy || !files.length) return;
  let outcome='', failed=false; busy=true; message(); updateAccount();
  const principal=account?.principal, parent=folder?.id;
  const current=()=>account?.principal===principal && !cloud.needs_refresh();
  try {
    for(const [index,file] of files.entries()) {
      if(file.size > 64*1024*1024) throw new Error(t('Fail võib olla kuni 64 MB.'));
      transfer('Valmistan ette: {name}',{name:file.name});
      text($('transfer-detail'),'{current} / {count} faili · {size}',{current:index+1,count:files.length,size:size(file.size)});
      const app=registry.find(app=>app.cloudExtension && file.name.toLowerCase().endsWith('.'+app.cloudExtension));
      const format=app?.openFormats?.[0];
      const payload={parent,name:file.name,mediaType:file.type || 'application/octet-stream',craft:format ? {format_id:format.id,format_version:format.version,created_by_app:app.id} : null,requestId:requestId(),...(replacement ? {file:replacement.id,revision:replacement.revision} : {})};
      const bytes=new Uint8Array(await file.arrayBuffer());
      if(!current()) throw new Error(t('Konto muutus. Logi uuesti sisse.'));
      const send=()=>cloud.upload(JSON.stringify(payload),bytes,(done,total)=>{
        if(!current()) return;
        if(done===total) transfer('Salvestan: {name}',{name:file.name});
        else transfer('Laadin üles: {name} · {percent}%',{name:file.name,percent:Math.round(done/total*100)},done/total);
      });
      try { await send(); } catch(error) {
        if(!current()) throw error;
        transfer('Ühendan uuesti: {name}',{name:file.name});
        await send(); // Same request ID and bytes: no duplicate publication.
      }
      if(!current()) throw new Error(t('Konto muutus. Logi uuesti sisse.'));
    }
    outcome='Üles laaditud.';
  } catch(error) { outcome=errorMessage(error); failed=true; }
  finally {
    busy=false; clearCache(); updateAccount();
    transfer('Värskendan faililoendit…',{});
    await refresh(true);
    $('transfer-status').hidden=true; message(outcome,failed);
  }
}
async function history(file) {
  showDialog('Versioonid',null); const dialogEpoch = dialogGeneration; const body = $('dialog-body'); body.append(element('p','','Laadin…'));
  try {
    const revisions = await call('history',{id:file.id}); if(!$('drive-dialog').open || dialogEpoch !== dialogGeneration || cloud.needs_refresh()) return; body.replaceChildren();
    for(const revision of revisions.reverse()) {
      const row = element('div','history-row'); const info = element('div');
      const stamp=element('span'); bind(stamp,()=>{stamp.textContent = `${new Date(revision.updatedMs).toLocaleString(window.SubnetI18n.locale)} · ${size(revision.bytes)}`;}); info.append(stamp);
      const author = element('small','account-id'); author.textContent = revision.author; info.append(author); row.append(info,picker ? button('Ava',()=>selectPickerFile(file,revision.id)) : button('Laadi alla',()=>download(file,revision.id)));
      const restoreRequestId = requestId();
      if(!picker && file.capabilities.edit && revision.id !== file.revision) row.append(button('Taasta versioon',()=>mutate(async()=> {
        const bytes = await cloud.download(file.id,revision.id);
        await cloud.upload(JSON.stringify({file:file.id,revision:file.revision,requestId:restoreRequestId}),bytes,()=>{});
      })));
      body.append(row);
    }
    if(!picker)body.append(element('p','muted','Taastamine lisab uue versiooni. Senine ajalugu säilib.'));
  } catch(error) { body.replaceChildren(element('p','is-error',errorMessage(error))); }
}
async function move(file) {
  let target = null, stack = []; const token = requestId();
  showDialog('Liiguta','Liiguta siia',()=>mutate(()=>call('organize',{id:file.id,version:file.version,change:'move',parent:target.id,requestId:token})));
  const dialogEpoch = dialogGeneration;
  async function browse(destination) {
    target = destination; $('dialog-submit').disabled = !target?.capabilities.create || target.id === file.parent;
    const body = $('dialog-body'); body.replaceChildren(element('p','','Laadin…'));
    try {
      let page, items = [], next = null;
      do { page = await call('list',{parent:destination?.id || null,cursor:next}); items.push(...page.entries); next = page.cursor; } while(next);
      if(!$('drive-dialog').open || dialogEpoch !== dialogGeneration || cloud.needs_refresh()) return; body.replaceChildren();
      if(destination) { const name = element('p','picker-location'); if(destination.id === root?.id) text(name,'Minu failid'); else name.textContent = destination.name; body.append(button('← Tagasi',()=>{stack.pop(); browse(stack.at(-1));}),name); }
      for(const item of items.filter(e=>e.folder && e.spaceId === file.spaceId && e.id !== file.id)) { const choice=button('',()=>{stack.push(item); browse(item);},'folder-choice'); if(item.id === root?.id) text(choice,'Minu failid'); else choice.textContent=item.name; body.append(choice); }
      if(!body.children.length) body.append(element('p','','Sobivaid kaustu ei ole.'));
    } catch(error) { body.replaceChildren(element('p','is-error',errorMessage(error))); }
  }
  await browse(null);
}
async function share(file) {
  let principal; const checks = {}, token = requestId();
  showDialog('Jaga','Jaga',()=>mutate(()=>call('grant',{root:file.id,subject:principal.value.trim(),requestId:token,capabilities:{read:true,propose:false,create:!!checks.create?.checked,edit:checks.edit.checked,delete:checks.delete.checked}})));
  const dialogEpoch = dialogGeneration;
  principal = field('Saaja konto ID'); principal.maxLength = 100;
  $('dialog-body').append(element('p','muted','Saaja peab olema korra pilveruumi sisse loginud. Konto ID saab kopeerida failivaate alt.'));
  const permissions = element('div','permission-options');
  permissions.append(element('span','','Lugemine'));
  for(const [key,label] of [['create','Loomine'],['edit','Muutmine'],['delete','Kustutamine']]) {
    if(key === 'create' && !file.folder) continue;
    const wrap = element('label'); const input = element('input'); input.type = 'checkbox'; checks[key] = input; wrap.append(input,element('span','',label)); permissions.append(wrap);
  }
  $('dialog-body').append(permissions); const roster = element('div','share-roster'); $('dialog-body').append(roster);
  try {
    const grants = await call('grants',{id:file.id}); if(!$('drive-dialog').open || dialogEpoch !== dialogGeneration || cloud.needs_refresh()) return;
    for(const grant of grants.filter(g=>g.active)) {
      const row = element('div','share-row'); const who = element('code','account-id'); who.textContent = grant.subject; row.append(who);
      if(grant.root === file.id) row.append(button('Eemalda ligipääs',()=>mutate(()=>call('grant',{...grant,requestId:requestId(),active:false}))));
      else row.append(element('span','muted','Päritud kaustast'));
      roster.append(row);
    }
  } catch(error) { roster.append(element('p','is-error',errorMessage(error))); }
}
$('dialog-form').addEventListener('submit', async event => { event.preventDefault(); if(!busy && modalAction) { $('dialog-submit').disabled = true; await modalAction(); $('dialog-submit').disabled = false; } });
$('dialog-close').addEventListener('click',closeDialog); $('dialog-cancel').addEventListener('click',closeDialog);
$('drive-dialog').addEventListener('cancel',event=>{if(busy)event.preventDefault();});
$('new-folder').addEventListener('click',newFolder);
$('upload-file').addEventListener('click',()=> $('upload-input').click());
$('upload-input').addEventListener('change',event=>{const files=[...event.target.files];event.target.value='';uploadFiles(files);});
$('replace-input').addEventListener('change',event=>{const files=[...event.target.files];event.target.value='';uploadFiles(files,replaceFile);});
$('copy-account').addEventListener('click',async()=>{try {await navigator.clipboard.writeText(account.principal);message('Konto ID kopeeritud.');}catch {message(account.principal);}});
$('sign-in').addEventListener('click',async()=>{
  if(!cloud)return; const attempt=++loginAttempt;
  try {message('Lõpeta sisselogimine Internet Identity aknas.');text($('sign-in'),'Ava sisselogimine uuesti');await cloud.sign_in();if(attempt===loginAttempt)await refresh();}
  catch(error){if(attempt===loginAttempt)message(errorMessage(error),true);}finally{if(attempt===loginAttempt)text($('sign-in'),'Logi sisse');}
});
$('sign-out').addEventListener('click',async()=>{closePreview();++generation;clearCache();account=null;root=null;folder=null;clearFiles('Login välja…');updateAccount();try{await cloud.sign_out();}catch(error){message(errorMessage(error),true);}await refresh();});
$('refresh-files').addEventListener('click',()=>refresh(true));
$('more-files').addEventListener('click',()=>{if(!loading)loadFiles(generation,true);});
$('preview-close').addEventListener('click',closePreview);
document.addEventListener('click',event=>{for(const menu of fileMenus)if(!menu.contains(event.target))menu.open=false;});
document.addEventListener('keydown',event=>{if(event.key==='Escape'){closeFileMenus();if(picker && !$('drive-dialog').open)postSelection({action:'cancel'});}});
$('preview-dialog').addEventListener('close',releasePreview);
$('preview-dialog').addEventListener('cancel',event=>{event.preventDefault();closePreview();});
$('preview-download').addEventListener('click',()=>{if(previewState)download(previewState.file);});
window.addEventListener('hashchange',()=>{closePreview();route();});
window.addEventListener('storage',event=>{if(!event.key||['ic-identity','ic-delegation','subnet.files.changed'].includes(event.key)) {if(cloud?.needs_refresh()){closePreview();++generation;clearCache();account=null;root=null;folder=null;$('drive-dialog').close();clearFiles('Kontrollin sisselogimist…');}refresh(true);}});
window.addEventListener('focus',()=>{if(Date.now()-lastRefresh>5000)refresh();});
document.addEventListener('visibilitychange',()=>{if(!document.hidden)refresh();});
setInterval(()=>{if(!document.hidden && cloud?.needs_refresh())refresh();},1000);
setInterval(()=>{if(!document.hidden && !loading && account?.authenticated)refresh();},30000);
window.addEventListener('beforeunload',event=>{if(busy){event.preventDefault();event.returnValue='';}});
heading(); updateAccount();
try {
  const response=await fetch('/apps.json'); if(!response.ok)throw new Error(t('Rakenduste register ei ole kättesaadav.'));
  registry=(await response.json()).apps; setupPicker(); await init({module_or_path:'__CLOUD_WASM__'});cloud=new CloudPortal();await refresh();
}catch(error){text($('account-status'),'Konto pole kättesaadav');message(errorMessage(error),true);}
