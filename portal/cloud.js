import { creationContext, editorContext, creationLabels, returnStateKey } from './workspace-context.js';
import { createPublicLinks } from './drive-public.js';
import { createTeamSpaces } from './drive-teams.js';
import init, { CloudPortal } from "__CLOUD_BINDINGS__";
import { createFolderTree, createFileDetails } from './drive-navigation.js';
import { previewFormat, renderOfficePreview, createThumbnails } from './drive-preview.js';
import { createDrivePeople } from './drive-people.js';
import { createDriveInsights, createRevisionHistory } from './drive-insights.js';
import { LastDriveAction, impactKey, recoveryPreview } from './drive-recovery.js';
import { createAccountEmail } from './account-email.js';
import { documentDisplayName, documentStorageName, contentLocation, canManage, driveRoute, driveHref, canonicalDriveLocation, OBJECT_UUID, DriveSelection, defaultPreferences, browseRoute, selectionCapabilities, selectionRoots, roleCapabilities, sharingRole, stableSuggestionOrder, menuPosition } from "./drive-browser.js";
import { ARCHIVE_LIMIT, downloadArchive } from "./download-archive.js";
import { TransferQueue, TransferCancelled, localFiles, droppedFiles, uploadJobs, listChildren, walkFiles } from "./drive-transfers.js";
const { t, text, bind, href: localizedHref } = window.SubnetI18n;
const $ = id => document.getElementById(id);
const currentRoute=()=>driveRoute(location);
function normalizeRoute(){
  const path=canonicalDriveLocation(location);
  if(path!==location.pathname+location.search+location.hash)window.history.replaceState(null,'',path);
  // Also keep native Cmd-click/open-in-new-tab language links on this object.
  for(const link of document.querySelectorAll('a[data-language]'))link.setAttribute('href',localizedHref(location.href,link.getAttribute('data-language')));
}
let cloud, account, root, folder, cursor, registry = [];
let generation = 0, loading = false, restoring = false, busy = false, lastRefresh = 0;
let entries = [], trail = [], currentView = 'files', modalAction, replaceFile, loginAttempt = 0, dialogGeneration = 0;
let previewState = null, previewQueue = Promise.resolve();
let inlineRename = null;
let fileMenus = [];
const selection = new DriveSelection();
let fileClipboard=null,internalDrag=null;
let preferences = defaultPreferences(), activeQuery = null, selectionRoute = null, preferenceQueue = Promise.resolve(), preferenceRevision = 0;
const fileRows = new Map(), operations = new Map();
let refreshing = false, moreLoading = false, uploading = false, readSequence = 0, quotaSequence = 0, mutationRevision = 0, sessionVersion = 0;
const working = id => operations.has(String(id));
const signedSession = () => ({principal:account?.principal, version:sessionVersion});
const currentSession = session => session.principal===account?.principal && session.version===sessionVersion && !!account?.authenticated && !cloud?.needs_refresh();
let picker = null, pickerSelection = null;
let displayedRoute = null, loadedPages = 1, dragDepth = 0;
let homeFolderSignature, homeFolders=[], lastVisitedRoute=null;
const creationFolder=()=>['files','shared'].includes(currentView)?folder:root;
function closeFileMenus(except=null) { for(const menu of fileMenus) if(menu!==except)menu.open=false; }
function openFileMenu(menu) {
  if(!menu)return;closeFileMenus(menu);menu.open=true;
  const anchor=menu.querySelector('summary'),items=menu.querySelector('.file-menu-items');
  if(!anchor?.getBoundingClientRect || !items)return;
  const point=menuPosition(anchor.getBoundingClientRect(),items.getBoundingClientRect(),{width:window.innerWidth,height:window.innerHeight});
  items.setAttribute('style',`position:fixed;left:${point.left}px;top:${point.top}px;right:auto`);
}
const call = (operation, payload = {}) => cloud.drive(operation, JSON.stringify(payload));
const requestId = () => crypto.randomUUID();
// Metadata only, memory only, scoped to this signed-in session. A cached view
// never grants authority: mutations remain disabled until revalidation finishes.
const views = new Map();
function clearCache() { views.clear(); }
function queryKey() { return JSON.stringify(activeQuery); }
function syncBrowseControls() {
  $('file-list').classList.toggle('is-grid',preferences.view==='grid');
  $('view-list').setAttribute('aria-pressed',String(preferences.view!=='grid'));
  $('view-grid').setAttribute('aria-pressed',String(preferences.view==='grid'));
  const recent=currentView==='recent',content=activeQuery?.scope==='content';
  if(currentView==='search')$('search-mode').value=content?'content':'name';
  bind($('search-text'),()=>{$('search-text').placeholder=t($('search-mode').value==='content'?'Otsi failide sisust':'Otsi failinimede järgi');});
  for(const id of ['search-type','search-owner'])($(id).parentElement||$(id)).hidden=$('search-mode').value==='content';
  $('date-filter').hidden=$('search-mode').value==='content';$('content-search-help').hidden=$('search-mode').value!=='content';
  $('sort-control').hidden=currentView==='home'||content;$('sort-direction').hidden=currentView==='home'||content;
  for(const field of ['name','modified','size'])$('column-'+field).disabled=currentView==='home'||content;
  $('sort-field').value=recent?(preferences.recentSort||'opened'):preferences.sort;
  text($('sort-direction'),(recent?(preferences.recentDescending??true):preferences.descending)?'Kahanev':'Kasvav');
  $('search-current-folder').disabled=!folder&&!activeQuery?.parent;
  $('search-clear').hidden=currentView!=='search';
  $('file-columns').hidden=preferences.view==='grid' || !!picker || currentView==='trash';
  for(const [field,label] of [['name','Nimi'],['modified',recent?'Viimati avatud':'Muudetud'],['size','Suurus']]) {
    const active=(recent && field==='modified'?'opened':field)===$('sort-field').value;
    text($('column-'+field),currentView==='home' && field==='modified'?'Soovituse põhjus':label);
    $('column-'+field).setAttribute('data-direction',active && currentView!=='home' && !content?((recent?(preferences.recentDescending??true):preferences.descending)?'descending':'ascending'):'');
  }
  const params=currentView==='search'?new URLSearchParams(currentRoute().slice(content?9:8)):null;
  if(params) {
    $('search-text').value=params.get('q')||'';$('search-type').value=params.get('type')||'';
    $('search-owner').value=params.get('owner')||'';$('search-after').value=params.get('after')||'';
    $('search-before').value=params.get('before')||'';$('search-location').value=params.has('in')?'current':'';
  }
}
function selectedFiles() {return entries.filter(file=>selection.ids.has(file.id));}
function selectionAllowed() {return selectionCapabilities(selectedFiles(),account?.principal,currentView==='trash');}
function selectionUnavailable() {return loading || restoring || busy || !account?.authenticated || cloud?.needs_refresh() || selectedFiles().some(file=>working(file.id));}
function syncSelection() {
  if(picker)return;
  selection.reconcile(entries.map(file=>file.id));
  for(const [id,{row}] of fileRows) {
    row.setAttribute('aria-selected',String(selection.ids.has(id)));
    row.classList.toggle('is-cut',!!fileClipboard?.cut && fileClipboard.files.some(file=>file.id===id));
    row.tabIndex=selection.focus===id?0:-1;
    if(row.selectionInput)row.selectionInput.checked=selection.ids.has(id);
  }
  $('clipboard-cut').disabled=selectionUnavailable() || !selectionAllowed().move;
  $('clipboard-copy').disabled=selectionUnavailable() || !selectionAllowed().copy;
  $('clipboard-paste').hidden=!fileClipboard || !!picker;
  $('clipboard-paste').disabled=!canPaste();
  const count=selection.ids.size;
  $('selection-toolbar').hidden=!count;
  $('browser-action-bar').classList.toggle('has-selection',!!count);
  $('browse-controls').inert=!!count;$('browse-controls').setAttribute('aria-hidden',String(!!count));
  if(!count)$('selection-more').open=false;
  text($('selection-count'),'{count} valitud',{count});
  $('select-all-files').disabled=loading || !entries.length;
  const allowed=selectionAllowed(),unavailable=selectionUnavailable();
  for(const name of Object.keys(allowed)) {
    $('selection-'+name).disabled=!!unavailable || !allowed[name];
    const menu=$('selection-menu-'+name);if(menu)menu.disabled=!!unavailable || !allowed[name];
  }
  for(const name of ['share','download','move','trash'])$('selection-'+name).hidden=currentView==='trash';
  $('selection-restore').hidden=currentView!=='trash';
  $('selection-details').disabled=unavailable || count!==1 || currentView==='trash';
  $('show-details').disabled=unavailable || currentView==='trash' || (!count && !folder) || count>1;
  details.selection(selectedFiles());
}
// Snapshot the selection/session and keep each request ID for explicit retries.
// Successful items are never replayed when a partial failure reopens the dialog.
async function selectionAncestor(id,session) {
  if(!currentSession(session))throw new TransferCancelled();
  try{return await call('get',{id});}
  catch(error){if(String(error?.message||error)==='Not found or access unavailable')return null;throw error;}
}
function batchAction(files,run,{label='Salvestan…',roots=false,clearSelection=false,quota=false}={}) {
  const session=signedSession(),tokens=new Map(files.map(file=>[file.id,requestId()])),done=new Set();let plan;
  return ()=>{
    if(!currentSession(session))return;
    return mutate(async()=>{
      if(!plan)plan=roots ? await selectionRoots(files,id=>selectionAncestor(id,session)) : files.map(file=>({file,ids:[file.id]}));
      const failures=[];
      for(const item of plan) {
        if(!currentSession(session))return;
        if(done.has(item.file.id))continue;
        try {
          await run(item.file,tokens.get(item.file.id));done.add(item.file.id);
          if(currentSession(session) && clearSelection)for(const id of item.ids)selection.ids.delete(id);
        }catch(error){failures.push(`${item.file.name}: ${errorMessage(error)}`);}
      }
      if(failures.length)throw new Error(failures.join('\n'));
    },{ids:files.map(file=>file.id),label,quota});
  };
}
function setStars(files,starred) {
  return batchAction(files,file=>call('star',{id:file.id,starred}))();
}
function selectionNames(files) {
  const names=element('ul','selection-names');
  for(const file of files){const name=element('li');name.textContent=file.name;names.append(name);}
  $('dialog-body').append(names);
}
async function savePreferences() {
  const session=signedSession(),sequence=++preferenceRevision, value={view:preferences.view,sort:preferences.sort,descending:preferences.descending};
  preferenceQueue=preferenceQueue.catch(()=>{}).then(async()=>{
    if(currentSession(session))await call('preferences',value);
  });
  try {await preferenceQueue;}catch(error){if(currentSession(session))message(errorMessage(error),true);}
  return sequence===preferenceRevision && currentSession(session);
}
function remember(key, value) {
  views.delete(key); views.set(key, {value, at:Date.now()});
  if(views.size > 12) views.delete(views.keys().next().value);
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
  return ({'Use one to eight search words':'Sisesta üks kuni kaheksa otsingusõna.','Search text is too long':'Otsingutekst on liiga pikk.','Retry content indexing later':'Proovi mõne minuti pärast uuesti.','Choose a folder to narrow this view':'Vali otsingu kitsendamiseks kaust.','Access request daily limit reached':'Ligipääsutaotluste päevapiir on täis. Proovi homme uuesti.','Access expiry must be in the future':'Ligipääsu lõpp peab olema tulevikus.','Invalid folder path':'Kausta tee ei sobi.','Too many transfer items':'Liiga palju üksusi ühes ülekandes.','Duplicate upload path':'Üleslaadimise teed kattuvad.','Folder changed; refresh and retry':'Kausta sisu muutus. Värskenda ja proovi uuesti.',
    'Parent folder is not ready. Retry it first.':'Ülemkaust ei ole valmis. Proovi esmalt selle loomist uuesti.',
    "Email is not configured":"E-posti saatmine ei ole veel seadistatud.",
    "Enter a supported email address":"Sisesta toetatud e-posti aadress.",
    "Enter a valid email address":"Sisesta korrektne e-posti aadress.",
    "Incorrect verification code":"Kinnituskood ei ole õige.",
    "Verification expired. Request another code.":"Kinnitamine aegus. Küsi uus kood.",
    "Email binding changed. Request another code.":"Aadressi seos muutus. Küsi uus kood.",
    "An account can have up to 10 email addresses":"Kontol võib olla kuni 10 e-posti aadressi.",
    "Email limit reached. Please try later.":"Kirjade saatmise piir on täis. Proovi hiljem.",
    "This email is already verified on your account":"See e-post on sinu kontol juba kinnitatud.",
    "Email request changed or expired":"Kinnituspäring muutus või aegus.",
    "Verification request changed or expired":"Kinnituspäring muutus või aegus.",
    "Email is being sent. Please wait before retrying.":"Kirja saatmine on pooleli. Oota enne korduskatset.",
    "Email request expired":"Kirja saatmise päring aegus.",
    "Email could not be sent":"Kirja ei õnnestunud saata.",
    "Email is still being prepared":"Kirja valmistatakse veel ette.",
    "Email is not linked to this account":"See e-post ei ole selle kontoga seotud.",
    "Verify an email address before sending invitations":"Kinnita enne kutsete saatmist enda e-post.",
    "Invitation is not available for this account. Verify the invited email or request a new invitation.":"See kutse ei ole sellele kontole saadaval. Kinnita kutsutud aadress või küsi uus kutse.",
    "Keep at least one team manager":"Tiimil peab olema vähemalt üks haldaja.",
    "Choose an existing signed-in account":"Vali olemasolev Subneti konto.",
    "External sharing is disabled for this team":"Selle tiimi väline jagamine on keelatud.",
    "Invitation is not available for this account":"See kutse ei ole sellele kontole saadaval.",
    "Invitation is no longer active":"See kutse ei ole enam aktiivne.",
    "Email capacity reached":"E-posti teenuse maht on täis.",
    "Invitation capacity reached":"Kutsete arv on täis.",
    "Item not found or not permitted":"Fail või kaust ei ole kättesaadav.",
    'Not found or access unavailable':'Fail või kaust ei ole kättesaadav.',
    'This item changed; refresh and try again':'Fail või kaust on vahepeal muutunud. Värskenda ja proovi uuesti.',
    'An item with this name already exists':'Selle nimega fail või kaust on juba olemas.',
    'Cloud storage quota reached':'Pilveruumi maht on täis.',
    'Revision limit reached':'Faili versioonide piir on täis.',
    'Use a display name of 1 to 80 characters without control characters':'Sisesta 1–80 märgi pikkune nimi ilma juhtmärkideta.',
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
  $('account-menu').hidden = !signed;
  $('signed-out-actions').hidden = signed;
  $('failid').setAttribute('data-signed-out', String(!signed));
  $('files-sign-in').disabled = !cloud || restoring;
  if (!signed) $('account-menu').open = false;
  $('account-principal').textContent = signed ? account.principal : '';
  $('sign-in').disabled = !cloud || restoring; $('sign-out').disabled = restoring || busy || uploading || operations.size>0;
  $('manage-space').hidden=!signed || !folder?.spaceAccess?.teamName || !!picker;
  $('manage-space').disabled=loading || restoring || busy;
  $('account-name').disabled = !signed || restoring || busy || !!cloud?.needs_refresh();
  $('account-email').disabled = !signed || restoring || busy || !!cloud?.needs_refresh();
  $('refresh-files').disabled = !signed || loading || restoring || refreshing;
  $('refresh-files').classList.toggle('is-refreshing', loading || restoring || refreshing);
  $('file-area').setAttribute('aria-busy', String(loading || restoring));
  $('new-folder').disabled = !signed || !folder?.capabilities.create || loading || busy || working(`children:${folder?.id}`) || !['files','shared'].includes(currentView);
  $('new-menu-folder').disabled=!signed || !creationFolder()?.capabilities.create || loading || busy || restoring || !!cloud?.needs_refresh();
  $('new-menu-upload').disabled=!canUploadFiles();
  $('new-menu-upload-folder').disabled=!canUploadFiles();
  for(const link of $('new-apps').children) {
    const destination=creationFolder();
    const disabled=!signed || !destination?.capabilities.create || loading || busy || restoring || !!cloud?.needs_refresh();
    link.setAttribute('aria-disabled',String(disabled));link.tabIndex=disabled?-1:0;
    const app=registry.find(item=>item.id===link.dataset.appId);
    link.href=localizedHref(creationContext(app,signed && destination?.capabilities.create?destination:null,location));
  }
  text($('new-folder-label'),working(`children:${folder?.id}`) ? 'Loon kausta…' : 'Uus kaust');
  $('new-folder').hidden = $('upload-file').hidden = !['files','shared'].includes(currentView) || !folder;
  $('upload-file').disabled = !canUploadFiles();
  $('file-drop-hint').hidden = !signed || !folder?.capabilities.create || !!picker || !['files','shared'].includes(currentView);
  if(!canUploadFiles()) resetDrop();
  $('empty-trash').hidden = currentView !== 'trash' || !signed || !!picker;
  $('empty-trash').disabled = !signed || loading || restoring || busy || !!cloud?.needs_refresh() || (!cursor && !entries.some(file=>canManage(file,account.principal))) || entries.some(file=>working(file.id));
  for(const [id, {row}] of fileRows) updateRowState(row,id);
  $('more-files').disabled = loading || moreLoading || refreshing;
  $('copy-account').hidden = !signed;
  $('ai-account').hidden = !signed;
  $('ai-principal').textContent = signed ? account.principal : '';
  $('ai-copy-account').disabled = !signed || restoring || !!cloud?.needs_refresh();
  folderTree.setRoot(signed && !picker && !restoring?root:null);
  $('sidebar-storage').hidden=!signed || !!picker;
  updateUndo();
  $('storage-meter').max=Math.max(1,account?.quotaBytes||1);$('storage-meter').value=account?.usedBytes||0;
  if(signed && account.displayName) {$('account-status').textContent=account.displayName;$('account-status').setAttribute('data-no-translate','');}
  else {$('account-status').removeAttribute('data-no-translate');text($('account-status'), signed ? 'Minu konto' : 'Sinu pilveruum');}
  $('account-status').title = signed ? account.principal : '';
  updatePicker();syncSelection();
  bind($('storage-usage'), () => { $('storage-usage').textContent = signed ? t('{used} / {quota} kasutusel', { used: size(account.usedBytes), quota: size(account.quotaBytes) }) : t('Sinu failid on seotud sinu kontoga.'); });
}
function heading() {
  const label = (currentView === 'teams'||folder?.spaceAccess?.teamName)?'Tiimiruumid':currentView === 'safety'?'Peidetud ja blokeeritud':currentView === 'inbox'?'Teavitused':currentView === 'contacts'?'Minu kontaktid':currentView === 'activity'?'Tegevused':currentView === 'storage'?'Salvestusruum':currentView === 'home'?'Kodu':currentView === 'ai' ? 'Tööta AI-ga' : currentView === 'apps' ? 'Rakendused' : currentView === 'trash' ? 'Prügikast' : currentView === 'shared' ? 'Minuga jagatud' : currentView === 'recent' ? 'Hiljutised' : currentView === 'starred' ? 'Tärniga' : ['link','public'].includes(currentView) ? 'Jagatud link' : currentView === 'search' ? 'Otsingutulemused' : 'Minu failid';
  text($('page-name'), label);
  if (folder && folder.id !== root?.id && currentView!=='search') { bind($('files-title'),()=>{$('files-title').textContent=folder?.name||'';}); }
  else text($('files-title'),currentView==='home'?'Jätka pooleli jäänust':label);
  text($('drive-description'), currentView === 'trash' ? 'Kustutatud failid ja kaustad. Taasta need või kustuta jäädavalt.' : currentView === 'shared' ? 'Failid ja kaustad, mille teised on sinuga jaganud.' : currentView === 'search' ? 'Vasted sinu ligipääsuga failidest.' : 'Sinu salvestatud tööd.');
  $('drive-description').hidden = ['files','home','recent','starred'].includes(currentView);
  $('failid').classList.toggle('is-home',currentView==='home');
  $('home-filter-toggle').hidden=currentView!=='home';
  $('home-folders').hidden=$('home-recent-heading').hidden=currentView!=='home' || !account?.authenticated;
  $('apps-view').hidden = currentView !== 'apps'; $('ai-view').hidden = currentView !== 'ai'; $('failid').hidden = ['apps','ai','activity','storage','inbox','contacts','safety','teams','public'].includes(currentView);
  $('public-view').hidden=currentView!=='public';
  $('insights-view').hidden=!['activity','storage'].includes(currentView);
  $('people-view').hidden=!['inbox','contacts','safety'].includes(currentView);
  for (const [id, view] of [['teams-nav','teams'],['inbox-nav','inbox'],['activity-nav','activity'],['storage-nav','storage'],['home-nav','home'],['files-nav','files'],['shared-nav','shared'],['recent-nav','recent'],['starred-nav','starred'],['trash-nav','trash'],['apps-nav','apps'],['ai-nav','ai']]) {
    const active=currentView===view && !(view==='files' && folder?.spaceAccess?.teamName) || view==='teams' && !!folder?.spaceAccess?.teamName;
    $(id).classList.toggle('nav-active', active);
    if (active) $(id).setAttribute('aria-current','page'); else $(id).removeAttribute('aria-current');
  }
  $('breadcrumbs').replaceChildren();
  if (folder) {
    const team=folder.spaceAccess?.teamName;const start = element('a', '', team?'Tiimiruumid':currentView === 'shared' ? 'Minuga jagatud' : 'Minu failid'); start.href = team?'#tiimid':currentView === 'shared' ? '#jagatud' : '#failid'; $('breadcrumbs').append(start);
    for (const item of trail.filter(item => item.id !== root?.id)) {
      const separator = element('span'); separator.textContent = '/';
      const link = element('a'); link.textContent = item.name; link.href = driveHref(`#folder/${item.id}`);
      if (item.id === folder.id) link.setAttribute('aria-current','page');
      $('breadcrumbs').append(separator, link);
    }
  } else $('breadcrumbs').append(element('span','',label));
  syncBrowseControls();folderTree.render();
}
function clearFiles(source) {
  cancelInlineRename();
  thumbnails.clear();entries = []; cursor = null; selection.clear();fileRows.clear(); fileMenus=[]; $('file-list').replaceChildren(); $('more-files').hidden = true;
  text($('file-status'), source); $('file-status').hidden = false;
  $('home-folder-list').replaceChildren();homeFolderSignature=undefined;homeFolders=[];
}
const visibleName = file => documentDisplayName(file, appFor(file));
const storedName = (name,file) => documentStorageName(name.trim(),file,appFor(file));
function appFor(file) { return registry.find(app => app.cloud && app.openFormats?.some(format => format.id === file.format?.craft?.format_id && format.version === file.format?.craft?.format_version)); }
function rememberReturnState() {
  if(picker || !account?.authenticated)return;
  try {sessionStorage.setItem(returnStateKey(account.principal,location),JSON.stringify({ids:[...selection.ids].slice(0,200),scroll:window.scrollY,pages:Math.min(loadedPages,10),at:Date.now()}));} catch {}
}
async function restoreReturnState() {
  if(picker || !account?.authenticated)return;
  try {
    const key=returnStateKey(account.principal,location),value=sessionStorage.getItem(key);
    if(!value)return;sessionStorage.removeItem(key);const state=JSON.parse(value);
    if(!Number.isFinite(state.at) || Date.now()-state.at>86400000 || !Array.isArray(state.ids))return;
    const epoch=generation,session=signedSession(),query={...activeQuery};
    const pages=Math.min(10,Math.max(1,Number(state.pages)||1));
    for(let page=1;page<pages && cursor;page++) {
      const next=await readBrowsePage(query,cursor);
      if(epoch!==generation || !currentSession(session))return;
      applyPage(next,true);
    }
    const allowed=new Set(entries.map(f=>f.id));selection.ids=new Set(state.ids.slice(0,200).filter(id=>allowed.has(id)));selection.focus=selection.anchor=[...selection.ids][0]||null;syncSelection();
    requestAnimationFrame(()=>window.scrollTo(0,Math.max(0,Math.min(Number(state.scroll)||0,document.documentElement.scrollHeight))));
  } catch {}
}
function editorHref(file) {
  const app = appFor(file);
  return app && file.capabilities.read && /^[a-z]+$/.test(app.id)
    ? localizedHref(editorContext(`/apps/${app.id}/?${new URLSearchParams({cloud_service:'cloud',cloud_space:file.spaceId,cloud_file:file.id})}`,location)) : null;
}
function setupPicker() {
  const params=new URLSearchParams(location.search), mode=params.get('picker');
  if(!mode) return;
  const app=registry.find(app=>app.id===params.get('app') && app.cloud), channel=params.get('channel');
  if(!['open','save'].includes(mode) || !app || !/^[a-f0-9-]{36}$/.test(channel||'') || window.parent===window) throw new Error(t('Failivalijat ei saa siin avada.'));
  picker={mode,app,channel,done:false};
  if(mode==='save' && new RegExp(`^${OBJECT_UUID}$`,'i').test(params.get('parent')||'') && new RegExp(`^${OBJECT_UUID}$`,'i').test(params.get('space')||'')) {
    picker.initialSpace=params.get('space');picker.initialParent=params.get('parent');location.hash='#folder/'+picker.initialParent;
  }
  document.body.classList.add('cloud-picker');
  $('file-list').setAttribute('role','list');$('file-list').removeAttribute('aria-multiselectable');
  if(params.get('theme')==='dark') document.body.classList.add('picker-dark');
  $('picker-footer').hidden=false;$('picker-name-field').hidden=mode!=='save';
  $('picker-name').value=documentDisplayName({name:params.get('name')||''},app);
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
  $('picker-owner').replaceChildren(...(picker.mode==='save' && folder?[destinationOwner(folder)]:[]));
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
function updateRowState(row,id) {
  const operation=operations.get(id);
  row.setAttribute('aria-busy',String(!!operation));
  row.classList.toggle('is-working',!!operation);
  row.operationStatus.hidden=!operation;
  if(operation)text(row.operationStatus,operation.label);
  for(const control of row.querySelectorAll('button')) control.disabled=!!operation || loading || restoring || busy || !!cloud?.needs_refresh();
  if(inlineRename?.row===row)inlineRename.input.disabled=!!operation || loading || restoring || !!cloud?.needs_refresh();
  for(const link of row.querySelectorAll('a')) link.setAttribute('aria-disabled',String(!!operation));
}
function cancelInlineRename() {
  const edit=inlineRename;inlineRename=null;
  if(edit)edit.title.replaceChildren(edit.label);
}
function editableName(file) {
  return !picker && currentView!=='trash' && file.capabilities.edit
    && !loading && !restoring && !busy && !working(file.id) && account?.authenticated && !cloud?.needs_refresh();
}
function startInlineRename(file,row,title,label) {
  if(!editableName(file) || fileRows.get(file.id)?.row!==row || $('drive-dialog').open || $('preview-dialog').open)return;
  if(inlineRename?.row===row)return;
  cancelInlineRename();closeFileMenus();
  const input=element('input','file-name-input');input.type='text';input.value=visibleName(file);input.maxLength=160;
  bind(input,()=>{input.setAttribute('aria-label',t(file.folder ? 'Kausta nimi' : 'Faili nimi'));input.title=t('Enter salvestab, Esc tühistab.');});
  const edit={file,row,title,label,input,saving:false,token:requestId()};inlineRename=edit;
  async function save() {
    if(inlineRename!==edit || edit.saving)return;
    if(storedName(input.value,file)===file.name){cancelInlineRename();return;}
    if(!editableName(file)) {cancelInlineRename();return;}
    if(!input.value.trim() || new TextEncoder().encode(input.value).length>160 || /[\/\\]/.test(input.value)) {
      message('Sisesta sobiv faili nimi (kuni 160 baiti).',true);return;
    }
    const name=storedName(input.value,file);edit.saving=true;message();
    const saved=await mutate(()=>trackedOrganize(file,{id:file.id,version:file.version,change:'rename',name,requestId:edit.token}),{ids:[file.id],label:'Nimetan ümber…'});
    if(inlineRename!==edit)return;
    edit.saving=false;
    if(saved)cancelInlineRename();
    else input.focus({preventScroll:true});
  }
  input.addEventListener('input',()=>{edit.token=requestId();});
  input.addEventListener('blur',save);
  input.addEventListener('click',event=>event.stopPropagation());
  input.addEventListener('dblclick',event=>event.stopPropagation());
  input.addEventListener('keydown',event=>{
    event.stopPropagation();
    if(event.isComposing)return;
    if(event.key==='Enter'){event.preventDefault();save();}
    if(event.key==='Escape'){event.preventDefault();cancelInlineRename();row.focus({preventScroll:true});}
  });
  title.replaceChildren(input);input.focus({preventScroll:true});
  input.select();
}
function createFileRow(file) {
    const row = element('article', 'file-row'); row.dataset.fileId = file.id;
    if(picker)row.setAttribute('role','listitem');
    row.addEventListener('click',event=>{if(working(file.id)){event.preventDefault();event.stopPropagation();}},true);
    const icon = element('span', `file-icon${file.folder ? ' folder-icon' : ''}`); icon.textContent = file.folder ? '' : file.shortcut?'↗':(appFor(file)?.symbol || file.name.match(/\.([a-z0-9]{1,4})$/i)?.[1].toUpperCase() || '↥'); icon.setAttribute('aria-hidden','true');if(file.color)icon.dataset.folderColor=file.color;
    const detail = element('div','file-detail'); const title = element('h2');
    if(!picker) {
      const label=element('span','file-name-button');label.textContent=visibleName(file);title.append(label);
      row.addEventListener('keydown',event=>{if(event.key==='F2' && editableName(file)){event.preventDefault();startInlineRename(file,row,title,label);}});
    } else if(file.folder && currentView!=='trash') {
      const link=element('a','folder-link');link.textContent=file.name;link.href=driveHref(`#folder/${file.id}`);title.append(link);
    } else title.textContent=visibleName(file);
    const meta = element('p'); bind(meta, () => { meta.textContent = currentView==='home' && file.reason?suggestionReason(file):`${file.folder ? t('Kaust') : file.shortcut?t('Otsetee'):(appFor(file)?.id==='wordcraft'?t('Tekstidokument')+' · ':'')+size(file.bytes || 0)} · ${new Date(file.updatedMs).toLocaleDateString(window.SubnetI18n.locale, {dateStyle:'medium'})}`; });
    const name=element('div','file-name-line');name.append(title);
    const badge=currentView==='trash' ? null : sharingBadge(file);if(badge)name.append(badge);
    const status=element('span','file-operation-status');status.hidden=true;status.setAttribute('role','status');row.operationStatus=status;
    detail.append(name, meta, status); row.append(icon, detail);
    if(file.contentMatch){const excerpt=element('p','content-snippet');excerpt.textContent=file.contentMatch.snippet;detail.append(excerpt);const location=element('small','content-location');bind(location,()=>{location.textContent=contentLocation(file.contentMatch.location,t);});detail.append(location);if(file.contentMatch.partial)detail.append(element('small','muted','Otsing hõlmab osa selle faili tekstist.'));}
    if(!picker) {
      row.setAttribute('role','row');row.setAttribute('aria-label',visibleName(file));
      connectInternalDrag(row,file);
      icon.setAttribute('role','gridcell');detail.setAttribute('role','gridcell');
      const checkbox=element('input','file-select');checkbox.type='checkbox';row.selectionInput=checkbox;
      bind(checkbox,()=>checkbox.setAttribute('aria-label',t('Vali {name}',{name:visibleName(file)})));
      checkbox.addEventListener('click',event=>{event.stopPropagation();selection.select(file.id,entries.map(f=>f.id),{toggle:true,range:event.shiftKey});syncSelection();});
      row.append(checkbox);
      row.addEventListener('click',event=>{
        if(event.target.closest('a, button, details, input'))return;
        selection.select(file.id,entries.map(f=>f.id),{toggle:event.ctrlKey||event.metaKey,range:event.shiftKey});syncSelection();row.focus({preventScroll:true});
      });
      row.addEventListener('contextmenu',event=>{
        if(!row.fileMenu || loading || working(file.id))return;
        event.preventDefault();if(!selection.ids.has(file.id))selection.select(file.id,entries.map(f=>f.id));syncSelection();
        if(selection.ids.size>1){openSelectionMenu(event);return;}
        openFileMenu(row.fileMenu);
        row.fileMenu.querySelector('button')?.focus();
      });
      row.addEventListener('keydown',event=>{
        if(event.target!==row)return;
        const order=entries.map(f=>f.id), toggle=event.ctrlKey||event.metaKey;
        let target;
        if(['ArrowDown','ArrowUp','ArrowLeft','ArrowRight'].includes(event.key)) {
          event.preventDefault();target=selection.step(order,['ArrowUp','ArrowLeft'].includes(event.key)?-1:1,{range:event.shiftKey,focusOnly:toggle});
        } else if(event.key==='Home' || event.key==='End') {
          event.preventDefault();target=event.key==='Home'?order[0]:order.at(-1);selection.select(target,order,{range:event.shiftKey});
        } else if(event.key===' ') {event.preventDefault();selection.select(file.id,order,{toggle:true,range:event.shiftKey});}
        else if(toggle && event.key.toLowerCase()==='a') {event.preventDefault();selection.all(order);}
        else if(event.key==='Escape') {event.preventDefault();selection.clear();selection.focus=file.id;}
        else if(event.key==='Delete' || event.key==='Backspace') {if(selectionAllowed().trash && !selectionUnavailable()){event.preventDefault();trash(selectedFiles());}}
        else if(event.key==='Enter' && currentView!=='trash') {event.preventDefault();openFile(file);}
        else if(event.key==='ContextMenu' || (event.shiftKey && event.key==='F10')) {event.preventDefault();if(selection.ids.size>1)openSelectionMenu();else {openFileMenu(row.fileMenu);row.fileMenu?.querySelector('button')?.focus();}}
        syncSelection();if(target)fileRows.get(target)?.row.focus({preventScroll:true});
      });
      if(currentView!=='trash') {
        const star=button('',()=>setStars([file],!file.starred),'button file-star');star.textContent=file.starred?'★':'☆';star.setAttribute('aria-pressed',String(!!file.starred));
        bind(star,()=>{const label=t(file.starred?'Eemalda tärn':'Lisa tärn');star.setAttribute('aria-label',label);star.title=label;});row.append(star);
      }
      const date=element('span','file-date');bind(date,()=>{date.textContent=new Date(file.updatedMs).toLocaleDateString(window.SubnetI18n.locale,{dateStyle:'medium'});});
      if(currentView==='recent')bind(date,()=>{date.textContent=new Date(file.openedMs||file.updatedMs).toLocaleDateString(window.SubnetI18n.locale,{dateStyle:'medium'});});
      if(currentView==='home')bind(date,()=>{date.textContent=suggestionReason(file);});
      const owner=element('span','file-owner');owner.title=file.spaceAccess?.teamName||file.owner;
      bind(owner,()=>{owner.textContent=file.spaceAccess?.teamName || (file.owner===account?.principal?t('Mina'):`${file.owner.slice(0,6)}…${file.owner.slice(-4)}`);});
      const bytes=element('span','file-size');bind(bytes,()=>{bytes.textContent=file.folder||file.shortcut?'—':size(file.bytes||0);});row.append(owner,date,bytes);
    }
    if(picker && file.shortcut) {row.append(button('Ava',()=>openFile(file)));row.addEventListener('dblclick',()=>openFile(file));return row;}
    if(picker && !file.folder) {
      const supported=pickerSupports(file);
      row.classList.toggle('picker-unsupported',!supported);
      if(supported && picker.mode==='open') {
        row.tabIndex=0;row.setAttribute('aria-label',visibleName(file));
        row.addEventListener('click',()=>{pickerSelection=file;for(const item of $('file-list').children)item.classList.toggle('picker-selected',item.dataset.fileId===file.id);updatePicker();});
        row.addEventListener('dblclick',()=>selectPickerFile(file));
        row.addEventListener('keydown',event=>{if(event.key==='Enter')selectPickerFile(file);});
        row.classList.toggle('picker-selected',pickerSelection?.id===file.id);
        row.append(button('Ava',()=>selectPickerFile(file)));
        if(file.capabilities.edit)row.append(button('Versioonid',()=>history(file)));
      }
      if(!file.capabilities.edit) detail.append(element('p','picker-access','Ainult lugemiseks'));
      return row;
    }
    if (currentView === 'trash') {
      const actions=element('div','trash-actions');
      const restore=button('Taasta', () => organize(file, 'restore'));actions.append(restore,button('Taasta mujale…',()=>restoreElsewhere(file)));
      if(canManage(file,account.principal)) {
        const remove=button('Kustuta jäädavalt',()=>confirmPurge(file),'button button-danger');actions.append(remove);
      }
      row.append(actions);
    }
    else {
      const href = picker ? null : editorHref(file);
      if (href) {
        const link = element('a','button','Ava'); link.href = href;link.addEventListener('click',event=>{if(event.metaKey||event.ctrlKey||event.shiftKey)return;event.preventDefault();openFile(file);}); row.append(link);
      } else if(!file.folder) row.append(button('Ava',()=>openFile(file)));
      row.tabIndex=0;
      row.addEventListener('dblclick',event=>{if(!event.target.closest('a, button, details, input'))openFile(file);});

      if(picker) {return row;}
      const menu = element('details','file-menu'); const summary = element('summary','button'); summary.textContent = '···'; bind(summary, () => summary.setAttribute('aria-label', t('Failitoimingud'))); menu.append(summary);
      row.fileMenu=menu;summary.addEventListener('click',event=>{
        if(working(file.id)){event.preventDefault();return;}
        if(selection.ids.has(file.id) && selection.ids.size>1){event.preventDefault();event.stopPropagation();openSelectionMenu(event);return;}
        event.preventDefault();if(menu.open)menu.open=false;else openFileMenu(menu);
      });
      const actions = element('div','file-menu-items');
      const action = (label, fn) => actions.append(button(label, () => { if(loading || busy || working(file.id)) return; menu.open = false; fn(); }));
      if(file.folder)action('Laadi alla',()=>download(file));
      if (!file.folder && !file.shortcut) { if(previewFormat(file))action('Eelvaade',()=>openFile(file,{preview:true}));actions.append(button('Ava',()=>{menu.open=false;openFile(file);})); action('Laadi alla', () => download(file)); if(file.capabilities.edit) action('Versioonid', () => history(file)); }
      if (file.capabilities.edit) { action('Nimeta ümber', () => rename(file)); if (!file.folder && !file.shortcut) action('Laadi uus versioon', () => { replaceFile = file; $('replace-input').click(); }); }
      if (file.capabilities.edit && file.capabilities.delete) {action('Liiguta', () => move(file));action('Lõika',()=>rememberClipboard([file],true));}
      if(!file.folder && !file.shortcut && file.capabilities.read)action('Kopeeri',()=>rememberClipboard([file],false));
      if (canManage(file,account.principal)) action('Jaga', () => share(file));
      else {action('Eemalda enda vaatest',()=>people.hideItem(file));if(!file.spaceAccess?.teamName)action('Blokeeri omanik',()=>people.block(file.owner));action('Raporteeri jagamist',()=>people.report(file));}
      if(!file.shortcut)action('Lisa otsetee',()=>move(file,{shortcut:true}));
      if(file.folder)action('Kausta värv',()=>folderColor(file));
      action('Faili teave',()=>details.show(file));
      action('Kopeeri link',()=>copyLink(file));
      if(!file.folder && !file.shortcut && file.capabilities.read)action('Tee koopia',()=>move(file,{copy:true}));
      if(currentView==='home')action('Peida seitsmeks päevaks',()=>mutate(()=>call('dismiss',{id:file.id}),{ids:[file.id]}));
      if (file.capabilities.delete) action('Prügikasti', () => trash(file));
      if (actions.children.length) { menu.append(actions); row.append(menu); }
    }
  if(!picker) {
    // Grid rows contain cells; keep buttons and checkboxes as controls inside
    // cells rather than replacing their native accessibility roles.
    for(const child of [...row.children]) if(child.getAttribute('role')!=='gridcell') {
      const cell=element('div','file-control-cell');cell.setAttribute('role','gridcell');
      row.insertBefore(cell,child);cell.append(child);
    }
  }
  return row;
}
function renderFiles() {
  const list=$('file-list'), signatures=new Map(entries.map(file=>[file.id,JSON.stringify([file,currentView,picker?.mode])]));
  let focusId=null;
  for(const [id,{row,signature}] of fileRows) if(signatures.get(id)!==signature) {
    if(row.contains(document.activeElement))focusId=id;
    if(inlineRename?.row===row)cancelInlineRename();
    row.remove();fileRows.delete(id);
  }
  const sorted=entries; // Already globally ordered by the service, including across pages.
  for(const [index,file] of sorted.entries()) {
    const signature=signatures.get(file.id);
    let rendered=fileRows.get(file.id);
    if(!rendered) {rendered={row:createFileRow(file),signature};fileRows.set(file.id,rendered);}
    const {row}=rendered;
    if(list.children[index]!==row)list.insertBefore(row,list.children[index] || null);
    updateRowState(row,file.id);
    if(focusId===file.id){row.tabIndex=0;row.focus({preventScroll:true});}
  }
  fileMenus=[...fileRows.values()].map(({row})=>row.fileMenu).filter(Boolean);
  $('more-files').hidden = !cursor; $('file-status').hidden = entries.length > 0;
  text($('file-status'), cursor ? 'Sellel lehel vasteid ei ole. Vaata järgmisi tulemusi.' : currentView === 'trash' ? 'Prügikast on tühi.' : currentView === 'shared' && !folder ? 'Sinuga pole veel faile ega kaustu jagatud.' : currentView==='search' ? 'Otsingule vastavaid faile ei leitud.' : currentView==='home' ? 'Soovitusi veel pole. Lisa või ava faile, et neid siin näha.' : currentView==='recent' ? 'Hiljuti avatud faile veel pole.' : currentView==='starred' ? 'Tärniga faile veel pole.' : 'Kaust on tühi.');
  syncBrowseControls();syncSelection();thumbnails.update(entries,fileRows,preferences.view==='grid'&&!picker,signedSession());
}
function suggestionReason(file) {
  const source={updated:'Uuendatud pärast avamist',opened:'Hiljuti avatud',starred:'Tärniga',created:'Hiljuti loodud'}[file.reason];
  if(!source)return '';
  return t(source)+(file.reasonMs?' · '+new Date(file.reasonMs).toLocaleDateString(window.SubnetI18n.locale,{dateStyle:'medium'}):'');
}
async function readBrowsePage(query,cursor) {
  if(query.scope==='content')return call('contentQuery',{text:query.text,parent:query.parent,cursor});
  return query.scope==='suggested' ? call('suggestions') : call('query',{...query,cursor});
}
// Only the foreground consumer acknowledges a successful open. Failure to save
// this private summary does not turn a successful document open into an error.
function acknowledge(file) {
  const session=signedSession();
  return call('acknowledge',{id:file.id,revision:file.folder?null:file.revision}).then(()=>{
    if(currentSession(session))for(const key of views.keys())if(['suggested','recent'].includes(JSON.parse(key).scope))views.delete(key);
  }).catch(()=>{});
}
function fileUrl(file) {return new URL(driveHref(`#${file.folder?'folder':'file'}/${file.id}`),location.origin).href;}
async function copyLink(file) {
  const session=signedSession();
  const url=fileUrl(file);
  try {await navigator.clipboard.writeText(url);if(currentSession(session))message('Link kopeeritud. Lingi kopeerimine ligipääsu ei muuda.');}
  catch(error){if(currentSession(session))message(errorMessage(error),true);}
}
function renderHomeFolders(files) {
  homeFolders=files||[];
  const signature=JSON.stringify(files);if(signature===homeFolderSignature)return;homeFolderSignature=signature;
  const list=$('home-folder-list');list.replaceChildren();
  if(!files){list.append(element('p','muted','Kaustade laadimine ebaõnnestus. Proovi värskendada.'));return;}
  for(const file of files) {
    const link=element('a','home-folder');link.href=driveHref(`#folder/${file.id}`);
    const icon=element('span','folder-icon');icon.setAttribute('aria-hidden','true');if(file.color)icon.dataset.folderColor=file.color;
    const detail=element('span','home-folder-detail'),name=element('span');name.textContent=file.name;
    const reason=element('small');bind(reason,()=>{reason.textContent=suggestionReason(file);});detail.append(name,reason);link.append(icon,detail);list.append(link);
  }
  if(!files.length)list.append(element('p','muted','Sinu kaustad ilmuvad siia.'));
}
async function route(initialPage=null) {
  normalizeRoute();
  publicLinks.hide();
  cancelInlineRename();
  const epoch=++generation;++readSequence;refreshing=false;moreLoading=false;resetDrop();displayedRoute=null;pickerSelection=null;
  const retained=selectionRoute===currentRoute();
  if(!retained){selection.clear();selectionRoute=currentRoute();delete preferences.recentSort;delete preferences.recentDescending;}
  const hash=picker && ['', '#kodu','#rakendused','#prugikast','#ai'].includes(currentRoute())?'#failid':currentRoute();
  const parsed=browseRoute(hash,root?.id,account?.principal,preferences);currentView=parsed.view;activeQuery=parsed.query;
  const insight=new RegExp(`^#(tegevused|salvestusruum)(?:/(${OBJECT_UUID}))?$`,'i').exec(hash);if(insight&&!picker)currentView=insight[1]==='tegevused'?'activity':'storage';else insights.hide();
  const peopleView=!picker && (hash==='#teavitused'?'inbox':hash==='#kontaktid'?'contacts':hash==='#peidetud'?'safety':null);if(peopleView)currentView=peopleView;else people.hide();
  if(!retained && currentView!=='search')for(const id of ['search-text','search-type','search-owner','search-after','search-before','search-location'])$(id).value='';
  if(hash==='#tiimid')currentView='teams';else teams.hide();
  folder=null;trail=[];text($('view-status'),'');
  if(currentView==='teams'){loading=false;clearFiles('');heading();updateAccount();details.close(false);await teams.show();return;}
  const linked=/^#(?:file|folder)\/([^/]+)$/.exec(hash);
  if(linked && !picker && cloud && !account?.authenticated){currentView='public';loading=false;clearFiles('');heading();updateAccount();await publicLinks.show(linked[1]);return;}
  if(!account?.authenticated || !cloud){loading=false;clearFiles('Logi sisse, et näha oma salvestatud faile.');heading();updateAccount();return;}
  if(peopleView){loading=false;clearFiles('');heading();updateAccount();details.close(false);await people.show(peopleView);return;}
  if(insight&&!picker){loading=false;clearFiles('');heading();updateAccount();details.close(false);await insights.show(currentView,insight[2]);return;}
  const invitation=/^#invite\/(\d+)$/.exec(hash);
  if(invitation && !picker){loading=false;clearFiles('Kutse vastuvõtmiseks kinnita saaja e-post.');heading();updateAccount();accountEmail.invite(invitation[1]);return;}
  if(hash==='#email' && !picker)accountEmail.open();
  if(['apps','ai'].includes(currentView)){lastVisitedRoute=hash;loading=false;heading();updateAccount();return;}
  const key=queryKey(),cached=views.get(key);
  if(initialPage?.folder?.id===activeQuery.parent && activeQuery.scope==='folder' && !cloud.needs_refresh()){applyPage(initialPage,false);remember(key,initialPage);await restoreReturnState();loading=false;updateAccount();return;}
  if(cached && Date.now()-cached.at<30000 && !cloud.needs_refresh())applyPage(cached.value,false);
  else if(!retained){entries=[];cursor=null;renderFiles();text($('file-status'),'Laadin sinu faile…');}
  loading=true;heading();updateAccount();text($('view-status'),'Värskendan…');
  try {
    const revision=mutationRevision;
    const page=await readBrowsePage(activeQuery,null,initialPage);
    if(epoch!==generation || cloud.needs_refresh())return;
    if(revision!==mutationRevision){await route();return;}
    applyPage(page,false);remember(key,page);await restoreReturnState();lastRefresh=Date.now();
    if(lastVisitedRoute!==hash && hash.startsWith('#folder/') && page.folder)acknowledge(page.folder);
    lastVisitedRoute=hash;
  } catch(error) {
    if(epoch===generation){if(linked&&!picker){currentView='public';loading=false;clearFiles('');heading();updateAccount();await publicLinks.show(linked[1]);return;}clearCache();folder=null;trail=[];clearFiles('Kaust ei ole kättesaadav. Värskenda või ava Minu failid.');message(errorMessage(error),true);heading();const unavailable=/^#(?:file|folder)\/([^/]+)$/.exec(hash);if(unavailable&&!picker)$('file-status').append(button('Küsi ligipääsu',()=>people.requestAccess(unavailable[1])));}
  } finally {if(epoch===generation){loading=false;text($('view-status'),'');updateAccount();}}
}
function applyPage(page,append) {
  displayedRoute=currentRoute();loadedPages=append?loadedPages+1:1;
  folder=page.folder||null;trail=page.breadcrumbs||[];
  if(picker?.initialParent && folder?.id===picker.initialParent && folder.spaceId!==picker.initialSpace)throw new Error(t('Sihtkaust ei ole kättesaadav.'));

  if(folder && activeQuery.scope==='folder')currentView=folder.spaceAccess?.teamName || folder.owner===account.principal?'files':'shared';
  entries=append?[...entries,...page.entries]:page.entries;cursor=page.cursor;
  if(currentView==='home' && Object.hasOwn(page,'quickFolders'))renderHomeFolders(page.quickFolders);
  heading();renderFiles();
}
async function loadFiles(epoch, append = false) {
  if (!account?.authenticated) return;
  if(moreLoading || refreshing)return;
  const revision=mutationRevision, sequence=++readSequence;moreLoading = true; updateAccount();
  text($('more-files'),'Laadin…');
  try {
    const page = await readBrowsePage(activeQuery,append?cursor:null);
    if (epoch !== generation || sequence!==readSequence || cloud.needs_refresh()) return;
    if(revision!==mutationRevision){await refreshFiles({force:true});return;}
    applyPage(page,append);
  } catch(error) { if(epoch === generation && sequence===readSequence) { clearCache(); message(errorMessage(error),true);if((error?.message||String(error))==='This item changed; refresh and try again')await route(); } }
  finally { if(epoch === generation && sequence===readSequence) { moreLoading=false; text($('more-files'),'Näita rohkem');updateAccount(); } }
}
async function refresh(full = false, force = false) {
  if(cloud?.needs_refresh()) closePreview();
  if($('preview-dialog').open) return;
  if (!cloud || restoring || busy || $('drive-dialog').open) return;
  if(!full && account?.authenticated && !cloud.needs_refresh()) {
    if(loading || dragDepth || fileMenus.some(menu=>menu.open) || pickerSelection || ['apps','ai','activity','storage','inbox','contacts','safety','teams','public'].includes(currentView)) return;
    if(displayedRoute === currentRoute()) await refreshFiles({force}); else await route();
    return;
  }
  restoring=true; lastVisitedRoute=null; ++sessionVersion;fileClipboard=null;internalDrag=null;transfers.reset();operations.clear();resetNavigation(); clearCache(); ++generation; clearFiles('Kontrollin sisselogimist…'); updateAccount();
  try {
    const state=await cloud.restore(); account=state; root=null; folder=null;
    if(state.authenticated) {
      root=state.home?.folder || await call('initialize');
      preferences={...defaultPreferences(),...state.preferences};
    }
    message(); lastRefresh=Date.now(); await route(state.home);
  } catch(error) { account=null; clearCache(); clearFiles('Kontoga ühendamine ei õnnestunud. Proovi uuesti.'); message(errorMessage(error),true); }
  finally { restoring=false; updateAccount(); }
}
// Revalidate only visible metadata. Keep the current rows (and loaded pages)
// until the response is ready; unchanged results do not replace focused DOM.
async function refreshFiles({force=false} = {}) {
  if(currentView==='public')return;
  if(currentView==='teams'){if(force)await teams.show();return;}
  if(['activity','storage'].includes(currentView)){await insights.refresh();return;}
  if(['inbox','contacts','safety'].includes(currentView)){await people.refresh();return;}
  if((refreshing && !force) || loading || displayedRoute!==currentRoute() || ['apps','ai'].includes(currentView))return;
  const epoch=generation, sequence=++readSequence, revision=mutationRevision;
  const query={...activeQuery},pageCount=loadedPages;
  const current=()=>epoch===generation && sequence===readSequence && revision===mutationRevision && !cloud.needs_refresh();
  refreshing=true;moreLoading=false;text($('more-files'),'Näita rohkem');updateAccount();
  try {
    const pages=[];let next=null;
    for(let index=0;index<pageCount;index++) {
      const page=await readBrowsePage(query,next);
      if(!current())return;
      pages.push(page);next=page.cursor;if(!next)break;
    }
    const first=pages[0];let all=pages.flatMap(page=>page.entries);
    if(currentView==='home' && !force)all=stableSuggestionOrder(entries,all);
    if(currentView==='home' && Object.hasOwn(first,'quickFolders')) {
      if(!force)first.quickFolders=stableSuggestionOrder(homeFolders,first.quickFolders);
      renderHomeFolders(first.quickFolders);
    }
    const visible=all;
    if(JSON.stringify([folder,trail,entries,cursor])!==JSON.stringify([first.folder || null,first.breadcrumbs || [],visible,next])) applyPage({...first,entries:all,cursor:next},false);
    loadedPages=pages.length;remember(queryKey(),first);lastRefresh=Date.now();details.refresh();
  } catch(error) {
    if(current()) {
      clearCache();
      if((error?.message || String(error))==='Not found or access unavailable') {
        folder=null;trail=[];displayedRoute=null;clearFiles('Kaust ei ole kättesaadav. Värskenda või ava Minu failid.');heading();
      }
      message(errorMessage(error),true);
    }
  } finally {if(sequence===readSequence){refreshing=false;updateAccount();}}
}
async function refreshQuota() {
  const session=signedSession(), sequence=++quotaSequence;
  try {
    const state=await call('account');
    if(currentSession(session) && sequence===quotaSequence && state.principal===session.principal) {
      account={...account,usedBytes:state.usedBytes,quotaBytes:state.quotaBytes};updateAccount();
    }
  } catch(error) {if(currentSession(session))message(errorMessage(error),true);}
}
function showDialog(title, submit, action) {
  cancelInlineRename();
  ++dialogGeneration;
  text($('dialog-title'),title); $('dialog-body').replaceChildren(); $('dialog-error').hidden = true;
  $('dialog-submit').hidden = !submit; $('dialog-submit').disabled = false;
  $('dialog-submit').classList.toggle('button-danger',false);
  if (submit) text($('dialog-submit'),submit);
  modalAction = action; $('drive-dialog').showModal();
}
function field(label, value = '') {
  const wrapper = element('label','dialog-field',label); const input = element('input'); input.value = value; input.required = true; input.maxLength = 160; wrapper.append(input); $('dialog-body').append(wrapper); return input;
}
function closeDialog() { if (!busy) { ++dialogGeneration; $('drive-dialog').close(); } }
const accountEmail=createAccountEmail({
  call,element,button,field,dialog:showDialog,body:()=>$('dialog-body'),text,
  error:error=>{text($('dialog-error'),errorMessage(error));$('dialog-error').hidden=false;},
  current:()=>{const session=signedSession(),epoch=dialogGeneration;return ()=>currentSession(session) && epoch===dialogGeneration && $('drive-dialog').open;},
  invitationId:()=>/^#invite\/(\d+)$/.exec(currentRoute())?.[1],
  accepted:async id=>{const session=signedSession(),epoch=dialogGeneration;const file=await call('get',{id});if(!currentSession(session)||epoch!==dialogGeneration)return;closeDialog();location.hash=`#${file.folder?'folder':'file'}/${id}`;},
});
$('account-name').addEventListener('click',()=>{
  $('account-menu').open=false;
  const session=signedSession();
  let input;
  showDialog('Konto nimi','Salvesta',async()=>{
    try {
      const result=await call('setDisplayName',{name:input.value.trim()});
      if(currentSession(session) && epoch===dialogGeneration){
        account={...account,displayName:result.displayName};
        try {localStorage.setItem('subnet.profile.changed',`${account.principal}:${Date.now()}`);} catch {}
        updateAccount();closeDialog();
      }
    } catch(error) {if(currentSession(session) && epoch===dialogGeneration){text($('dialog-error'),errorMessage(error));$('dialog-error').hidden=false;}}
  });
  const epoch=dialogGeneration;
  input=field('Kuvatav nimi',account?.displayName || '');input.maxLength=80;input.autocomplete='name';
  $('dialog-body').append(element('p','muted','Seda nime kasutatakse Subneti rakendustes.'));input.focus();
});
$('account-email').addEventListener('click',()=>{$('account-menu').open=false;accountEmail.open();});
const publicLinks=createPublicLinks({canRequest:()=>!!account?.authenticated,requestAccess:id=>people.requestAccess(id),host:$('public-view'),body:$('public-body'),read:(op,p)=>cloud.public_read(op,JSON.stringify(p)),download:id=>cloud.public_download(id),call,element,button,text:t,error:errorMessage,field,requestId,mutate,size,href:file=>driveHref(`#${file.folder?'folder':'file'}/${file.id}`),preview:file=>openFile(file,{preview:true,publicRead:true})});
const teams=createTeamSpaces({host:$('teams-view'),body:$('teams-body'),call,element,button,text:t,bind,session:signedSession,current:currentSession,size,error:errorMessage,dialog:showDialog,dialogBody:()=>$('dialog-body'),field,requestId,mutate,navigate:id=>{location.hash=`#folder/${id}`;},changed:clearCache,dialogCurrent:()=>{const epoch=dialogGeneration;return ()=>$('drive-dialog').open&&epoch===dialogGeneration;}});
$('manage-space').addEventListener('click',()=>teams.manage(folder.spaceId));
const folderTree=createFolderTree({host:$('folder-tree'),toggle:$('folder-tree-toggle'),call,element,button,text:t,session:signedSession,current:currentSession,
  navigate:id=>{location.hash=`#folder/${id}`;},active:()=>folder?.id,error:errorMessage});
const details=createFileDetails({displayName:visibleName,typeName:file=>appFor(file)?.id==='wordcraft'?t('Tekstidokument'):null,host:$('file-details'),body:$('details-body'),closeButton:$('details-close'),title:$('details-title'),call,element,button,text:t,bind,role:entry=>({viewer:'Vaataja',editor:'Muutja',advanced:'Kohandatud ligipääs'})[sharingRole(entry.capabilities,entry.folder)],
  session:signedSession,current:currentSession,owner:()=>account?.principal,size,date:ms=>ms?new Date(ms).toLocaleString(window.SubnetI18n.locale):'—',error:errorMessage,history,share,activity:entry=>{location.hash=`#tegevused/${entry.id}`;}});
const thumbnails=createThumbnails({download:file=>cloud.download(file.id,file.revision),current:currentSession,element});
const people=createDrivePeople({host:$('people-view'),title:$('people-title'),body:$('people-body'),call,element,button,bind,text:t,session:signedSession,current:currentSession,error:errorMessage,showDialog,dialogBody:()=>$('dialog-body'),field,mutate,requestId,dialogEpoch:()=>dialogGeneration});
function driveHelp(){
  showDialog('Abi ja failivormingud',null);const body=$('dialog-body');body.append(element('h3','','Klaviatuuri otseteed'));
  const help=element('dl','shortcut-help');for(const [key,action] of [['Shift + klõps','Vali vahemik'],['Cmd / Ctrl + klõps','Lisa valikusse'],['Cmd / Ctrl + A','Vali kõik'],['Cmd / Ctrl + C / X / V','Kopeeri / lõika / kleebi'],['Cmd / Ctrl + Z','Võta viimane toiming tagasi'],['Enter','Ava'],['F2','Nimeta ümber'],['Delete','Prügikasti'],['Escape','Sulge või tühista valik']]){const term=element('dt','',key);help.append(term,element('dd','',action));}body.append(help,element('h3','','Pilvest avamine ja eelvaated'));
  const table=element('table','format-support'),head=element('tr');for(const label of ['Vorming','Eelvaade','Pilvest redigeerimine'])head.append(element('th','',label));table.append(head);
  for(const app of apps.filter(a=>a.cloud)){const row=element('tr'),name=element('td'),editing=element('td');name.textContent=app.cloudExtension;editing.textContent=app.name;row.append(name,element('td','',app.id==='pdfcraft'?'PDF eelvaade':'Ava redaktoris'),editing);table.append(row);}
  for(const [format,preview] of [['DOCX / XLSX / PPTX','Tekst ja salvestatud väärtused'],['PNG / JPEG / GIF / WebP / AVIF / SVG','Pildi eelvaade'],['TXT / CSV / JSON / HTML','Teksti eelvaade'],['MP3 / WAV / FLAC / MP4 / WebM','Brauseri meediamängija']]){const row=element('tr'),name=element('td');name.textContent=format;row.append(name,element('td','',preview),element('td','','Impordi redaktoris'));table.append(row);}body.append(table,element('p','muted','Muud vormingud ava rakenduse enda importimise menüüst. Eelvaade ei tõenda redaktori importimise ega eksportimise täielikku ühilduvust.'));
}
$('account-help').addEventListener('click',()=>{$('account-menu').open=false;driveHelp();});
$('account-safety').addEventListener('click',()=>{$('account-menu').open=false;location.hash='#peidetud';});
$('account-contacts').addEventListener('click',()=>{$('account-menu').open=false;location.hash='#kontaktid';});
const insights=createDriveInsights({host:$('insights-view'),title:$('insights-title'),body:$('insights-body'),call,element,button,bind,text:t,session:signedSession,current:currentSession,owner:()=>account?.principal,size,date:ms=>new Date(ms).toLocaleString(window.SubnetI18n.locale),error:errorMessage,history,open:async id=>{const session=signedSession();try{const entry=await call('get',{id});if(currentSession(session))location.hash=`#${entry.folder?'folder':'file'}/${id}`;}catch(error){if(currentSession(session))message(errorMessage(error),true);}}});
const revisionHistory=createRevisionHistory({body:$('dialog-body'),showDialog,dialogOpen:()=>$('drive-dialog').open,dialogEpoch:()=>dialogGeneration,call,element,button,bind,text:t,field,session:signedSession,current:currentSession,owner:()=>account?.principal,size,date:ms=>new Date(ms).toLocaleString(window.SubnetI18n.locale),error:errorMessage,mutate,requestId,download,picker:()=>picker,select:selectPickerFile,message,
  restore:async(file,revision,token)=>{const bytes=await cloud.download(file.id,revision.id);await cloud.upload(JSON.stringify({file:file.id,revision:file.revision,requestId:token}),bytes,()=>{});}});
const undo=new LastDriveAction();
let undoPreparing=false,undoRetry=null;
function resetNavigation(){people.hide();insights.hide();folderTree.reset();details.close(false);undo.clear();undoRetry=null;undoPreparing=false;updateUndo();}
function updateUndo(){
  $('undo-notice').hidden=!undo.items.length || !!picker || !account?.authenticated;
  text($('undo-label'),'Toiming tehtud. {count} üksust.',{count:undo.items.length});
  $('undo-action').disabled=undoPreparing || busy || loading || restoring || operations.size>0 || !!cloud?.needs_refresh();
}
async function trackedOrganize(file,request,group=request.requestId){
  const session=signedSession(),before={...file},receipt=await call('organize',request);
  if(currentSession(session)){undo.record(group,before,request,receipt);updateUndo();}return receipt;
}
function appendAccessImpact(body,impact){
  if(!impact?.affectedSubjects)return;
  const panel=element('div','move-impact'),summary=element('p');
  text(summary,'Toiming muudab {objects} üksuse ligipääsu. Mõjutatud kontosid: {people}.',{objects:impact.affectedObjects,people:impact.affectedSubjects});panel.append(summary);
  for(const change of impact.changes){const line=element('p');line.textContent=(change.subject==='2vxsx-fae'?t('Kõik lingiga'):change.subject)+' · '+t(change.gains && change.loses?'Õigused või nende kestus muutuvad':change.gains?'Ligipääs laieneb või pikeneb':'Ligipääs väheneb või lüheneb');panel.append(line);}
  body.append(panel);
}
async function undoLastAction(){
  if($('undo-action').disabled || !undo.items.length)return;
  if(undoRetry?.group===undo.group)return undoRetry.execute();
  const session=signedSession(),group=undo.group,items=[...undo.items].reverse(),previews=new Map(),requests=new Map();
  undoPreparing=true;updateUndo();
  try{
    for(const item of items){const preview=await recoveryPreview(call,item);if(!currentSession(session)||undo.group!==group)return;if(preview)previews.set(item.file.id,preview);}
    const batch=batchAction(items.map(item=>item.file),async(file,token)=>{
      if(undo.group!==group)throw new Error(t('Viimane toiming on muutunud. Ava tagasivõtmine uuesti.'));
      const item=items.find(item=>item.file.id===file.id);
      if(!requests.has(file.id)){
        const preview=await recoveryPreview(call,item);if(!currentSession(session))throw new TransferCancelled();
        if(preview && impactKey(preview)!==impactKey(previews.get(file.id)))throw new Error(t('Ligipääs või fail muutus. Ava liigutamine uuesti ja kontrolli mõju.'));
        requests.set(file.id,{id:file.id,version:file.version,requestId:token,...item.inverse,...(preview?{treeVersion:preview.treeVersion}:{})});
      }
      await call('organize',requests.get(file.id));if(currentSession(session) && undo.group===group){undo.complete(file.id);updateUndo();}
    },{label:'Võtan tagasi…',clearSelection:true});
    const execute=()=>{undoRetry={group,execute};return batch();};
    if([...previews.values()].some(p=>p.affectedSubjects)){
      showDialog('Võta toiming tagasi','Võta tagasi',execute);selectionNames(items.map(item=>item.file));
      for(const preview of previews.values())appendAccessImpact($('dialog-body'),preview);
    }else await execute();
  }catch(error){if(currentSession(session))message(errorMessage(error),true);}
  finally{if(currentSession(session)){undoPreparing=false;updateUndo();}}
}
$('undo-action').addEventListener('click',undoLastAction);
$('undo-dismiss').addEventListener('click',()=>{undo.clear();undoRetry=null;updateUndo();});
document.addEventListener('keydown',event=>{
  if((event.ctrlKey||event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase()==='z' && fileClipboardEvent(event) && undo.items.length){event.preventDefault();undoLastAction();}
});
function showDetails(){const files=selectedFiles();if(!selectionUnavailable() && currentView!=='trash' && files.length<=1 && (files[0]||folder))details.show(files[0]||folder);}
$('show-details').addEventListener('click',showDetails);$('selection-details').addEventListener('click',showDetails);

async function restoreElsewhere(file){
  const session=signedSession();let target=null,read=0,stack=[],input,choices;
  showDialog('Taasta mujale','Kontrolli taastamist',async()=>{
    if(!target || !currentSession(session))return;
    const parent=target.id,name=input.value,epoch=dialogGeneration;
    $('dialog-submit').disabled=true;
    try{
      const impact=await call('restorePreview',{id:file.id,parent,name});
      if(!currentSession(session)||epoch!==dialogGeneration)return;
      if(impact.version!==file.version)throw new Error('This item changed; refresh and try again');
      let request;
      showDialog('Kinnita taastamine','Taasta',()=>mutate(async()=>{
        if(!request){const fresh=await call('restorePreview',{id:file.id,parent,name});if(!currentSession(session))throw new TransferCancelled();
          if(impactKey(fresh)!==impactKey(impact))throw new Error(t('Ligipääs või fail muutus. Ava liigutamine uuesti ja kontrolli mõju.'));
          request={id:file.id,version:file.version,change:'restoreTo',parent,name,treeVersion:fresh.treeVersion,requestId:requestId()};}
        await call('organize',request);
      },{ids:[file.id],label:'Taastan…'}));
      const label=element('p');label.textContent=name;$('dialog-body').append(label,destinationOwner(target));
      const location=element('p');if(target.id===root?.id)text(location,'Minu failid');else location.textContent=target.name;$('dialog-body').append(location);appendAccessImpact($('dialog-body'),impact);
    }catch(error){if(currentSession(session)&&epoch===dialogGeneration){text($('dialog-error'),errorMessage(error));$('dialog-error').hidden=false;$('dialog-submit').disabled=false;}}
  });
  const epoch=dialogGeneration;input=field('Nimi',file.name);choices=element('div','restore-folders');$('dialog-body').append(choices);
  $('dialog-body').append(element('p','muted','Vali samas pilveruumis kaust. Olemasolevaid faile ei kirjutata üle.'));
  async function browse(destination,cursor=null){
    const sequence=++read;target=null;$('dialog-submit').disabled=true;
    if(!cursor)choices.replaceChildren(element('p','','Laadin…'));
    try{
      const page=destination?await call('query',{scope:'folder',parent:destination.id,fileType:'folder',sort:'name',cursor}):await call('list',{parent:null,cursor});
      if(!currentSession(session)||epoch!==dialogGeneration||sequence!==read)return;
      target=destination;if(!cursor){choices.replaceChildren();if(destination){choices.append(button('← Tagasi',()=>{stack.pop();browse(stack.at(-1));}));const label=element('p','picker-location');if(destination.id===root?.id)text(label,'Minu failid');else label.textContent=destination.name;choices.append(label);}}
      for(const entry of page.entries.filter(e=>e.folder && e.spaceId===file.spaceId && e.id!==file.id)){const choice=button('',()=>{stack.push(entry);browse(entry);},'folder-choice');choice.textContent=entry.name;choices.append(choice);}
      if(page.cursor){const more=button('Näita rohkem kaustu',()=>{more.remove();browse(destination,page.cursor);});choices.append(more);}
      $('dialog-submit').disabled=!target || !(target.capabilities.create || target.id===file.parent);
    }catch(error){if(currentSession(session)&&epoch===dialogGeneration&&sequence===read)choices.replaceChildren(element('p','is-error',errorMessage(error)));}
  }
  if(root?.spaceId===file.spaceId){stack=[root];await browse(root);}else await browse(null);
}
async function mutate(action, {ids=[], label='Salvestan…', success, quota=false} = {}) {
  const keys=ids.map(String);
  if(busy || loading || restoring || !account?.authenticated || cloud.needs_refresh() || keys.some(working))return;
  const session=signedSession(), epoch=dialogGeneration, undoRevision=undo.revision, hadDialog=$('drive-dialog').open, routeAtStart=currentRoute();
  const operation={label};for(const key of keys)operations.set(key,operation);
  ++mutationRevision;
  if(hadDialog)$('drive-dialog').close();
  updateAccount();
  let succeeded=false;
  try {
    await action();succeeded=true;
    if(currentSession(session) && undo.revision===undoRevision){undo.clear();undoRetry=null;}
  } catch(error) {
    if(currentSession(session)) {
      if(hadDialog && epoch===dialogGeneration && routeAtStart===currentRoute() && !$('drive-dialog').open) {
        $('drive-dialog').showModal();text($('dialog-error'),errorMessage(error));$('dialog-error').hidden=false;
      } else message(errorMessage(error),true);
    }
  } finally {
    if(currentSession(session)) {
      // Even a lost response or a failed concurrent action must reconcile the
      // current view. Reads started before this settlement cannot replace it.
      ++mutationRevision;clearCache();folderTree.refresh();
      await Promise.all([refreshFiles({force:true}), ...(quota ? [refreshQuota()] : [])]);
      if(succeeded && currentSession(session) && success)message(success);
    }
    for(const key of keys)if(operations.get(key)===operation)operations.delete(key);
    updateAccount();
  }
  return succeeded;
}

function newFolder() {
  const parent = creationFolder()?.id; if(!parent) return; const token = requestId(); let input;
  showDialog('Uus kaust','Loo kaust',() => mutate(() => call('folder',{parent,name:input.value,requestId:token}),{ids:[`children:${parent}`],label:'Loon kausta…'}));
  input = field('Kausta nimi'); input.focus();
}
function rename(file) {
  const token = requestId(); let input;
  showDialog('Nimeta ümber','Salvesta',() => mutate(() => trackedOrganize(file,{id:file.id,version:file.version,change:'rename',name:storedName(input.value,file),requestId:token}),{ids:[file.id],label:'Nimetan ümber…'}));
  input = field('Nimi',visibleName(file)); input.focus(); input.select();
}
function organize(file, change, extra = {}) { return mutate(() => call('organize',{id:file.id,version:file.version,change,requestId:requestId(),...extra}),{ids:[file.id],label:change==='restore'?'Taastan…':'Salvestan…'}); }
function trash(file) {
  const files=Array.isArray(file)?file:[file];
  const group=requestId();
  showDialog('Prügikasti','Prügikasti',batchAction(files,(file,requestId)=>trackedOrganize(file,{id:file.id,version:file.version,change:'trash',requestId},group),{roots:true,clearSelection:true,label:'Viin prügikasti…'}));
  selectionNames(files);$('dialog-body').append(element('p','muted','Faili või kausta saab hiljem prügikastist taastada.'));
}
async function confirmPurge(file = null) {
  if(busy || loading || restoring || picker || !account?.authenticated || cloud.needs_refresh() || (file && !canManage(file,account.principal)))return;
  const principal=account.principal, token=requestId();let items=null;
  showDialog(file ? 'Kustuta jäädavalt' : 'Tühjenda prügikast',file ? 'Kustuta jäädavalt' : 'Tühjenda prügikast',()=>{
    if(!items?.length || account?.principal!==principal || cloud.needs_refresh())return;
    return mutate(()=>call('purge',{items,requestId:token}),{ids:items.map(item=>item.id),label:'Kustutan…',success:'Jäädavalt kustutatud.',quota:true});
  });
  const dialogEpoch=dialogGeneration, body=$('dialog-body');
  $('dialog-submit').classList.toggle('button-danger',true);$('dialog-submit').disabled=true;
  if(file) {
    const name=element('p');name.textContent=file.name;body.append(name);
    items=[{id:file.id,version:file.version}];
  } else {
    body.append(element('p','','Laadin…'));
    try {
      const all=[];let next=null;
      do {
        const page=await call('trash',{cursor:next});
        if(dialogEpoch!==dialogGeneration || !$('drive-dialog').open || account?.principal!==principal || cloud.needs_refresh())return;
        all.push(...page.entries.filter(entry=>canManage(entry,principal)));next=page.cursor;
      } while(next);
      items=all.map(entry=>({id:entry.id,version:entry.version}));body.replaceChildren();
      if(!items.length) {body.append(element('p','','Sinu prügikastis pole kustutatavaid faile ega kaustu.'));$('dialog-submit').hidden=true;return;}
      body.append(element('p','','Kõik sinu prügikastis olevad failid ja kaustad kustutatakse jäädavalt. Teistele kuuluvad üksused jäävad alles.'));
    } catch(error) {
      if(dialogEpoch===dialogGeneration && $('drive-dialog').open) {text($('dialog-error'),errorMessage(error));$('dialog-error').hidden=false;body.replaceChildren();}
      return;
    }
  }
  body.append(element('p','muted','Kustutatakse ka kõik failiversioonid ja kaustade sisu. Seda toimingut ei saa tagasi võtta.'));
  $('dialog-submit').disabled=false;$('dialog-cancel').focus();
}
async function download(file, revision = file.revision) {
  return downloadFiles([file],revision);
}
async function downloadFiles(files,revision) {
  if(busy || files.some(file=>working(file.id)) || !account?.authenticated || cloud.needs_refresh())return;
  const archive=files.length>1 || files.some(file=>file.folder),tooLarge='Vali korraga vähem faile. ZIP-allalaadimise piir on 256 MB.';
  if(archive && files.reduce((sum,file)=>sum+Number(file.bytes||0),0)>=ARCHIVE_LIMIT){message(tooLarge,true);return;}
  const session=signedSession(), operation={label:'Laadin faili alla…'};
  for(const file of files)operations.set(file.id,operation);updateAccount();
  const failures=[],downloaded=[];let total=0;
  try {
    const check=()=>{if(!currentSession(session))throw new TransferCancelled();};
    const roots=await selectionRoots(files,id=>selectionAncestor(id,session));
    const planned=await walkFiles(roots.map(group=>group.file),(parent,cursor)=>call('list',{parent,cursor}),check);
    if(archive && planned.reduce((sum,file)=>sum+Number(file.bytes||0),0)>=ARCHIVE_LIMIT)throw new RangeError(tooLarge);
    for(const file of planned) {
      if(!currentSession(session))return;
      if(file.folder){downloaded.push({path:file.path,directory:true,bytes:new Uint8Array()});continue;}
      if(file.shortcut){failures.push(`${file.name}: ${t('Otsetee jäi arhiivist välja. Ava sihtfail, et selle sisu alla laadida.')}`);continue;}
      try {
        const bytes=await cloud.download(file.id,revision??file.revision);
        if(!currentSession(session))return;
        total+=bytes.byteLength;
        if(archive && total>=ARCHIVE_LIMIT){message(tooLarge,true);return;}
        downloaded.push({name:file.name,path:file.path,bytes});
      }catch(error){failures.push(`${file.name}: ${errorMessage(error)}`);}
    }
    if(currentSession(session) && downloaded.length) {
      const blob=archive?downloadArchive(downloaded):new Blob([downloaded[0].bytes],{type:'application/octet-stream'});
      const url=URL.createObjectURL(blob),link=element('a');link.href=url;link.download=archive?'Cloud Space.zip':files[0].name;
      document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),30000);
    }
    if(currentSession(session) && failures.length)message(failures.join('\n'),true);
  } catch(error){if(currentSession(session))message(error instanceof RangeError?tooLarge:errorMessage(error),true);}
  finally {for(const file of files)if(operations.get(file.id)===operation)operations.delete(file.id);updateAccount();}
}
// Rendering is deliberately restricted to inert image, media and plain-text
// contexts. Never put uploaded HTML/SVG markup into this document or an iframe.
function releasePreview() {
  const state=previewState;previewState=null;
  state?.pdf?.close();
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
async function openFile(file,{preview=false,publicRead=false}={}) {
  if(!publicRead && (busy || working(file.id) || !account?.authenticated || cloud.needs_refresh()))return;
  cancelInlineRename();closeFileMenus();
  if(file.shortcut){const session=signedSession(),epoch=generation;try{const target=await call('shortcutTarget',{id:file.id});if(currentSession(session)&&epoch===generation){if(picker&&!target.folder&&!pickerSupports(target)){message('See fail ei sobi valitud redaktorile.',true);return;}return openFile(target,{preview});}}catch(error){if(currentSession(session))message(errorMessage(error),true);}return;}
  if(picker && !file.folder) {selectPickerFile(file);return;}
  if(file.folder) {location.hash=`#folder/${file.id}`;return;}
  const session=signedSession(),epoch=generation;
  try {if(!publicRead)file=await call('get',{id:file.id});if((!publicRead&&!currentSession(session))||epoch!==generation)return;clearCache();}
  catch(error){if(currentSession(session))message(errorMessage(error),true);return;}
  const href=publicRead?null:editorHref(file);if(href && !preview) {rememberReturnState();location.href=href;return;}
  // Keep the dialog open while switching files: a queued close event must not clear the next preview.
  releasePreview();
  const state={file,principal:account?.principal,zoom:1,publicRead};previewState=state;
  const current=()=>previewState===state && (publicRead || account?.principal===state.principal && !cloud.needs_refresh());
  $('preview-name').textContent=visibleName(file);$('preview-meta').textContent=size(file.bytes||0);
  if(!$('preview-dialog').open)$('preview-dialog').showModal();
  const format=previewFormat(file),siblings=publicRead?[]:entries.filter(item=>!item.folder&&!item.shortcut),index=siblings.findIndex(item=>item.id===file.id);
  $('preview-previous').disabled=index<=0;$('preview-next').disabled=index<0||index>=siblings.length-1;
  state.neighbors=[siblings[index-1],siblings[index+1]];
  $('preview-zoom-in').hidden=$('preview-zoom-out').hidden=$('preview-fit').hidden=format?.kind!=='image';
  $('preview-open-with').hidden=!href;$('preview-open-with').href=href||'';

  if(!format) {previewNotice('Selle failitüübi eelvaade pole veel saadaval. Faili saad alla laadida.');return;}
  if(format.kind==='office' && file.bytes>20*1024*1024){previewNotice('Office’i eelvaate piir on 20 MB.');return;}
  if(format.kind==='text' && file.bytes>1024*1024) {previewNotice('Teksti eelvaade on saadaval kuni 1 MB failidele. Suurema faili saad alla laadida.');return;}
  previewNotice('Laadin eelvaadet…');$('preview-content').setAttribute('aria-busy','true');
  // At most one verified download in flight. Closing discards the result; queued
  // stale requests never start, even when users open/close files rapidly.
  previewQueue=previewQueue.then(async()=>{
    if(!current()) return;
    try {
      const bytes=await (publicRead?cloud.public_download(file.id):cloud.download(file.id,file.revision));
      if(!current())return;
      if(format.kind==='office') {const office=await renderOfficePreview(bytes,format.extension,{element,text:t});if(current()){$('preview-content').replaceChildren(office);$('preview-content').setAttribute('aria-busy','false');(!publicRead&&acknowledge(file));}return;}
      if(format.kind==='pdf') {
        if(new TextDecoder().decode(bytes.slice(0,5))!=='%PDF-')throw new Error(t('See fail ei ole loetav PDF.'));
        const {createPdfPreview}=await import('./drive-pdf.js');if(!current())return;
        state.pdf=createPdfPreview({host:$('preview-content'),element,button,text:t,current,
          ready:()=>(!publicRead&&acknowledge(file)),error:()=>previewNotice('PDF-i eelvaadet ei saa kuvada. Ava fail redaktoris või laadi alla.',true)});
        await state.pdf.open(bytes);return;
      }
      if(format.kind==='text') {
        if(bytes.length>1024*1024)throw new Error(t('Teksti eelvaade on saadaval kuni 1 MB failidele. Suurema faili saad alla laadida.'));
        const encoding=bytes[0]===255 && bytes[1]===254 ? 'utf-16le' : bytes[0]===254 && bytes[1]===255 ? 'utf-16be' : 'utf-8';
        let value;try {value=new TextDecoder(encoding,{fatal:true}).decode(bytes);} catch {throw new Error(t('Selle teksti kodeeringut ei saa eelvaates lugeda. Laadi fail alla.'));}
        const pre=element('pre','preview-text');pre.textContent=value || t('Fail on tühi.');pre.tabIndex=0;
        $('preview-content').replaceChildren(pre);$('preview-content').setAttribute('aria-busy','false');(!publicRead&&acknowledge(file));return;
      }
      state.url=URL.createObjectURL(new Blob([bytes],{type:format.mime}));
      const viewer=element(format.kind==='image'?'img':format.kind,`preview-${format.kind}`);if(format.kind==='image')state.image=viewer;
      const ready=()=>{if(current()){(!publicRead&&acknowledge(file));$('preview-content').setAttribute('aria-busy','false');$('preview-content').querySelector('.preview-notice')?.remove();}};
      viewer.addEventListener('error',()=>{if(current())previewNotice('Brauser ei saa seda faili kuvada või esitada. Laadi fail alla.',true);});
      if(format.kind==='image') {viewer.alt=file.name;viewer.addEventListener('load',ready);}
      else {state.media=viewer;viewer.controls=true;viewer.preload='metadata';viewer.playsInline=true;viewer.setAttribute('aria-label',file.name);viewer.addEventListener('loadedmetadata',ready);}
      viewer.src=state.url;$('preview-content').append(viewer);
    } catch(error) {if(current())previewNotice(errorMessage(error),true);}
  });
}
const transferRows=new Map();
let collisionOpen=false;
const transfers=new TransferQueue({changed:renderTransfers});
function renderTransfers() {
  const active=transfers.jobs.some(job=>['queued','running'].includes(job.status)),changed=uploading!==active;
  uploading=active;$('transfer-status').hidden=!transfers.jobs.length;
  const completed=transfers.jobs.filter(job=>job.status==='done').length;
  text($('transfer-label'),active?'Ülekanded':'Ülekannete tulemused');
  text($('transfer-detail'),'{current} / {count} valmis',{current:completed,count:transfers.jobs.length});
  $('transfer-progress').hidden=true;$('transfer-clear').disabled=active;
  for(const [job,row] of transferRows)if(!transfers.jobs.includes(job)){row.remove();transferRows.delete(job);}
  for(const job of transfers.jobs) {
    let row=transferRows.get(job);
    if(!row) {
      row=element('div','transfer-row');const name=element('span','transfer-name');name.textContent=job.name;
      row.state=element('small');row.retry=button('Proovi uuesti',()=>transfers.retry(job));
      row.cancel=button('Tühista',()=>{transfers.cancel(job);if(collisionOpen)closeDialog();});
      row.append(name,row.state,row.retry,row.cancel);$('transfer-items').append(row);transferRows.set(job,row);
    }
    const labels={queued:'Ootel',running:'Ülekanne: {percent}%',done:'Valmis',skipped:'Vahele jäetud',cancelled:'Tühistatud',failed:'Ebaõnnestus: {error}'};
    text(row.state,labels[job.status],{percent:Math.round(job.progress*100),error:errorMessage(job.error||'')});
    row.retry.hidden=job.status!=='failed';row.retry.disabled=!job.valid();row.cancel.hidden=!['queued','running'].includes(job.status);
  }
  if(changed)updateAccount();
}
function uploadFormat(file) {
  const app=registry.find(app=>app.cloudExtension && file.name.toLowerCase().endsWith('.'+app.cloudExtension)),format=app?.openFormats?.[0];
  return {mediaType:file.type||'application/octet-stream',craft:format?{format_id:format.id,format_version:format.version,created_by_app:app.id}:null};
}
function collisionChoice(item,existing) {
  return new Promise(resolve=>{
    showDialog('Sama nimega üksus on juba olemas',null);collisionOpen=true;
    const closed=()=>{collisionOpen=false;resolve('skip');};$('drive-dialog').addEventListener('close',closed,{once:true});
    const choose=value=>{$('drive-dialog').removeEventListener('close',closed);collisionOpen=false;closeDialog();resolve(value);};
    const name=element('p');name.textContent=item.path.join('/');$('dialog-body').append(name);
    $('dialog-body').append(element('p','muted','Valik kehtib sellele üksusele. Juba lõpetatud ülekanded säilivad.'));
    if(item.folder && existing.folder)$('dialog-body').append(button('Ühenda kaustad',()=>choose('merge')));
    else if(!item.folder && !existing.folder && existing.capabilities.edit)$('dialog-body').append(button('Asenda',()=>choose('replace')));
    $('dialog-body').append(button('Säilita mõlemad',()=>choose('keep')),button('Jäta vahele',()=>choose('skip')));
  });
}
async function transferFinished(session) {
  if(!currentSession(session))return;
  ++mutationRevision;clearCache();await Promise.all([refreshFiles({force:true}),refreshQuota()]);
}
async function enqueueUploads(items,replacement,parent=creationFolder()?.id,session=signedSession()) {
  if(!currentSession(session) || !parent)return;
  const api={id:requestId,valid:()=>currentSession(session),format:file=>file.cloudFormat||uploadFormat(file),collision:collisionChoice,
    list:(parent,cursor)=>call('list',{parent,cursor}),folder:payload=>call('folder',payload),
    upload:(payload,bytes,progress)=>cloud.upload(JSON.stringify(payload),bytes,progress)};
  try {
    const jobs=replacement?items.map(item=>{
      const payload={file:replacement.id,revision:replacement.revision,requestId:requestId()};
      return {name:item.file.name,valid:api.valid,run:async control=>{
        const bytes=new Uint8Array(await item.file.arrayBuffer());control.check();
        await api.upload(payload,bytes,control.progress);
      }};
    }):uploadJobs(items,parent,api);
    for(const job of jobs){const run=job.run;job.run=async control=>{try{return await run(control);}finally{await transferFinished(session);}};}
    await transfers.add(jobs);
  } catch(error){if(currentSession(session))message(errorMessage(error),true);}
}
async function uploadFiles(files,replacement) {
  if(!files.length || busy || loading || restoring || picker || currentView==='trash' || !account?.authenticated || cloud?.needs_refresh())return;
  if(replacement?!replacement.capabilities.edit || working(replacement.id):!canUploadFiles())return;
  try {return await enqueueUploads(localFiles(files),replacement);}catch(error){message(errorMessage(error),true);}
}
async function history(file) {return revisionHistory.show(file);}

function rememberClipboard(files,cut,event) {
  const allowed=selectionCapabilities(files,account?.principal,currentView==='trash');
  if(!files.length || (cut?!allowed.move:!allowed.copy) || selectionUnavailable())return;
  const clipboardText=files.map(fileUrl).join('\n');
  fileClipboard={files:files.map(file=>({...file})),cut,session:signedSession(),clipboardText};syncSelection();
  if(event?.clipboardData){event.clipboardData.setData('text/plain',clipboardText);event.preventDefault();}
  else navigator.clipboard.writeText(clipboardText).catch(()=>{}); // Menu paste remains available without clipboard permission.
  message(cut?'Valik lõigatud. Ava sihtkaust ja kleebi.':'Valik kopeeritud. Ava sihtkaust ja kleebi.');
}
function canPaste() {
  return !!fileClipboard && currentSession(fileClipboard.session) && !!creationFolder()?.capabilities.create && ['files','shared'].includes(currentView) && !loading && !busy && !picker;
}
function pasteFiles() {
  if(!canPaste())return;
  const clipboard=fileClipboard;
  move(clipboard.files,{copy:!clipboard.cut,destination:creationFolder()});
}
function connectInternalDrag(row,file) {
  row.draggable=currentView!=='trash';
  row.addEventListener('dragstart',event=>{
    if(event.target.closest('button,input,a') || selectionUnavailable()) {event.preventDefault();return;}
    if(!selection.ids.has(file.id)){selection.select(file.id,entries.map(file=>file.id));syncSelection();}
    const files=selectedFiles();if(!selectionCapabilities(files,account?.principal).move){event.preventDefault();return;}
    internalDrag={files,session:signedSession()};event.dataTransfer.effectAllowed='move';
    event.dataTransfer.setData('application/x-subnet-drive','move');
  });
  if(!file.folder)return;
  const allowed=()=>internalDrag && currentSession(internalDrag.session) && file.capabilities.create && !internalDrag.files.some(item=>item.id===file.id) && internalDrag.files.every(item=>item.spaceId===file.spaceId);
  row.addEventListener('dragover',event=>{if(!allowed())return;event.preventDefault();event.stopPropagation();event.dataTransfer.dropEffect='move';row.classList.add('drop-folder');});
  row.addEventListener('dragleave',()=>row.classList.remove('drop-folder'));
  row.addEventListener('drop',event=>{
    row.classList.remove('drop-folder');if(!allowed())return;event.preventDefault();event.stopPropagation();
    const files=internalDrag.files;internalDrag=null;move(files,{destination:file});
  });
}
const moveImpactKey=impactKey;
function destinationOwner(destination) {
  const node=element('p','muted');
  if(destination.spaceAccess?.teamName){text(node,'Sihtkoht: {name}',{name:destination.spaceAccess.teamName});return node;}
  text(node,destination.owner===account?.principal?'Sihtkausta omanik: mina':'Sihtkausta omanik: {owner}',{owner:destination.owner});return node;
}
function folderColor(file) {
  showDialog('Kausta värv',null);const colors=[[null,'Vaikimisi'],['blue','Sinine'],['gold','Kuldne'],['green','Roheline'],['purple','Lilla'],['rose','Roosa'],['gray','Hall']];
  $('dialog-body').append(element('p','muted','Värv muutub ainult sinu kaustavaates.'));
  const palette=element('div','folder-palette');
  for(const [color,label] of colors){const b=button(label,()=>mutate(()=>call('folderColor',{id:file.id,color}),{ids:[file.id]}));if(color)b.dataset.folderColor=color;b.setAttribute('aria-pressed',String((file.color||null)===color));palette.append(b);}
  $('dialog-body').append(palette);
}
async function move(file,{copy=false,shortcut=false,destination:initialDestination=null}={}) {
  const files=Array.isArray(file)?file:[file],selected=new Set(files.map(file=>file.id)),session=signedSession(),undoGroup=requestId();
  let target=null,stack=initialDestination?[initialDestination]:(copy||shortcut)?[root]:[],read=0,execute,copyName,previews=new Map(),moveRequests=new Map();
  showDialog(shortcut?'Lisa otsetee':copy?'Tee koopia':'Liiguta',shortcut?'Lisa siia':copy?'Kopeeri siia':'Liiguta siia',()=>{
    if(!target || !currentSession(session))return;
    // Lock the destination for retries after a partially successful operation.
    if(!execute){
      const parent=target.id,name=copyName?.value.trim();
      if((copy||shortcut) && files.length===1 && (!name || new TextEncoder().encode(name).length>160 || /[\/\\]/.test(name))) {
        text($('dialog-error'),'Sisesta sobiv faili nimi (kuni 160 baiti).');$('dialog-error').hidden=false;return;
      }
      if(shortcut){execute=batchAction(files,(file,requestId)=>call('shortcut',{parent,target:file.id,name:files.length===1?name:file.name,requestId}));copyName.disabled=true;return execute();}
      if(copy) {
        const items=files.map(file=>({path:[files.length===1?name:file.name],folder:false,file:{name:file.name,size:file.bytes,
          cloudFormat:{mediaType:file.format?.media_type||'application/octet-stream',craft:file.format?.craft||null},
          arrayBuffer:async()=>{if(!currentSession(session))throw new TransferCancelled();return cloud.download(file.id,file.revision);}}}));
        closeDialog();return enqueueUploads(items,null,parent,session);
      }
      execute=batchAction(files,async(file,requestId)=>{
        if(moveRequests.has(file.id))return trackedOrganize(file,moveRequests.get(file.id),undoGroup);
        const expected=previews.get(file.id),fresh=await call('movePreview',{id:file.id,parent});
        if(!currentSession(session))throw new TransferCancelled();
        if(!expected || moveImpactKey(fresh)!==moveImpactKey(expected))throw new Error(t('Ligipääs või fail muutus. Ava liigutamine uuesti ja kontrolli mõju.'));
        const request={id:file.id,version:fresh.version,change:'move',parent,treeVersion:fresh.treeVersion,requestId};moveRequests.set(file.id,request);
        const result=await trackedOrganize(file,request,undoGroup);
        if(fileClipboard?.cut) {fileClipboard.files=fileClipboard.files.filter(item=>item.id!==file.id);if(!fileClipboard.files.length)fileClipboard=null;}
        return result;
      },{roots:true,clearSelection:true,label:'Liigutan…'});
      if(copyName)copyName.disabled=true;
    }
    return execute();
  });
  const dialogEpoch = dialogGeneration;
  if((copy||shortcut) && files.length===1){copyName=field(shortcut?'Otsetee nimi':'Koopia nimi',shortcut?files[0].name:t('Koopia failist {name}',{name:files[0].name}));}
  const nameField=copyName?.parentElement;
  async function browse(destination) {
    if(execute)return;
    const sequence=++read;target=null;$('dialog-submit').disabled=true;
    const body = $('dialog-body'); body.replaceChildren(element('p','','Laadin…'));
    try {
      let page, items = [], next = null;
      do {
        if(!currentSession(session))return;
        page=await call('list',{parent:destination?.id || null,cursor:next});items.push(...page.entries);next=page.cursor;
      } while(next);
      if(!$('drive-dialog').open || dialogEpoch !== dialogGeneration || sequence!==read || !currentSession(session))return;
      target=destination;$('dialog-submit').disabled=!target?.capabilities.create || (!shortcut && selected.has(target?.id)) || (!copy && !shortcut && files.every(file=>file.parent===target?.id));
      body.replaceChildren();selectionNames(files);
      if(nameField)body.append(nameField);if(shortcut)body.append(element('p','muted','Otsetee ei anna sihtfailile ligipääsu. Selle kustutamine ei kustuta algset faili.'));
      if(copy)body.append(element('p','muted','Koopia on uus fail sihtkausta õigustega. Algse faili jagamisi ei kopeerita.'));
      if(destination)body.append(destinationOwner(destination));
      if(destination) { const name = element('p','picker-location'); if(destination.id === root?.id) text(name,'Minu failid'); else name.textContent = destination.name; body.append(button('← Tagasi',()=>{stack.pop(); browse(stack.at(-1));}),name); }
      for(const item of items.filter(e=>e.folder && (copy || shortcut || e.spaceId === files[0].spaceId) && (shortcut || !selected.has(e.id)))) { const choice=button('',()=>{stack.push(item); browse(item);},'folder-choice'); if(item.id === root?.id) text(choice,'Minu failid'); else choice.textContent=item.name; body.append(choice); }
      if(!body.children.length) body.append(element('p','','Sobivaid kaustu ei ole.'));
      if(!copy && !shortcut && target && !$('dialog-submit').disabled) {
        $('dialog-submit').disabled=true;previews=new Map();
        const roots=await selectionRoots(files,id=>selectionAncestor(id,session));
        for(const {file} of roots) {
          const impact=await call('movePreview',{id:file.id,parent:destination.id});
          if(dialogEpoch!==dialogGeneration || sequence!==read || !currentSession(session))return;
          previews.set(file.id,impact);
          if(impact.affectedSubjects) {
            const panel=element('div','move-impact');
            const summary=element('p');panel.append(summary);
            text(summary,'Liigutamine muudab {objects} üksuse ligipääsu. Mõjutatud kontosid: {people}.',{objects:impact.affectedObjects,people:impact.affectedSubjects});
            for(const change of impact.changes) {
              const line=element('p');line.textContent=(change.subject==='2vxsx-fae'?t('Kõik lingiga'):change.subject)+' · '+t(change.gains && change.loses?'Õigused või nende kestus muutuvad':change.gains?'Ligipääs laieneb või pikeneb':'Ligipääs väheneb või lüheneb');panel.append(line);
            }
            body.append(panel);
          }
        }
        $('dialog-submit').disabled=false;
      }
    } catch(error) {if(dialogEpoch===dialogGeneration && sequence===read && currentSession(session))body.replaceChildren(element('p','is-error',errorMessage(error)));}
  }
  await browse(initialDestination||((copy||shortcut)?root:null));
}
function roleSelect(value,onChange) {
  const select=element('select','sharing-role');
  bind(select,()=>select.setAttribute('aria-label',t('Ligipääsu roll')));
  // Existing capability grants remain exact until the owner explicitly replaces them.
  if(value==='advanced'){const custom=element('option','','Kohandatud ligipääs');custom.value='advanced';custom.disabled=true;select.append(custom);}
  for(const [role,label] of [['viewer','Vaataja'],['editor','Muutja']]) {
    const option=element('option','',label);option.value=role;select.append(option);
  }
  select.value=value;select.addEventListener('change',()=>onChange(select.value));return select;
}
async function share(file) {
  const files=Array.isArray(file)?file:[file],session=signedSession();
  let principal,execute,role,expiry;
  showDialog('Jaga','Valmis',()=>{
    if(!currentSession(session))return;
    if(!execute) {
      const recipients=[...new Set(principal.value.trim().split(/[\s,;]+/).filter(Boolean))];
      if(!recipients.length){closeDialog();return;}
      if(recipients.length>20 || recipients.length*files.length>200){text($('dialog-error'),'Vali kuni 20 saajat ja kuni 200 jagamist korraga.');$('dialog-error').hidden=false;return;}
      const expiresAt=expiry.value?String(BigInt(new Date(expiry.value).getTime())*1000000n):null;
      if(expiresAt && Number(expiresAt)/1e6<=Date.now()){text($('dialog-error'),'Ligipääsu lõpp peab olema tulevikus.');$('dialog-error').hidden=false;return;}
      const capabilities=roleCapabilities(role.value,true),chosenRole=role.value;
      const jobs=new Map(files.map(file=>[file.id,recipients.map(subject=>({subject,requestId:requestId(),done:false}))]));
      execute=batchAction(files,async file=>{
        const failures=[];
        for(const job of jobs.get(file.id)){
          if(job.done)continue;if(!currentSession(session))throw new TransferCancelled();
          try {
            if(job.subject.includes('@')) {
              const result=await call('emailShare',{id:file.id,email:job.subject,role:chosenRole,expiresAt,requestId:job.requestId});
              if(currentSession(session) && result.delivery!=='Accepted')message('Jagamine salvestati, kuid kirja saatmine vajab kontrollimist. Ava jagamise vaade.',true);
            } else await call('grant',{root:file.id,subject:job.subject,requestId:job.requestId,expiresAt,capabilities:{...capabilities,create:file.folder && capabilities.create}});
            job.done=true;
          }catch(error){failures.push(`${job.subject}: ${errorMessage(error)}`);}
        }
        if(failures.length)throw new Error(failures.join('\n'));
      });
      principal.disabled=true;role.disabled=true;expiry.disabled=true;
    }
    return execute();
  });
  const dialogEpoch = dialogGeneration;
  selectionNames(files);
  principal = field('Saaja e-post või konto ID'); principal.maxLength = 5080;principal.required=false;principal.addEventListener('input',()=>text($('dialog-submit'),principal.value.trim()?'Jaga':'Valmis'));
  $('dialog-body').append(element('p','muted','Mitu saajat eralda koma või tühikuga.'));
  const contacts=element('datalist');contacts.id='sharing-contacts';principal.setAttribute('list',contacts.id);$('dialog-body').append(contacts);
  call('contacts').then(items=>{if(dialogEpoch!==dialogGeneration||!currentSession(session))return;for(const item of items){const option=element('option');option.value=item.principal;option.textContent=item.label;contacts.append(option);}}).catch(()=>{});
  role=roleSelect('viewer',()=>{});
  const sharingHelp=element('details','sharing-help');sharingHelp.append(element('summary','','Jagamise selgitus'),element('p','muted','Vaataja saab lugeda. Muutja saab muuta ja prügikasti teisaldada ning kausta faile lisada. Jagamist ja lõplikku kustutamist haldab omanik.'));
  $('dialog-body').append(role,sharingHelp);
  expiry=field('Ligipääs kuni');expiry.type='datetime-local';expiry.required=false;
  const expiryDetails=element('details','share-expiry'),expirySummary=element('summary','','Lisa lõppkuupäev');
  expiryDetails.append(expirySummary,expiry.parentElement);$('dialog-body').append(expiryDetails);
  expiry.addEventListener('change',()=>text(expirySummary,expiry.value?'Ligipääs kuni':'Lisa lõppkuupäev'));
  sharingHelp.append(element('p','muted','E-postiga jagamiseks kinnita esmalt enda e-post konto menüüs. Uus saaja saab kutse ja kinnitab oma aadressi Subnetis.'));
  if(files.length===1)$('dialog-body').append(button('Kopeeri link',()=>copyLink(files[0])));
  if(files.length===1){const section=element('section','sharing-public');$('dialog-body').append(section);publicLinks.mountSettings(files[0],section,()=>dialogEpoch===dialogGeneration && currentSession(session) && $('drive-dialog').open);}
  const roster = element('div','share-roster'); $('dialog-body').append(element('h3','','Ligipääsuga inimesed'),roster);
  if(files.length===1){const owner=element('div','share-row');owner.append(element('span','',files[0].spaceAccess?.teamName || (files[0].owner===account.principal?'Mina':files[0].owner)),element('strong','','Omanik'));roster.append(owner);}
  if(files.length>1)return;
  file=files[0];
  try {
    const grants = await call('grants',{id:file.id}); if(!$('drive-dialog').open || dialogEpoch !== dialogGeneration || !currentSession(session)) return;
    for(const grant of grants.filter(g=>g.active && (!g.expiresAt || Number(g.expiresAt)/1e6>Date.now()))) {
      const row = element('div','share-row'); const who = element('code','account-id'); who.textContent = grant.subject; row.append(who);
      const role=roleSelect(sharingRole(grant.capabilities,file.folder),value=>{
        if(value==='advanced')return;
        const requestIdForChange=requestId();
        mutate(()=>call('grant',{...grant,capabilities:roleCapabilities(value,file.folder),requestId:requestIdForChange}),{ids:[file.id]});
      });
      // Preserve custom grants; selecting a preset is an explicit replacement.
      role.querySelector('option[value="advanced"]')?.setAttribute('disabled','');
      role.disabled=grant.root!==file.id;row.append(role);
      if(grant.root === file.id) {
        row.append(button('Muuda aegumist',()=>{
          let input,request;const token=requestId();showDialog('Ligipääs kuni','Salvesta',()=>{if(!request)request={...grant,expiresAt:input.value?String(BigInt(new Date(input.value).getTime())*1000000n):null,requestId:token};input.disabled=true;return mutate(()=>call('grant',request),{ids:[file.id]});});
          input=field('Ligipääs kuni');input.type='datetime-local';input.required=false;
          if(grant.expiresAt){const d=new Date(Number(grant.expiresAt)/1e6);input.value=new Date(d.getTime()-d.getTimezoneOffset()*60000).toISOString().slice(0,16);}
          $('dialog-body').append(element('p','muted','Tühi kuupäev tähendab tähtajatut ligipääsu.'));
        }));
        row.append(button('Saada märguanne',()=>mutate(()=>call('notify',{id:file.id,recipient:grant.subject}),{ids:[file.id]})));
        const revokeId=requestId();row.append(button('Eemalda ligipääs',()=>mutate(()=>call('grant',{...grant,requestId:revokeId,active:false}),{ids:[file.id]})));
      } else row.append(element('span','muted','Päritud kaustast'));
      if(grant.expiresAt){const until=element('small','muted');bind(until,()=>{until.textContent=t('Ligipääs kuni')+' '+new Date(Number(grant.expiresAt)/1e6).toLocaleString(window.SubnetI18n.locale);});row.append(until);}
      roster.append(row);
    }
    const invitations=await call('emailInvitations',{id:file.id});
    if(dialogEpoch!==dialogGeneration || !currentSession(session) || !$('drive-dialog').open)return;
    for(const invitation of invitations.filter(i=>i.active && (i.accepted || Number(i.expiresAt)/1e6>Date.now()))) {
      const row=element('div','share-row'),address=element('span');address.textContent=invitation.email;
      row.append(address,element('span','muted',invitation.accepted?'Kutse on vastu võetud':'Kutse ootab vastuvõtmist'));
      if(invitation.delivery!=='Accepted')row.append(element('span','muted','Kirja saatmine vajab kontrollimist'));
      if(['Unknown','Retryable','Sending','Queued'].includes(invitation.delivery))row.append(button('Proovi saatmist uuesti',()=>mutate(()=>call('emailRetry',{id:invitation.id}),{ids:[file.id]})));
      if(!invitation.accepted)row.append(button('Tühista kutse',()=>mutate(()=>call('emailRevoke',{id:invitation.id}),{ids:[file.id]})));
      roster.append(row);
    }
  } catch(error) { if(dialogEpoch===dialogGeneration && currentSession(session))roster.append(element('p','is-error',errorMessage(error))); }
}
$('dialog-form').addEventListener('submit', async event => { event.preventDefault(); if(!busy && !$('dialog-submit').disabled && modalAction) { const epoch=dialogGeneration; $('dialog-submit').disabled = true; await modalAction(); if(epoch===dialogGeneration)$('dialog-submit').disabled = false; } });
$('dialog-close').addEventListener('click',closeDialog); $('dialog-cancel').addEventListener('click',closeDialog);
$('drive-dialog').addEventListener('cancel',event=>{if(busy)event.preventDefault();});
$('drive-dialog').addEventListener('close',updateAccount);
$('new-folder').addEventListener('click',newFolder);
$('new-menu-folder').addEventListener('click',()=>{if($('new-menu-folder').disabled)return;$('new-menu').open=false;newFolder();});
$('transfer-clear').addEventListener('click',()=>transfers.clear());
$('new-menu-upload-folder').addEventListener('click',()=>{if(!canUploadFiles())return;$('new-menu').open=false;$('folder-upload-input').click();});
$('folder-upload-input').addEventListener('change',event=>{uploadFiles(Array.from(event.target.files||[]));event.target.value='';});
$('new-menu-upload').addEventListener('click',()=>{if(!canUploadFiles())return;$('new-menu').open=false;$('upload-input').click();});
document.addEventListener('click',event=>{if(!$('new-menu').contains(event.target))$('new-menu').open=false;});
$('empty-trash').addEventListener('click',()=>confirmPurge());
function canUploadFiles() {
  return !!(cloud && account?.authenticated && creationFolder()?.capabilities.create && currentView!=='trash'
    && !picker && !busy && !loading && !restoring && !cloud.needs_refresh() && !$('drive-dialog').open && !$('preview-dialog').open);
}
const canDropFiles=()=>['files','shared'].includes(currentView) && canUploadFiles();
function resetDrop() {dragDepth=0;$('file-drop-overlay').hidden=true;}
function isFileDrag(event) {return Array.from(event.dataTransfer?.types || []).includes('Files');}
const fileArea=$('file-area');
fileArea.addEventListener('dragenter',event=>{
  if(!isFileDrag(event))return;
  event.preventDefault();event.stopPropagation();
  if(!canDropFiles())return;
  dragDepth++;$('file-drop-target').textContent=folder.id===root?.id ? t('Minu failid') : folder.name;
  $('file-drop-overlay').hidden=false;
});
fileArea.addEventListener('dragover',event=>{
  if(!isFileDrag(event))return;
  event.preventDefault();event.stopPropagation();event.dataTransfer.dropEffect=canDropFiles() ? 'copy' : 'none';
});
fileArea.addEventListener('dragleave',event=>{
  if(!isFileDrag(event))return;
  event.stopPropagation();if(--dragDepth<=0)resetDrop();
});
fileArea.addEventListener('drop',event=>{
  if(!isFileDrag(event))return;
  event.preventDefault();event.stopPropagation();resetDrop();
  if(!canDropFiles())return;
  const parent=creationFolder()?.id,session=signedSession();
  droppedFiles(event.dataTransfer).then(items=>enqueueUploads(items,null,parent,session)).catch(error=>message(errorMessage(error),true));
});
// Prevent the browser from navigating away if a file misses the content area.
for(const type of ['dragover','drop']) document.addEventListener(type,event=>{
  if(!isFileDrag(event))return;
  event.preventDefault();event.dataTransfer.dropEffect='none';if(type==='drop')resetDrop();
});
document.addEventListener('dragend',()=>{internalDrag=null;resetDrop();for(const {row} of fileRows.values())row.classList.remove('drop-folder');});
window.addEventListener('blur',resetDrop);
$('upload-file').addEventListener('click',()=> $('upload-input').click());
$('upload-input').addEventListener('change',event=>{const files=[...event.target.files];event.target.value='';uploadFiles(files);});
$('replace-input').addEventListener('change',event=>{const files=[...event.target.files];event.target.value='';uploadFiles(files,replaceFile);});
async function copyAccount() {
  if(!account?.authenticated || restoring || cloud?.needs_refresh()) return;
  const principal=account.principal;
  try {await navigator.clipboard.writeText(principal);message('Konto ID kopeeritud.');}catch {message(principal);}
}
function search() {
  if(!account?.authenticated)return;
  $('date-filter').open=false;
  const params=new URLSearchParams();
  for(const [key,id] of [['q','search-text'],['type','search-type'],['owner','search-owner'],['after','search-after'],['before','search-before']]) {
    const value=($(id).value||'').trim();if(value)params.set(key,value);
  }
  if($('search-location').value==='current' && (folder||activeQuery?.parent))params.set('in',folder?.id||activeQuery.parent);
  const content=$('search-mode').value==='content';if(content){for(const key of ['type','owner','after','before'])params.delete(key);}
  const hash=(content?'#content?':'#search?')+params.toString();if(currentRoute()===hash)route();else location.hash=hash;
}
$('drive-search').addEventListener('submit',event=>{event.preventDefault();search();});
$('search-mode').addEventListener('change',()=>{if($('search-text').value.trim())search();else syncBrowseControls();});
for(const id of ['search-type','search-owner','search-location'])$(id).addEventListener('change',search);
$('search-clear').addEventListener('click',()=>{for(const id of ['search-text','search-type','search-owner','search-location','search-after','search-before'])$(id).value='';location.hash=folder?`#folder/${folder.id}`:'#failid';});
function openSelectionMenu(event) {
  closeFileMenus();const menu=$('selection-more'),context=Number.isFinite(event?.clientX);
  menu.classList.toggle('is-context',context);
  if(context)menu.setAttribute('style',`--menu-x:${Math.max(8,Math.min(event.clientX,window.innerWidth-230))}px;--menu-y:${Math.max(8,Math.min(event.clientY,window.innerHeight-410))}px`);
  menu.open=true;menu.querySelector('button:not(:disabled)')?.focus({preventScroll:true});
}
$('clipboard-cut').addEventListener('click',()=>{rememberClipboard(selectedFiles(),true);$('selection-more').open=false;});
$('clipboard-copy').addEventListener('click',()=>{rememberClipboard(selectedFiles(),false);$('selection-more').open=false;});
$('clipboard-paste').addEventListener('click',pasteFiles);
// Browser clipboard events also cover macOS Edit-menu commands. Plain-text
// links are inert; only this account's in-memory references can trigger a paste.
function fileClipboardEvent(event) {
  return !picker && !$('drive-dialog').open && !$('preview-dialog').open && !event.target.closest?.('input,textarea,select,[contenteditable="true"]');
}
for(const action of ['copy','cut'])document.addEventListener(action,event=>{
  if(fileClipboardEvent(event) && selectedFiles().length && !selectionUnavailable())rememberClipboard(selectedFiles(),action==='cut',event);
});
document.addEventListener('paste',event=>{
  if(!fileClipboardEvent(event) || !canPaste() || event.clipboardData?.getData('text/plain')!==fileClipboard.clipboardText)return;
  event.preventDefault();pasteFiles();
});
$('selection-more').querySelector('summary')?.addEventListener('click',()=>{closeFileMenus();$('selection-more').classList.remove('is-context');});
$('select-all-files').addEventListener('click',()=>{selection.all(entries.map(file=>file.id));$('selection-more').open=false;syncSelection();});
$('selection-clear').addEventListener('click',()=>{const focus=selection.focus;selection.clear();selection.focus=focus;syncSelection();fileRows.get(focus)?.row.focus({preventScroll:true});});
const selectedActions={
  open:files=>openFile(files[0]),rename:files=>rename(files[0]),
  link:files=>copyLink(files[0]),copy:files=>move(files,{copy:true}),
  share:files=>share(files),move:files=>move(files),trash:files=>trash(files),
  download:files=>downloadFiles(files),star:files=>setStars(files,true),unstar:files=>setStars(files,false),
  restore:files=>batchAction(files,(file,requestId)=>call('organize',{id:file.id,version:file.version,change:'restore',requestId}),{label:'Taastan…',clearSelection:true})(),
};
for(const [name,action] of Object.entries(selectedActions))for(const id of ['selection-'+name,'selection-menu-'+name]) {
  $(id)?.addEventListener('click',()=>{
    if(selectionUnavailable() || !selectionAllowed()[name])return;
    $('selection-more').open=false;action(selectedFiles());
  });
}
document.addEventListener('click',event=>{if(!$('selection-more').contains(event.target))$('selection-more').open=false;});
$('selection-more').addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();$('selection-more').open=false;fileRows.get(selection.focus)?.row.focus({preventScroll:true});}});
async function changeSort() {
  if(!account?.authenticated)return;
  preferences.sort=$('sort-field').value;
  if(currentView==='recent'){preferences.recentSort=preferences.sort;preferences.recentDescending=preferences.descending;}
  const locationAtStart=currentRoute();if(await savePreferences() && currentRoute()===locationAtStart)await route();
}
$('sort-field').addEventListener('change',changeSort);
function toggleSort() {preferences.descending=!(currentView==='recent'?(preferences.recentDescending??true):preferences.descending);changeSort();}
$('sort-direction').addEventListener('click',toggleSort);
for(const field of ['name','modified','size'])$('column-'+field).addEventListener('click',()=>{
  const next=field==='modified' && currentView==='recent'?'opened':field;
  if($('sort-field').value===next)toggleSort();
  else {$('sort-field').value=next;preferences.descending=next!=='name';changeSort();}
});
for(const view of ['list','grid'])$('view-'+view).addEventListener('click',async()=>{
  if(!account?.authenticated)return;preferences.view=view;syncBrowseControls();
  // A layout change does not require changed metadata to start/stop previews.
  thumbnails.update(entries,fileRows,view==='grid'&&!picker,signedSession());
  const locationAtStart=currentRoute();
  if(await savePreferences() && currentRoute()===locationAtStart) {
    const desired=browseRoute(currentRoute(),root?.id,account?.principal,preferences).query;
    if(JSON.stringify(desired)!==queryKey())await route();else await refreshFiles({force:true});
  }
});
$('copy-account').addEventListener('click',copyAccount);
$('ai-copy-account').addEventListener('click',copyAccount);
async function signIn() {
  if(!cloud)return; const attempt=++loginAttempt;
  try {message('Lõpeta sisselogimine Internet Identity aknas.');text($('sign-in'),'Ava sisselogimine uuesti');await cloud.sign_in();if(attempt===loginAttempt)await refresh();}
  catch(error){if(attempt===loginAttempt)message(errorMessage(error),true);}finally{if(attempt===loginAttempt)text($('sign-in'),'Logi sisse');}
}
$('sign-in').addEventListener('click',signIn);
$('files-sign-in').addEventListener('click',signIn);
$('sign-out').addEventListener('click',async()=>{closePreview();++sessionVersion;fileClipboard=null;internalDrag=null;transfers.reset();operations.clear();resetNavigation();++generation;clearCache();account=null;root=null;folder=null;clearFiles('Login välja…');updateAccount();try{await cloud.sign_out();}catch(error){message(errorMessage(error),true);}await refresh();});
$('refresh-files').addEventListener('click',()=>{closeFileMenus();pickerSelection=null;if(!refreshing)refresh(false,true);});
$('more-files').addEventListener('click',()=>{if(!loading)loadFiles(generation,true);});
for(const [id,direction] of [['preview-previous',0],['preview-next',1]])$(id).addEventListener('click',()=>{const next=previewState?.neighbors?.[direction];if(next)openFile(next,{preview:true});});
function zoomPreview(change){const state=previewState;if(!state?.image)return;state.zoom=change===0?1:Math.max(.25,Math.min(4,state.zoom+change));state.image.style.maxWidth=state.zoom===1?'100%':'none';state.image.style.width=state.zoom===1?'auto':`${state.image.naturalWidth*state.zoom}px`;state.image.style.maxHeight=state.zoom===1?'65vh':'none';}
$('preview-zoom-in').addEventListener('click',()=>zoomPreview(.25));$('preview-zoom-out').addEventListener('click',()=>zoomPreview(-.25));$('preview-fit').addEventListener('click',()=>zoomPreview(0));
$('preview-dialog').addEventListener('keydown',e=>{
  // Media controls and horizontally scrollable content own their arrow keys.
  // Modified keys also belong to the browser/assistive technology.
  if(e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey ||
    e.target.closest?.('audio,video,iframe,input,textarea,select,[contenteditable="true"],.preview-text,.office-preview,.pdf-preview'))return;
  const control=e.key==='ArrowLeft'?'preview-previous':e.key==='ArrowRight'?'preview-next':null;
  if(control && !$(control).disabled){e.preventDefault();$(control).click();}
});
$('preview-close').addEventListener('click',closePreview);
document.addEventListener('click',event=>{for(const menu of fileMenus)if(!menu.contains(event.target))menu.open=false;});
document.addEventListener('scroll',event=>{if(!event.target.closest?.('.file-menu-items'))closeFileMenus();},true);
window.addEventListener('resize',()=>closeFileMenus());
document.addEventListener('keydown',event=>{if(event.key==='Escape'){cancelInlineRename();resetDrop();closeFileMenus();$('new-menu').open=false;$('date-filter').open=false;if(picker && !$('drive-dialog').open)postSelection({action:'cancel'});}});
$('preview-dialog').addEventListener('close',()=>{if(!$('preview-dialog').open)releasePreview();updateAccount();});
$('preview-dialog').addEventListener('cancel',event=>{event.preventDefault();closePreview();});
$('home-filter-toggle').addEventListener('click',()=>{const expanded=$('failid').classList.toggle('show-filters');$('home-filter-toggle').setAttribute('aria-expanded',String(expanded));});
$('preview-download').addEventListener('click',()=>{if(previewState)(previewState.publicRead?publicLinks.save(previewState.file):download(previewState.file));});
window.addEventListener('hashchange',()=>{closePreview();route();});
window.addEventListener('popstate',()=>{closePreview();route();});
document.addEventListener('click',event=>{
  if(event.defaultPrevented || event.button!==0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)return;
  const link=event.target.closest?.('a[href]');if(!link || link.target || link.hasAttribute('download'))return;
  const url=new URL(link.href,location.href);if(url.origin!==location.origin || url.hash)return;
  const next=driveRoute(url);if(!next)return;
  event.preventDefault();window.history.pushState(null,'',url.pathname+url.search);closePreview();route();
});
window.addEventListener('storage',event=>{
  if(event.key==='subnet.files.changed' && account?.authenticated && !cloud?.needs_refresh()) {
    ++mutationRevision;clearCache();refreshFiles({force:true});refreshQuota();return;
  }
  if(!event.key || ['ic-identity','ic-delegation'].includes(event.key) || cloud?.needs_refresh()) {
    if(cloud?.needs_refresh()) {
      closePreview();++sessionVersion;fileClipboard=null;internalDrag=null;transfers.reset();operations.clear();resetNavigation();++generation;clearCache();account=null;root=null;folder=null;
      ++dialogGeneration;$('drive-dialog').close();clearFiles('Kontrollin sisselogimist…');
    }
    refresh(true);
  }
});
let foregroundRefresh;
function scheduleForegroundRefresh() {
  clearTimeout(foregroundRefresh);
  // Let the click that focuses this window open its dialog or start its action
  // before starting a background metadata request.
  if(!document.hidden && Date.now()-lastRefresh>5000) foregroundRefresh=setTimeout(()=>refresh(),200);
}
window.addEventListener('focus',scheduleForegroundRefresh);
document.addEventListener('visibilitychange',()=>{resetDrop();scheduleForegroundRefresh();});
setInterval(()=>{if(!document.hidden && cloud?.needs_refresh())refresh();},1000);
setInterval(()=>{if(!document.hidden && !loading && account?.authenticated)refresh();},30000);
window.addEventListener('beforeunload',event=>{if(busy || uploading || operations.size){event.preventDefault();event.returnValue='';}});
route();
try {
  const response=await fetch('/apps.json'); if(!response.ok)throw new Error(t('Rakenduste register ei ole kättesaadav.'));
  registry=(await response.json()).apps;
  for(const app of registry.filter(app=>app.cloud)) {
    const option=element('option');option.value=app.id;option.textContent=app.name;option.setAttribute('data-no-translate','');$('search-type').append(option);
    const link=element('a','button');text(link,creationLabels[app.id]||app.name);link.title=app.name;link.dataset.appId=app.id;link.addEventListener('click',event=>{if(link.getAttribute('aria-disabled')==='true'){event.preventDefault();return;}rememberReturnState();});link.href=localizedHref(`/apps/${app.id}/`);$('new-apps').append(link);
  }
  setupPicker(); await init({module_or_path:'__CLOUD_WASM__'});cloud=new CloudPortal();await refresh();
}catch(error){text($('account-status'),'Konto pole kättesaadav');message(errorMessage(error),true);}
