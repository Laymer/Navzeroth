# Record provenance binding correction
Owner /root/inventory; baseline29008c2; branch fix/record-provenance.
Independent review of shared pack extraction found inherited record-source
mapping assigned every dataset source to every base record. This could let party
or release evidence borrow an unrelated source. Core records now resolve each
record's own evidence IDs; stages inherit quest evidence, pins retain their own,
and imagery includes its explicit source plus own/calibration/applicability
facts. TravelNode has no evidence field: its identity remains enumerable with
empty provenance, so it cannot satisfy source-bearing dependency claims. No
source is inferred from coordinates or adjacent records. V1 serialized documents
and hashes remain unchanged; release binding becomes stricter.

Three focused core tests passed: multiple-source separation, missing/empty
provenance, image facts and pin isolation. Eleven v2 pack tests passed, including
shared exact verified bytes and >1024 catalog. Strict core all-target Clippy
passed. Initial test assumptions about fixture quest evidence and missing-ID
parse success were corrected; the synthetic party helper now explicitly assigns
its quest evidence before calculating the catalog hash. No real evidence or
permissions were added. Next: coordinator independently reviews and integrates;
all nine hosted checks remain required. External content/client gates unchanged.
