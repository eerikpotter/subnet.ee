// Disposable display only: one page/canvas at a time, no PDF actions or external URLs.
export function createPdfPreview({host,element,button,text,current,ready,error,
  load=()=>import('./vendor/pdfjs/build/pdf.mjs')}) {
  let closed=false,loading,document,page,rendering,number=1,zoom=1,acknowledged=false;
  const alive=()=>!closed&&current();
  const root=element('div','pdf-preview'),toolbar=element('div','pdf-controls');
  toolbar.setAttribute('role','group');toolbar.setAttribute('aria-label',text('PDF-i leheküljed'));
  const status=element('span');status.setAttribute('role','status');
  const back=button('Eelmine lehekülg',()=>draw(number-1)),next=button('Järgmine lehekülg',()=>draw(number+1));
  const smaller=button('Vähenda',()=>{zoom=Math.max(.25,zoom-.25);draw(number);});
  const larger=button('Suurenda',()=>{zoom=Math.min(4,zoom+.25);draw(number);});
  const fit=button('Mahuta',()=>{zoom=1;draw(number);});
  const surface=element('div','pdf-page'),transcript=element('details','pdf-transcript');
  transcript.append(element('summary','','Lehekülje tekst'));
  const content=element('pre','preview-text');transcript.append(content);
  toolbar.append(back,status,next,smaller,larger,fit);root.append(toolbar,surface,transcript);
  const controls=[back,next,smaller,larger,fit];
  function disable(value){for(const control of controls)control.disabled=value;}
  async function draw(target){
    if(!alive()||!document||rendering||target<1||target>document.numPages)return;
    disable(true);host.setAttribute('aria-busy','true');surface.replaceChildren();content.textContent='';
    page?.cleanup();page=null;
    try {
      const selected=await document.getPage(target);if(!alive()){selected.cleanup();return;}page=selected;number=target;
      const base=page.getViewport({scale:1}),width=Math.max(200,Math.min(host.clientWidth-32||900,1200));
      const scale=Math.min(width/base.width*zoom,Math.sqrt(4_000_000/(base.width*base.height)));
      const viewport=page.getViewport({scale}),canvas=element('canvas','pdf-canvas');
      canvas.width=Math.ceil(viewport.width);canvas.height=Math.ceil(viewport.height);
      canvas.setAttribute('role','img');canvas.setAttribute('aria-label',text('Lehekülg {page} / {total}',{page:number,total:document.numPages}));
      const context=canvas.getContext('2d');if(!context)throw new Error(text('PDF-i eelvaadet ei saa kuvada. Ava fail redaktoris või laadi alla.'));
      const task=page.render({canvasContext:context,viewport});rendering=task;await task.promise;rendering=null;
      if(!alive())return;surface.append(canvas);
      status.textContent=text('Lehekülg {page} / {total}',{page:number,total:document.numPages});
      const words=await page.getTextContent();if(!alive())return;
      content.textContent=words.items.slice(0,5000).map(item=>item.str||'').join(' ').slice(0,200000);
      if(!acknowledged){acknowledged=true;ready();}
    }catch(failure){if(alive())error(failure);}
    finally{rendering=null;if(alive()){disable(false);back.disabled=number<=1;next.disabled=number>=document.numPages;smaller.disabled=zoom<=.25;larger.disabled=zoom>=4;host.setAttribute('aria-busy','false');}}
  }
  return {
    async open(bytes){
      const pdf=await load();if(!alive())return;
      pdf.GlobalWorkerOptions.workerSrc='/vendor/pdfjs/build/pdf.worker.mjs';
      loading=pdf.getDocument({data:bytes,isEvalSupported:false,useSystemFonts:false,
        cMapUrl:'/vendor/pdfjs/cmaps/',cMapPacked:true,standardFontDataUrl:'/vendor/pdfjs/standard_fonts/',
        wasmUrl:'/vendor/pdfjs/wasm/',maxImageSize:16_000_000,canvasMaxAreaInBytes:16_000_000});
      document=await loading.promise;if(!alive())return;
      host.replaceChildren(root);await draw(1);
    },
    close(){closed=true;rendering?.cancel();loading?.destroy().catch(()=>{});surface.replaceChildren();content.textContent='';},
  };
}
