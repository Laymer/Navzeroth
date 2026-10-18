-- Opt-in recorder with manual lifecycle. Pure injected clocks/reads; no timer.
local _, ns = ...
local Session, V = {MAX_EVENTS=128}, ns.Predicate.validation
Session.PREFIX = 'ANAVSESSION1:'
-- The encoder is bounded by the JSON output cap, so a stored export never exceeds this.
Session.MAX_BYTES = #Session.PREFIX + ns.Json.limits.max_output_bytes
local SAFE = 9007199254740991
local function check(ok) if not ok then error('session_invalid',0) end end
local function plain(value)
  -- Rust optional fields may encode as null. Omission keeps the same None meaning.
  if value==ns.Decode.null then return nil end
  if type(value)~='table' then return value end
  local copy={}
  for key,child in pairs(value) do copy[key]=plain(child) end
  return copy
end
local function frozen(value)
  local text, code = ns.Json.encode(plain(value))
  if not text then return nil, code end
  return plain(assert(ns.Decode.decode(text)))
end
function Session.encode(document)
  local text, code = ns.Json.encode(plain(document))
  if not text then return nil,code end
  return 'ANAVSESSION1:' .. text
end
-- start receives a fresh identity from the binding's independent persisted counter.
-- clock returns metadata from the same runtime confirmation used by snapshots.
function Session.new(monotonic, clock)
  local state = {phase='idle',events=0}
  local document, origin, previous, lastUtc, used = nil,nil,nil,nil,{}
  local function fail(code)
    state.phase, state.reason = 'stopped', code
    return nil, code
  end
  local function reading()
    local ok, value = pcall(function()
      check(type(monotonic)=='function')
      local seconds=monotonic()
      check(type(seconds)=='number' and seconds==seconds and seconds>=0 and seconds<=SAFE/1000)
      local mono=math.floor(seconds*1000)
      if previous and mono<previous then return nil,'session_clock_reset' end
      local metadata={clock_trust='uncertain'}
      if type(clock)=='function' then
        local read, result=pcall(clock)
        if read and type(result)=='table' then metadata=result end
      end
      V.metadata(metadata)
      return {mono=mono,utc=metadata.observed_at_utc_ms,
        clock_trust=metadata.clock_trust or 'uncertain'}
    end)
    if not ok then return fail('session_clock_unavailable') end
    if not value then return fail('session_clock_reset') end
    if previous and lastUtc and lastUtc.clock_trust=='trusted' and value.clock_trust=='trusted' and
      lastUtc.utc and value.utc and (value.utc<lastUtc.utc or
      math.abs((value.utc-lastUtc.utc)-(value.mono-previous))>5000) then
      value.clock_trust='uncertain'
      state.clock_warning='session_utc_changed'
    end
    previous,lastUtc=value.mono,value
    return value
  end
  local function append(kinds,value)
    if #document.batch.events+#kinds>Session.MAX_EVENTS then return fail('session_event_limit') end
    local candidate = assert(frozen(document))
    local elapsed=value.mono-origin
    for _,kind in ipairs(kinds) do
      local sequence=#candidate.batch.events+1
      if kind.type=='level' then
        kind.level.session_id=document.batch.events[1].session
        kind.level.session_elapsed_ms,kind.level.sequence=elapsed,sequence
      end
      candidate.batch.events[#candidate.batch.events+1]={session=document.batch.events[1].session,
        sequence=sequence,elapsed_ms=elapsed,utc=value.utc,clock_trust=value.clock_trust,kind=kind}
    end
    local text,code=Session.encode(candidate)
    if not text then return fail('session_bound_' .. code) end
    document=plain(assert(ns.Decode.decode(text:sub(14))))
    state.events=#document.batch.events
    return true
  end
  local function active(phase)
    if not document or state.phase~=phase then return nil,'session_' .. phase .. '_required' end
    return true
  end
  local function unclassified(label)
    return {type='activity',label=label or 'manual_sample',quality='unclassified'}
  end
  state.start=function(_,spec)
    if state.phase=='recording' or state.phase=='paused' then return nil,'session_stop_required' end
    local valid,copy=pcall(function()
      V.id(spec.session);V.id(spec.character);V.context(spec.context)
      check(spec.context.build.edition=='wow_forever' and type(spec.fixture)=='boolean')
      check(spec.context.namespace~='demo' or spec.fixture)
      check(not used[spec.session])
      if spec.locale~=nil then
        check(type(spec.locale)=='string' and
          ({enUS=true,enGB=true,deDE=true,esES=true,esMX=true,frFR=true,
          itIT=true,koKR=true,ptBR=true,ruRU=true,zhCN=true,zhTW=true})[spec.locale]==true)
      end
      return assert(frozen(spec))
    end)
    if not valid then return nil,'session_invalid_identity' end
    -- Build candidate locally; a failed start retains the previous exportable batch.
    origin,previous,lastUtc=nil,nil,nil
    local value,code=reading();if not value then return nil,code end
    origin=value.mono
    local candidate={format='anav-session-events',version=copy.locale and 2 or 1,
      locale=copy.locale,fixture=copy.fixture,
      batch={character=copy.character,context=copy.context,events={{session=copy.session,sequence=1,
        elapsed_ms=0,utc=value.utc,clock_trust=value.clock_trust,kind={type='session_start'}}}}}
    local text,errorCode=Session.encode(candidate);if not text then return fail(errorCode) end
    document=plain(assert(ns.Decode.decode(text:sub(14))));used[copy.session]=true
    state.phase,state.reason,state.clock_warning,state.events='recording',nil,nil,1
    return true
  end
  state.sample=function(_,levelReader)
    local ok,code=active('recording');if not ok then return nil,code end
    local value,errorCode=reading();if not value then return nil,errorCode end
    local metadata={source=document.fixture and 'synthetic_fixture' or 'addon_snapshot',
      observed_at_utc_ms=value.utc,clock_trust=value.clock_trust}
    local read,level=pcall(function()
      local result=type(levelReader)=='function' and levelReader(metadata) or {status='unknown'}
      V.observation(result,function(_,owner,key) V.integer(owner,key,1,65535) end)
      return assert(frozen(result))
    end)
    if not read then
      level={status='unknown'};state.observation_warning='session_level_unreadable'
    end
    return append({unclassified(),{type='level',level=level}},value)
  end
  state.questTurnIn=function(_,questId)
    local ok,code=active('recording');if not ok then return nil,code end
    local valid,id=pcall(function()
      check(type(questId)=='number' and questId==questId and questId==math.floor(questId)
        and questId>0 and questId<=4294967295)
      return string.format('%.0f',questId)
    end)
    if not valid then return fail('session_quest_unreadable') end
    local value,errorCode=reading();if not value then return nil,errorCode end
    -- Observing a milestone does not classify the time before it as questing.
    return append({unclassified('quest_turn_in_interval'),{type='quest_turn_in',quest=id}},value)
  end
  state.unavailable=function(_) return fail('session_observation_unavailable') end
  state.pause=function(_)
    local ok,code=active('recording');if not ok then return nil,code end
    local value,errorCode=reading();if not value then return nil,errorCode end
    local result,why=append({unclassified(),{type='recording_paused'}},value)
    if result then state.phase='paused' end;return result,why
  end
  state.resume=function(_)
    local ok,code=active('paused');if not ok then return nil,code end
    local value,errorCode=reading();if not value then return nil,errorCode end
    local result,why=append({{type='recording_resumed'}},value)
    if result then state.phase='recording' end;return result,why
  end
  state.stop=function(_)
    if state.phase~='recording' and state.phase~='paused' then
      return nil,'session_active_required'
    end
    local value,code=reading();if not value then return nil,code end
    -- Manual stop observes recording being disabled, never a game logout.
    local terminal={type='recording_paused'}
    local kinds=state.phase=='paused' and {terminal} or {unclassified(),terminal}
    local result,why=append(kinds,value)
    if result then state.phase,state.reason='stopped','manual_stop' end;return result,why
  end
  state.export=function(_,db)
    if state.phase~='stopped' then return nil,'session_stop_before_export' end
    if not document then return nil,'session_missing' end
    if type(db)~='table' or getmetatable(db)~=nil or db.db_version~=ns.Version.db_version then
      return nil,'session_db_version'
    end
    local text,code=Session.encode(document)
    if not text then db.session_export_error=code;return nil,code end
    db.session_export,db.session_export_error=text,nil
    return text
  end
  state.status=function(_) return {phase=state.phase,reason=state.reason,events=state.events,
    clock_warning=state.clock_warning,observation_warning=state.observation_warning,
    session=document and document.batch.events[1].session} end
  return state
end
ns.provide('Session',Session)
