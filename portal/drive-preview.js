// Derived, disposable browser previews. No content or preview authority is stored here.
export function previewFormat(file) {
  const ext=file.name.split('.').pop().toLowerCase(),mime=(file.format?.media_type||'').split(';')[0].toLowerCase();
  if(ext==='pdf'||mime==='application/pdf')return {kind:'pdf',mime:'application/pdf'};
  if(file.format?.craft)return null;
  if(['docx','xlsx','pptx'].includes(ext))return {kind:'office',extension:ext};
  const types={png:'image/png',jpg:'image/jpeg',jpeg:'image/jpeg',gif:'image/gif',webp:'image/webp',avif:'image/avif',bmp:'image/bmp',svg:'image/svg+xml',mp3:'audio/mpeg',wav:'audio/wav',ogg:'audio/ogg',oga:'audio/ogg',m4a:'audio/mp4',flac:'audio/flac',aac:'audio/aac',opus:'audio/ogg',mp4:'video/mp4',m4v:'video/mp4',webm:'video/webm',ogv:'video/ogg',mov:'video/quicktime'};
  const media=Object.values(types).includes(mime)?mime:types[ext];if(media)return {kind:media.split('/')[0],mime:media};
  if(mime.startsWith('text/')||/^application\/(json|xml|javascript|yaml|toml)$/.test(mime)||['txt','md','csv','tsv','json','xml','html','htm','css','js','ts','yaml','yml','toml','log','rs','py','sh'].includes(ext))return {kind:'text',mime:'text/plain'};
  return null;
}
export async function readOfficeParts(bytes,extension) {
  if(bytes.length>20*1024*1024)throw new Error('Office’i eelvaate piir on 20 MB.');
  const {unzipSync}=await import('./vendor/fflate/browser.js');
  let total=0,count=0;const names=new Set();
  const allowed=name=>extension==='docx'?name==='word/document.xml':extension==='pptx'?['ppt/presentation.xml','ppt/_rels/presentation.xml.rels'].includes(name)||/^ppt\/slides\/slide\d+\.xml$/.test(name):['xl/sharedStrings.xml','xl/workbook.xml','xl/_rels/workbook.xml.rels'].includes(name)||/^xl\/worksheets\/sheet\d+\.xml$/.test(name);
  const parts=unzipSync(bytes,{filter(file){
    if(!allowed(file.name))return false;
    if(names.has(file.name)||!Number.isSafeInteger(file.originalSize)||file.originalSize>2*1024*1024||++count>100||(total+=file.originalSize)>10*1024*1024)throw new Error('Dokument on eelvaate jaoks liiga keerukas.');
    names.add(file.name);return true;
  }});
  if(!Object.keys(parts).length)throw new Error('Dokumendist ei leitud eelvaate sisu.');
  return Object.entries(parts).sort(([a],[b])=>a.localeCompare(b,'en',{numeric:true})).map(([name,data])=>{
    const xml=new TextDecoder('utf-8',{fatal:true}).decode(data);
    if(/<!DOCTYPE|<!ENTITY/i.test(xml))throw new Error('Dokumendi XML ei sobi eelvaateks.');
    return {name,xml};
  });
}
export async function renderOfficePreview(bytes,extension,{element,text}) {
  const parts=await readOfficeParts(bytes,extension),root=element('div','office-preview');
  root.append(element('p','muted','Lihtsustatud eelvaade: tekst ja salvestatud väärtused. Küljendus, pildid ning valemite arvutamine ei ole kaasatud.'));
  const parse=xml=>{const doc=new DOMParser().parseFromString(xml,'application/xml');if(doc.getElementsByTagName('parsererror').length)throw new Error('Dokumendi XML ei sobi eelvaateks.');return doc;};
  const nodes=(doc,name)=>Array.from(doc.getElementsByTagNameNS('*',name));
  const strings=parts.find(p=>p.name==='xl/sharedStrings.xml');const shared=strings?nodes(parse(strings.xml),'si').slice(0,20000).map(node=>nodes(node,'t').map(n=>n.textContent).join('')):[];
  const primary=parts.find(p=>p.name===(extension==='xlsx'?'xl/workbook.xml':'ppt/presentation.xml'));
  const relations=parts.find(p=>p.name===(extension==='xlsx'?'xl/_rels/workbook.xml.rels':'ppt/_rels/presentation.xml.rels'));
  let content=parts.filter(p=>extension==='docx'?p.name==='word/document.xml':extension==='xlsx'?/^xl\/worksheets\//.test(p.name):/^ppt\/slides\//.test(p.name));
  const titles=new Map();
  if(primary&&relations){
    const base=extension==='xlsx'?'xl/':'ppt/';
    const links=new Map(nodes(parse(relations.xml),'Relationship').filter(n=>n.getAttribute('TargetMode')!=='External').map(n=>[n.getAttribute('Id'),n.getAttribute('Target')||'']));
    const order=nodes(parse(primary.xml),extension==='xlsx'?'sheet':'sldId').map(n=>{
      const target=links.get(n.getAttribute('r:id')||n.getAttributeNS('http://schemas.openxmlformats.org/officeDocument/2006/relationships','id'));
      if(!target)return null;const name=target.startsWith('/')?target.slice(1):base+target.replace(/^\.\//,'');
      if(extension==='xlsx')titles.set(name,n.getAttribute('name')||'');return content.find(p=>p.name===name);
    }).filter(Boolean);
    if(order.length)content=order;
  }
  let paragraphs=0,cells=0,truncated=false,partIndex=0;
  for(const part of content){
    const doc=parse(part.xml),section=element('section');partIndex++;
    if(extension!=='docx'){const heading=element('h3');heading.textContent=titles.get(part.name)||text(extension==='xlsx'?'Leht {number}':'Slaid {number}',{number:partIndex});section.append(heading);}
    if(extension==='xlsx'){
      const table=element('table','preview-table'),rows=nodes(doc,'row');truncated ||= rows.length>200;
      const heading=element('tr');heading.append(element('th'));for(let col=1;col<=30;col++){const th=element('th');th.textContent=col<=26?String.fromCharCode(64+col):'A'+String.fromCharCode(38+col);heading.append(th);}table.append(heading);
      for(const [index,row] of rows.slice(0,200).entries()){
        const tr=element('tr'),label=element('th');label.textContent=row.getAttribute('r')||String(index+1);tr.append(label);
        const byColumn=new Map();let sequential=0;
        for(const cell of nodes(row,'c')){
          const letters=cell.getAttribute('r')?.match(/^[A-Z]+/)?.[0];
          const column=letters?[...letters].reduce((n,c)=>n*26+c.charCodeAt(0)-64,0):++sequential;
          if(column>30){truncated=true;continue;}byColumn.set(column,cell);
        }
        for(let col=1;col<=30;col++){
          if(++cells>6000){truncated=true;break;}const td=element('td'),cell=byColumn.get(col);
          if(cell){td.setAttribute('title',cell.getAttribute('r')||'');const value=nodes(cell,'v')[0]?.textContent||'';td.textContent=cell.getAttribute('t')==='s'?(shared[Number(value)]||''):cell.getAttribute('t')==='inlineStr'?nodes(cell,'t').map(t=>t.textContent).join(''):value;}tr.append(td);
        }table.append(tr);if(cells>=6000)break;
      }section.append(table);
    } else {
      for(const p of nodes(doc,'p')){if(++paragraphs>2000)break;const out=element('p');out.textContent=nodes(p,'t').map(t=>t.textContent).join('');section.append(out);}
    }
    root.append(section);if(cells>=6000||paragraphs>=2000)break;
  }
  if(truncated||cells>=6000||paragraphs>=2000)root.append(element('p','muted','Kuvatud on dokumendi algus. Kogu sisu jaoks laadi fail alla.'));
  return root;
}

export function createThumbnails({download,current,element}) {
  const records=new Map();let active=0,queue=[];
  async function pump(){
    while(active<2&&queue.length){const record=queue.shift();if(record.cancelled)continue;active++;
      (async()=>{try{
        const bytes=await download(record.file);if(record.cancelled||!current(record.session))return;
        record.url=URL.createObjectURL(new Blob([bytes],{type:previewFormat(record.file).mime}));
        const img=element('img','file-thumbnail');img.alt='';img.loading='lazy';img.src=record.url;record.node.append(img);
      }catch{/* Keep the file-type icon when a thumbnail cannot be produced. */}finally{active--;pump();}})();
    }
  }
  function remove(key,record){record.cancelled=true;record.observer?.disconnect();record.node.querySelector('.file-thumbnail')?.remove();if(record.url)URL.revokeObjectURL(record.url);records.delete(key);}
  return {
    update(files,rows,grid,session){
      const wanted=new Set();
      if(grid&&typeof IntersectionObserver!=='undefined')for(const file of files.filter(f=>!f.folder&&!f.shortcut&&previewFormat(f)?.kind==='image'&&f.bytes<=8*1024*1024).slice(0,16)){
        const node=rows.get(file.id)?.row.querySelector('.file-icon');if(!node)continue;
        const key=`${session.principal||''}/${file.id}/${file.revision}`;wanted.add(key);
        if(records.has(key)&&records.get(key).node===node)continue;
        if(records.has(key))remove(key,records.get(key));
        const record={file,node,session,cancelled:false,url:null};records.set(key,record);
        record.observer=new IntersectionObserver(items=>{if(items.some(i=>i.isIntersecting)){record.observer.disconnect();queue.push(record);pump();}});record.observer.observe(node);
      }
      for(const [key,record] of records)if(!wanted.has(key))remove(key,record);
    },
    clear(){for(const [key,record] of records)remove(key,record);queue=[];},
  };
}
