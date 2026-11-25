# Competency registry V1

The native host registry maps five bounded competency families to built-in
skill IDs, registered tool-pack candidates, verification guidance and known
limitations. It is advisory: it does not install skills, execute workflows,
grant permissions, certify dependencies, or replace existing tool validation.

The agent inserts a bounded hint into the original system context. Actual
discovery determines which tool schemas are selected. Other candidates need
fresh discovery; not selected does not mean unavailable. Tool policy and exact
review remain authoritative. Matching uses whole words, is not semantic intent
classification, and selects at most four families.

Tests validate that registry references exist and that undiscovered tools
are not labeled discovered. Real-model acceptance and unified executable
workflow contracts remain future work.
