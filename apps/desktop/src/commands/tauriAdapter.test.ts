import {invoke} from '@tauri-apps/api/core';
import {isTauri,tauriAdapter} from './tauriAdapter';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn()}));
afterEach(()=>{vi.clearAllMocks();Reflect.deleteProperty(window,'__TAURI_INTERNALS__');});
it('preserves command payloads and normalizes absent payloads for native IPC',async()=>{
 vi.mocked(invoke).mockResolvedValue({status:'ready'});
 await tauriAdapter.invoke('get_database_status',undefined);
 expect(invoke).toHaveBeenLastCalledWith('get_database_status',{});
 const input={profile_id:'synthetic-profile'};
 await tauriAdapter.invoke('get_character_snapshot',input);
 expect(invoke).toHaveBeenLastCalledWith('get_character_snapshot',input);
});
it('propagates native rejections and detects the native context without assuming browser access',async()=>{
 expect(isTauri()).toBe(false);Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{},configurable:true});expect(isTauri()).toBe(true);
 vi.mocked(invoke).mockRejectedValue(new Error('native refused'));
 await expect(tauriAdapter.invoke('list_profiles',undefined)).rejects.toThrow('native refused');
 vi.stubGlobal('window',undefined);expect(isTauri()).toBe(false);vi.unstubAllGlobals();
});
