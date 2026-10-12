-- Diagnostic export. Pure: builds a package-local JSON object from a probe report.
-- This is NOT an ANAV1 envelope and NOT a contract kind (docs/compatibility/
-- ADDON_SAVEDVARIABLES.md). The string is written to the SavedVariables table only
-- after encoding succeeded within MAX_BYTES; a failed encode never leaves a partial
-- string behind and keeps the previous export untouched.
local _, ns = ...

local Version = ns.Version
local Json = ns.Json

-- Cap on the encoded export, below the 32 KiB addon text bound of plan section 7.1.
local MAX_BYTES = 12288

local Export = { MAX_BYTES = MAX_BYTES }

-- Assembles the export document; capabilities are already sorted by id by the probe.
function Export.document(report)
  return {
    format = Version.format,
    version = Version.format_version,
    addon_version = Version.addon,
    toc_interface = Version.toc_interface,
    build = report.build,
    capabilities = report.capabilities,
  }
end

-- Returns the JSON string, or nil plus an error code.
function Export.encode(report)
  if type(report) ~= "table" then
    return nil, "export_invalid_report"
  end
  return Json.encode(Export.document(report), MAX_BYTES)
end

-- Stores the export into db.diagnostic_export. Returns true, or false plus an error
-- code (also recorded in db.diagnostic_export_error). Only these two fields are touched.
function Export.store(db, report)
  if type(db) ~= "table" then
    return false, "export_no_db"
  end
  local text, code = Export.encode(report)
  if not text then
    db.diagnostic_export_error = code
    return false, code
  end
  db.diagnostic_export = text
  db.diagnostic_export_error = nil
  return true
end

ns.provide("Export", Export)
