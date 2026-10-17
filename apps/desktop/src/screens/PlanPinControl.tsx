import { Button } from "../ui";
import {useEffect,useRef,useState} from "react";
import type {TodayClient} from "../commands/today";
export function PlanPinControl({client,jobId}:{client:TodayClient;jobId:string}) {
 const generation=useRef(0);const owner=useRef({client,jobId});if(owner.current.client!==client||owner.current.jobId!==jobId){owner.current={client,jobId};generation.current++;}
 const [pending,setPending]=useState<number|null>(null);const [message,setMessage]=useState<{epoch:number;text:string}|null>(null);
 useEffect(()=>()=>{generation.current++;},[]);
 async function pin(){const ticket=++generation.current;setPending(ticket);setMessage(null);const today=await client.getTodayView();if(ticket!==generation.current)return;if(!today.ok){setPending(null);setMessage({epoch:ticket,text:today.error.message});return;}
 const result=await client.pinCurrentPlan(jobId,today.value.current_plan.revision);if(ticket!==generation.current)return;setPending(null);setMessage({epoch:ticket,text:result.ok?"Saved on Today. It will be checked against current imported inputs when read.":result.error.message});}
 return <div><Button variant="primary" disabled={pending===generation.current} onClick={()=>void pin()}>Save on Today</Button>{message?.epoch===generation.current&&<p role="status">{message.text}</p>}</div>;
}
