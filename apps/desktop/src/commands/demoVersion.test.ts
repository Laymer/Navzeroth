import desktopPackage from '../../package.json';
import {demoAdapter} from './demoAdapter';
it('reports the same application version from browser status and reference view without upgrading demo capabilities',async()=>{
 const status=await demoAdapter.invoke('get_app_status',undefined);
 const reference=await demoAdapter.invoke('get_reference_view',undefined);
 expect(status.version).toBe(desktopPackage.version);
 expect(status.version).toBe(reference.app_version);
 expect(status.data_mode).toBe('demo');
 expect(status.capabilities).toEqual({addon_transport:'unverified',live_position:'unverified'});
});
