-- Injected clock observations. UTC trust requires explicit per-runtime confirmation.
local _, ns = ...
local Clock = {}
local SAFE_SECONDS = math.floor(9007199254740991 / 1000)
local function valid(value, integer)
  return type(value) == 'number' and value == value and value >= 0 and
    value <= SAFE_SECONDS and (not integer or value == math.floor(value))
end
local function coherent(a, b)
  return b.mono >= a.mono and b.utc >= a.utc and
    math.abs((b.utc - a.utc) - (b.mono - a.mono)) <= 5
end
function Clock.new(utc, monotonic)
  local pending, anchor, previous
  local state = {}
  local function sample()
    local ok, value = pcall(function()
      if type(utc) ~= 'function' or type(monotonic) ~= 'function' then return nil end
      local before, seconds = monotonic(), utc()
      local after = monotonic()
      if not valid(seconds, true) or not valid(before) or not valid(after) or
        after < before or after - before > 2 then return nil end
      return {utc=seconds, mono=after}
    end)
    if not ok or not value then return nil, 'clock_unavailable' end
    return value
  end
  state.probe = function(_)
    -- Probing starts a new confirmation; an old trust decision is never reused.
    pending, anchor, previous = nil, nil, nil
    local value, code = sample()
    pending = value
    if not value then return nil, code end
    return {utc_seconds=value.utc, clock_trust='uncertain'}
  end
  state.confirm = function(_)
    local candidate = pending
    pending, anchor, previous = nil, nil, nil
    if not candidate then return nil, 'clock_confirmation_missing' end
    local value, code = sample()
    if not value then return nil, code end
    if not coherent(candidate, value) or value.mono - candidate.mono > 120 then
      return nil, 'clock_confirmation_expired_or_changed'
    end
    anchor, previous = value, value
    return {utc_seconds=value.utc, clock_trust='trusted'}
  end
  state.capture = function(_)
    local value, code = sample()
    if not value then
      pending, anchor, previous = nil, nil, nil
      return {clock_trust='uncertain'}, code
    end
    if anchor and (not coherent(anchor, value) or not coherent(previous, value)) then
      pending, anchor, previous = nil, nil, nil
      return {clock_trust='uncertain', observed_at_utc_ms=value.utc * 1000},
        'clock_changed'
    end
    local trust = anchor and 'trusted' or 'uncertain'
    if anchor then previous = value end
    return {clock_trust=trust, observed_at_utc_ms=value.utc * 1000}
  end
  state.clear = function(_) pending, anchor, previous = nil, nil, nil end
  -- Read-only view for display: unlike capture() it never changes trust or the
  -- pending probe. available=false means the clock APIs could not be read now.
  state.status = function(_)
    local result = {available=false, pending=pending ~= nil, trusted=false,
      confirmable=false}
    local value = sample()
    if not value then return result end
    result.available = true
    if pending then
      result.pending_utc_seconds = pending.utc
      if value.mono >= pending.mono then result.age_seconds = value.mono - pending.mono end
      result.confirmable = result.age_seconds ~= nil and result.age_seconds <= 120 and
        coherent(pending, value)
    end
    result.trusted = anchor ~= nil and coherent(anchor, value) and coherent(previous, value)
    return result
  end
  return state
end
ns.provide('Clock', Clock)
