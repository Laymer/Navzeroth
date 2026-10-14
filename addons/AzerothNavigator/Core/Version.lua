-- Version and format constants. Pure data; no client globals.
local _, ns = ...

ns.provide("Version", {
  addon = "0.8.0",
  -- Diagnostic export format. This is NOT an ANAV1 envelope and NOT a contract kind.
  format = "anav-diagnostic",
  format_version = 1,
  -- Layout version of the SavedVariables table (docs/compatibility/ADDON_SAVEDVARIABLES.md).
  db_version = 1,
  saved_variables = "AzerothNavigatorDB",
  -- UNVERIFIED: community evidence for the Forever beta, not a measured value.
  toc_interface = 16001,
  toc_interface_unverified = true,
  -- The probe cannot read the edition; the target is recorded as an assumption.
  edition = "wow_forever",
  edition_basis = "assumed_target",
})
