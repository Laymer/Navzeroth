import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {useState} from 'react';
import {describe,it,expect,vi} from 'vitest';
import {CatalogPager,OffPageOptions} from './CatalogPager';
import type {CatalogPageInfo,CatalogRecordPage,CatalogPageClient} from '../commands/catalogPages';
const page:CatalogPageInfo={revision:'r1',catalog_sha256:'a'.repeat(64),collection:'nodes',offset:0,total:405,next_offset:200};
const subject={kind:'character' as const,profile_id:'p1'};
describe('bounded catalog browsing',()=>{
 it('requests exact binding, replaces rows and preserves an off-page selection',async()=>{
  const getCatalogPage=vi.fn(async(text:string)=>{const request=JSON.parse(text) as {offset:number};return {ok:true as const,value:{...page,offset:request.offset,next_offset:400,records:[{id:'node-201'}]}};});
  function Browser(){const [info,setInfo]=useState(page);const [rows,setRows]=useState(['node-1']);return <><select aria-label="Node" value="node-1" onChange={()=>undefined}><OffPageOptions selected={['node-1']} visible={rows}/>{rows.map(id=><option key={id}>{id}</option>)}</select><CatalogPager client={{getCatalogPage}} subject={subject} page={info} label="nodes" onPage={p=>{setInfo(p);setRows(p.records.map(r=>(r as {id:string}).id));}}/></>;}
  render(<Browser/>);fireEvent.click(screen.getByText('Next nodes'));
  await screen.findByText('node-201');expect(screen.getByLabelText('Node')).toHaveValue('node-1');
  expect(JSON.parse(getCatalogPage.mock.calls[0]?.[0]??"{}")).toMatchObject({subject,revision:'r1',catalog_sha256:page.catalog_sha256,offset:200});
  expect(screen.getAllByRole('option')).toHaveLength(2);
 });
 it('ignores a pending page from a previous scope',async()=>{
  let resolve:(value:{ok:true;value:CatalogRecordPage})=>void=()=>undefined;
  const client:CatalogPageClient={getCatalogPage:()=>new Promise(r=>{resolve=r;})};const onPage=vi.fn();
  const view=render(<CatalogPager client={client} subject={subject} page={page} label="nodes" onPage={onPage}/>);fireEvent.click(screen.getByText('Next nodes'));
  view.rerender(<CatalogPager client={client} subject={{kind:'character',profile_id:'p2'}} page={page} label="nodes" onPage={onPage}/>);
  await act(async()=>resolve({ok:true,value:{...page,offset:200,records:[]}}));expect(onPage).not.toHaveBeenCalled();
 });
 it('refuses a mismatched response and exposes the error',async()=>{
  const client:CatalogPageClient={getCatalogPage:async()=>({ok:true,value:{...page,revision:'other',offset:200,records:[]}})};const onPage=vi.fn();
  render(<CatalogPager client={client} subject={subject} page={page} label="nodes" onPage={onPage}/>);fireEvent.click(screen.getByText('Next nodes'));
  await waitFor(()=>expect(screen.getByRole('alert')).toHaveTextContent('Catalog page changed'));expect(onPage).not.toHaveBeenCalled();
 });
});
