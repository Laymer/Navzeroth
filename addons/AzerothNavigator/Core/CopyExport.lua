-- Select existing bounded export bytes for a manual copy view. No capture or I/O.
local _, ns = ...
local Copy = {}

function Copy.select(db, kind)
  local keys = {diagnostic = 'diagnostic_export', snapshot = 'snapshot_export',
    session = 'session_export'}
  local limits = {diagnostic = ns.Export.MAX_BYTES, snapshot = ns.Snapshot.MAX_BYTES,
    session = ns.Session.MAX_BYTES}
  if type(kind) ~= 'string' or not keys[kind] then return nil, 'copy_kind_invalid' end
  if type(db) ~= 'table' or db.db_version ~= 1 then return nil, 'copy_db_unavailable' end
  local key, limit = keys[kind], limits[kind]
  local text = db[key]
  if text == nil then return nil, 'copy_export_missing' end
  if type(text) ~= 'string' or #text == 0 then return nil, 'copy_export_invalid' end
  if #text > limit then return nil, 'copy_export_too_large' end
  local json = text
  if kind == 'snapshot' then
    if text:sub(1, 6) ~= 'ANAV1:' then return nil, 'copy_export_invalid' end
    json = text:sub(7)
  elseif kind == 'session' then
    if text:sub(1, #ns.Session.PREFIX) ~= ns.Session.PREFIX then
      return nil, 'copy_export_invalid'
    end
    json = text:sub(#ns.Session.PREFIX + 1)
  end
  local value = ns.Decode.decode(json, {bytes=limit})
  if type(value) ~= 'table' or ns.Decode.kind(value) ~= 'object' then
    return nil, 'copy_export_invalid'
  end
  if kind == 'snapshot' then
    if value.kind ~= 'character_snapshot' or value.schema_version ~= 1 or
      ns.Decode.kind(value.payload) ~= 'object' then return nil, 'copy_export_invalid' end
  elseif kind == 'session' then
    if value.format ~= 'anav-session-events' or value.version ~= 1 or
      ns.Decode.kind(value.batch) ~= 'object' then return nil, 'copy_export_invalid' end
  elseif value.format ~= ns.Version.format or value.version ~= ns.Version.format_version then
    return nil, 'copy_export_invalid'
  end
  -- Shape checks do not validate game truth, freshness or compatibility. Keep exact bytes.
  return {kind=kind, text=text, bytes=#text, limit=limit}
end

ns.provide('CopyExport', Copy)
