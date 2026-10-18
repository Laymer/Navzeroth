import {act,screen} from '@testing-library/react';
import type {Root} from 'react-dom/client';
const state=vi.hoisted(()=>({roots:[] as Root[],client:{label:'bootstrap client'}}));
vi.mock('react-dom/client',async importOriginal=>{
 const actual=await importOriginal<typeof import('react-dom/client')>();
 return {...actual,createRoot:(...args:Parameters<typeof actual.createRoot>)=>{const root=actual.createRoot(...args);state.roots.push(root);return root;}};
});
vi.mock('./commands',()=>({defaultClient:()=>state.client}));
vi.mock('./App',()=>({App:({client}:{client:{label:string}})=><main>Main workspace: {client.label}</main>}));
vi.mock('./screens/ReferenceView',()=>({ReferenceView:({client}:{client:{label:string}})=><main>Reference workspace: {client.label}</main>}));
vi.mock('./theme/Backdrop',()=>({Backdrop:()=>null}));
afterEach(async()=>{await act(async()=>{for(const root of state.roots.splice(0))root.unmount();});document.body.innerHTML='';history.replaceState(null,'','/');});
it('boots the normal workspace using the application client',async()=>{
 vi.resetModules();document.body.innerHTML='<div id="root"></div>';
 await act(async()=>{await import('./main');});
 expect(screen.getByText('Main workspace: bootstrap client')).toBeVisible();
 expect(screen.queryByText('Reference workspace: bootstrap client')).not.toBeInTheDocument();
});
it('boots a dedicated reference window without mounting the main workspace',async()=>{
 vi.resetModules();history.replaceState(null,'','/#reference');document.body.innerHTML='<div id="root"></div>';
 await act(async()=>{await import('./main');});
 expect(screen.getByText('Reference workspace: bootstrap client')).toBeVisible();
 expect(screen.queryByText('Main workspace: bootstrap client')).not.toBeInTheDocument();
});
it('refuses a missing mount target before creating a React root',async()=>{
 vi.resetModules();document.body.innerHTML='';
 await expect(import('./main')).rejects.toThrow('missing #root');
 expect(state.roots).toHaveLength(0);
});
