import {useEffect,useRef,useState} from 'react';
import type {CatalogPageClient,CatalogPageInfo,CatalogRecordPage,CatalogSubject} from '../commands/catalogPages';
export function CatalogPager({client,subject,page,label,onPage,disabled=false}:{client:CatalogPageClient;subject:CatalogSubject;page?:CatalogPageInfo|undefined;label:string;disabled?:boolean;onPage:(page:CatalogRecordPage)=>void}) {
 const [busy,setBusy]=useState(false);const [error,setError]=useState('');const generation=useRef(0);
 const identity=JSON.stringify([subject,page?.revision,page?.catalog_sha256,page?.collection]);
 const current=useRef({identity,client});if(current.current.identity!==identity||current.current.client.getCatalogPage!==client.getCatalogPage){current.current={identity,client};generation.current++;}
 useEffect(()=>{setBusy(false);setError('');return()=>{generation.current++;};},[identity,client.getCatalogPage]);
 if(!page||page.total<=200)return null;
 async function move(offset:number){if(!page||busy||disabled)return;const ticket=++generation.current;setBusy(true);setError('');
  const response=await client.getCatalogPage(JSON.stringify({subject,revision:page.revision,catalog_sha256:page.catalog_sha256,collection:page.collection,offset}));
  if(ticket!==generation.current)return;setBusy(false);
  if(!response.ok){setError(response.error.message);return;}
  const value=response.value;
  if(value.revision!==page.revision||value.catalog_sha256!==page.catalog_sha256||value.collection!==page.collection||value.offset!==offset||value.records.length>200||value.total!==page.total){setError('Catalog page changed; refresh this view.');return;}
  onPage(value);
 }
 return <nav aria-label={`${label} pages`}><span>{label}: {page.offset+1}–{Math.min(page.offset+200,page.total)} of {page.total}. Selections on other pages are retained.</span> <button type="button" disabled={disabled||busy||page.offset===0} onClick={()=>void move(Math.max(0,page.offset-200))}>Previous {label}</button> <button type="button" disabled={disabled||busy||page.next_offset===null} onClick={()=>page.next_offset!==null&&void move(page.next_offset)}>Next {label}</button>{error&&<p role="alert">{error}</p>}</nav>;
}
export function OffPageOptions({selected,visible}:{selected:string[];visible:string[]}){return <>{[...new Set(selected)].filter(id=>id&&!visible.includes(id)).map(id=><option key={id} value={id}>{id} (selected on another page)</option>)}</>;}
