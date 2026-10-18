// ZIP STORE: retain original bytes, UTF-8 names and CRC32, without a compressor.
// This buffered browser path deliberately stays below ZIP32 and memory limits.
export const ARCHIVE_LIMIT = 256 * 1024 * 1024;
const crcTable = Uint32Array.from({length:256},(_,value)=>{
  for(let bit=0;bit<8;bit++)value=(value>>>1)^((value&1)?0xedb88320:0);
  return value>>>0;
});
function crc32(bytes) {
  let crc=0xffffffff;
  for(const byte of bytes)crc=(crc>>>8)^crcTable[(crc^byte)&255];
  return (crc^0xffffffff)>>>0;
}
function archiveName(name,used) {
  const safe=String(name).normalize('NFC').replace(/[\\/\x00-\x1f\x7f:]/g,'_').replace(/^[. ]+|[. ]+$/g,'') || 'file';
  const dot=safe.lastIndexOf('.'),stem=dot>0?safe.slice(0,dot):safe,extension=dot>0?safe.slice(dot):'';
  let candidate=safe,count=1;
  while(used.has(candidate.toLowerCase()))candidate=`${stem} (${++count})${extension}`;
  used.add(candidate.toLowerCase());return new TextEncoder().encode(candidate);
}
function archivePath(file,used,folders,paths) {
  if(file.path===undefined)return archiveName(file.name,used);
  const parts=String(file.path).normalize('NFC').split('/');
  if(parts.some(part=>!part || part==='.' || part==='..' || /[\\\x00-\x1f\x7f]/.test(part)) || parts.length>64)throw new Error('Invalid archive path');
  if(paths.has(file.path))throw new Error('Duplicate archive path');paths.add(file.path);
  let prefix='';
  for(let i=0;i<parts.length;i++) {
    const directory=i<parts.length-1 || file.directory,raw=parts.slice(0,i+1).join('/');
    if(directory && folders.has(raw)) {prefix=folders.get(raw);continue;}
    const safe=parts[i].replace(/[:]/g,'_').replace(/^[. ]+|[. ]+$/g,'')||'file';
    const dot=!directory?safe.lastIndexOf('.'):-1,stem=dot>0?safe.slice(0,dot):safe,ext=dot>0?safe.slice(dot):'';
    let name=safe,n=1;
    while(used.has((prefix+name).toLowerCase()))name=`${stem} (${++n})${ext}`;
    used.add((prefix+name).toLowerCase());prefix+=name+(directory?'/':'');
    if(directory)folders.set(raw,prefix);
  }
  return new TextEncoder().encode(prefix);
}
export function downloadArchive(files) {
  if(files.length>10000)throw new RangeError('Archive too large');
  const parts=[],directory=[],used=new Set(),folders=new Map(),paths=new Set();let offset=0,directorySize=0;
  for(const file of files) {
    const bytes=file.directory?new Uint8Array():file.bytes,name=archivePath(file,used,folders,paths),size=bytes.byteLength;
    if(name.length>65535 || offset+directorySize+size+76+2*name.length+22>ARCHIVE_LIMIT)throw new RangeError('Archive too large');
    const local=new Uint8Array(30),central=new Uint8Array(46),a=new DataView(local.buffer),b=new DataView(central.buffer);
    const crc=crc32(bytes);
    a.setUint32(0,0x04034b50,true);a.setUint16(4,20,true);a.setUint16(6,0x0800,true);
    a.setUint16(12,0x21,true);a.setUint32(14,crc,true);a.setUint32(18,size,true);a.setUint32(22,size,true);a.setUint16(26,name.length,true);
    b.setUint32(0,0x02014b50,true);b.setUint16(4,20,true);b.setUint16(6,20,true);b.setUint16(8,0x0800,true);
    b.setUint16(14,0x21,true);b.setUint32(16,crc,true);b.setUint32(20,size,true);b.setUint32(24,size,true);b.setUint16(28,name.length,true);b.setUint32(42,offset,true);
    if(file.directory)b.setUint32(38,0x10,true);
    parts.push(local,name,bytes);directory.push(central,name);
    offset+=30+name.length+size;directorySize+=46+name.length;
  }
  const end=new Uint8Array(22),view=new DataView(end.buffer);
  view.setUint32(0,0x06054b50,true);view.setUint16(8,files.length,true);view.setUint16(10,files.length,true);
  view.setUint32(12,directorySize,true);view.setUint32(16,offset,true);
  return new Blob([...parts,...directory,end],{type:'application/zip'});
}
