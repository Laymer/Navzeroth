import atlas from '../../../../fixtures/desktop/atlas/query-atlas.json';
import fresh from '../../../../fixtures/desktop/atlas/snapshot-markers-fresh.json';
import type {AtlasView,AtlasPin,SnapshotMarker,ProjectionGap,Freshness} from '../commands/contract';
import {formatDuration,describeUncertainty,describeGap,describeNotice,describePin,describePinAccess,describeLayerAvailability,describeUtc} from './describe';
import {markerState,placementOf} from './markerState';
function first<T>(values:T[]):T{const value=values[0];if(value===undefined)throw new Error('Missing synthetic atlas fixture');return value;}
const image=first((atlas as AtlasView).images);
const marker=first(fresh) as SnapshotMarker;
it.each([[0,'0 s'],[59000,'59 s'],[60000,'1 min'],[3600000,'1 h'],[3900000,'1 h 5 min'],[90000000,'1 d 1 h']] as const)('describes duration %i without inflating precision', (value,label)=>expect(formatDuration(value)).toBe(label));
it.each([
 ['missing_or_untrusted_time','No trusted clock'],['clock_reversed','later than the current time'],['age_out_of_range','outside the supported range'],
] as const)('preserves the reason age cannot be stated: %s',(reason,words)=>expect(describeUncertainty(reason)).toContain(words));
it('keeps every projection refusal distinct from drawable geography',()=>{
 const map={...image.map,floor:'2'};
 const gaps:[ProjectionGap,string][]=[
  [{gap:'namespace_mismatch'},'different dataset namespace'],
  [{gap:'no_image_for_map',map},'no imagery for map demo-map-1'],
  [{gap:'in_instance',map},'is an instance'],
  [{gap:'different_floor',map,available_floors:['0','1']},'floor 2'],
  [{gap:'build_mismatch',position_build:map.context.build,image:null,applicable_builds:[]},'recorded on build UNVERIFIED_DEMO'],
  [{gap:'embedding_chain_too_deep',via:[map]},'nesting is deeper'],
  [{gap:'uncalibrated_image',image:'image-unknown',status:'restricted'},'no known calibration (restricted)'],
  [{gap:'out_of_bounds',image:'image-unknown',pixel:{x:-1,y:0}},'outside the frame'],
 ];
 for(const [gap,words] of gaps){expect(describeGap(gap)).toContain(words);expect(placementOf({...marker,projection:{outcome:'gap',...gap}})).toBeNull();}
 expect(describeNotice({notice:'embedding_fallback',via:[map]})).toContain('via demo-map-1');
 expect(describeNotice({notice:'cross_build_reuse',image_build:map.context.build})).toContain('build UNVERIFIED_DEMO is reused');
});
it('labels reference pins/access and refuses to imply character flight unlocks',()=>{
 const pin=first(image.pins);
 for(const [kind,label] of [[{kind:'settlement'},'Reference settlement'],[{kind:'landmark'},'Reference landmark']] as const){const value={...pin,kind} as AtlasPin;expect(describePin(value)).toBe(label);expect(describePinAccess(value)).toBeNull();}
 const flight:AtlasPin={...pin,kind:{kind:'flight_master',access:['alliance','horde','tower_route']}};
 expect(describePin(flight)).toContain('Not your unlocks');expect(describePinAccess(flight)).toBe('Access: Alliance, Horde, tower route');
 expect(describePinAccess({...flight,kind:{kind:'flight_master',access:[]}})).toBeNull();
});
it('separates missing, unservable and integrity-refused layers and preserves out-of-calendar UTC',()=>{
 const layer=first(image.layers);
 for(const [availability,words] of [['available','available'],['not_embedded','no file is installed'],['integrity_mismatch','does not match'],['unservable_id','cannot be requested']] as const)expect(describeLayerAvailability({...layer,availability})).toContain(words);
 expect(describeUtc(0)).toBe('1970-01-01T00:00:00.000Z');expect(describeUtc(Number.MAX_SAFE_INTEGER)).toContain('outside date display range');
});
it('unavailable location states never become drawable; all freshness/calibration states stay explicit',()=>{
 for(const location_status of ['unknown','unsupported','restricted','not_applicable'] as const){const value={...marker,location_status,projection:null};expect(markerState(value)).toBe('location_'+location_status);expect(placementOf(value)).toBeNull();}
 const ages:Freshness[]=[{status:'fresh',age_ms:0},{status:'stale',age_ms:60000},{status:'uncertain',reason:'clock_reversed'}];
 for(const freshness of ages)for(const approximate of [true,false]){const value={...marker,freshness,approximate};expect(markerState(value)).toBe(`placed_${freshness.status}_${approximate?'approximate':'calibrated'}`);expect(placementOf(value)?.approximate).toBe(approximate);}
});
