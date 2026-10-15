import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {AuthoredCatalog,AuthoredSelection} from "../commands/authoredContent";
export function AuthoredContentPicker({client,kind,subject,refreshKey,value,onChange}:{client:CommandClient;kind:"models"|"dungeons";subject:{kind:"character";profile_id:string}|{kind:"party";party_id:string};refreshKey:number;value:AuthoredSelection|null;onChange:(v:AuthoredSelection|null)=>void}) {
  const [page,setPage]=useState<AuthoredCatalog|null>(null);const [offset,setOffset]=useState(0);const [error,setError]=useState("");const generation=useRef(0);
  const identity=JSON.stringify(subject);const selectedId="profile_id" in subject?subject.profile_id:subject.party_id;
  const current=useRef({identity,kind,client,refreshKey});
  if(current.current.identity!==identity||current.current.kind!==kind||current.current.client!==client||current.current.refreshKey!==refreshKey){current.current={identity,kind,client,refreshKey};generation.current++;}
  useEffect(()=>{let live=true;const ticket=++generation.current;setPage(null);setOffset(0);setError("");
    if(selectedId) void client.getAuthoredCatalog(JSON.stringify({subject:JSON.parse(identity) as unknown,kind,revision:null,offset:0})).then(result=>{if(!live||ticket!==generation.current)return;if(result.ok)setPage(result.value);else setError(result.error.message);});
    return()=>{live=false;generation.current++;};
  },[client,identity,kind,refreshKey,selectedId]);
  async function changePage(next:number){if(!page)return;const ticket=++generation.current;setError("");const result=await client.getAuthoredCatalog(JSON.stringify({subject,kind,revision:page.revision,offset:next}));if(ticket!==generation.current)return;if(result.ok){setPage(result.value);setOffset(next);}else{setPage(null);setError(result.error.message);}}
  const label=kind==="models"?"Authored gear model":"Authored dungeon";
  return <fieldset><legend>{label}</legend><p>Selections use the exact active content revision and its cited evidence. Missing facts remain unavailable.</p>
    <label>{label} selection <select value={value?.id??""} onChange={e=>onChange(e.target.value&&page?.revision?{id:e.target.value,revision:page.revision}:null)}><option value="">Manual / no authored selection</option>{value&&!page?.records.some(r=>r.id===value.id)&&<option value={value.id}>{value.id} (selected revision {value.revision})</option>}{page?.records.map(r=><option key={r.id} value={r.id}>{r.label}</option>)}</select></label>
    {page&&<p>{page.total} records; revision {page.revision??"unavailable"}. {page.records.length===0&&"No authored records on this page."}</p>}
    {value&&page&&value.revision!==page.revision&&<p role="alert">Selected revision changed. Select the model or dungeon again before evaluating.</p>}
    {value&&page?.records.find(r=>r.id===value.id)&&<p>Evidence: {page.records.find(r=>r.id===value.id)?.evidence.join(", ")}</p>}
    {page&&<><button type="button" disabled={offset===0} onClick={()=>void changePage(Math.max(0,offset-200))}>Previous authored records</button><button type="button" disabled={page.next_offset===null} onClick={()=>void changePage(page.next_offset??offset)}>Next authored records</button></>}
    {error&&<p role="alert">{error}</p>}
  </fieldset>;
}
