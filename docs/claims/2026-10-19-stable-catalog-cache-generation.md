# STABLE-CATALOG-CACHE-GENERATION

Owner/sole writer: world_review; branch fix/stable-catalog-cache-generation,
/tmp/navzeroth-stable-catalog-cache-generation. Baseline
35c0dc0137d4a5f38efa996408894b92140431c7, tracked rehearsal code only.

Fix immutable parsed-catalog/index cache invalidation on unrelated snapshot,
reminder and settings writes. Preserve same-connection content mutations,
external commits, schema changes, staging/activation and transaction/rollback
semantics. No DB migration, production content/source/receipt edits or validation
shortcuts. Root independently reviews before integration. Measure actual full-pack
behavior after integration; no ordinary-PC or Windows performance claim.
