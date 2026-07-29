# External adverse claim rule

This repository exists to build and verify a simulator. It does not use public criticism of researchers as an audit shortcut.

## Standing rule

No repository publication, issue, pull-request comment, message, or author contact may assert or imply that an external author, publication, dataset, table, identifier, or result is erroneous, invalid, retracted, misattributed, or in need of correction unless an exact release receipt passes the external-claim gate.

The same gate applies to a contact framed as a question. Polite wording does not reduce the possible reputational effect.

The release receipt must bind:

1. The exact UTF-8 claim text by SHA-256.
2. The exact destination by SHA-256.
3. One action, either `contact` or `publication`.
4. A private evidence-dossier digest.
5. At least five independent corroborating lineages beyond the subject artifact.
6. The canonical lineage-set digest.
7. The approver identity, expiry, and revocation state.
8. The exact subject, typed subject roots, and signature namespace.
9. A human signature over the exact canonical approval payload.
10. Agreement from the independent external-claim watchdog.

Five links are not five lineages. Mirrors, editions, a preprint and its journal version, OCR and visual reads of one table, or papers inheriting one dataset count as one lineage. Shared or unknown authorship, samples, apparatus, datasets, methods, or upstream evidence roots connect lineages conservatively. The connected components after those links are applied must number at least five.

There is no baseline, waiver, or `single_witness_reason` path. If the threshold is not met, the simulator may refuse the disputed input and the internal question may remain open. No adverse external claim or contact is authorized.

## Neutral records that do not require release

The repository may record an exact quotation, two parallel source values, an unreadable glyph, an unresolved discrepancy, or a project refusal without assigning fault. It may also say that its own transcription, code, fetch seed, or earlier prose was wrong when the statement does not attribute fault externally.

Pending public register rows contain neutral subject metadata and an optional digest of a private dossier. They do not contain unapproved adverse prose, private email addresses, or contact details.

## Authority boundary

The Python producer and independent watchdog can verify declared lineages, connected components, digests, exact bytes, and signatures. They cannot prove that a scientific independence classification is sound or that a proposed claim is true. The signed human approval remains the final authority boundary.

Approval is scoped. Contact approval cannot authorize publication. Publication approval cannot authorize contact or another destination. A one-byte text or destination change requires a new signature. Expired or revoked approval authorizes nothing.

Revocation is independently protected. The signed payload binds its inline revocation state, and its digest must also be absent from `sources/external_claim_revocations.toml`. The gate receipt binds the exact revocation-registry digest. Adding a digest there invalidates the release without editing or re-signing its row.

The public-surface phrase scan is a conservative tripwire, not a semantic proof. It can catch known wording in repository text but cannot understand every paraphrase or inspect a GitHub comment, email, chat message, or other outbound action. No future outbound publisher or contact tool is authorized to bypass the signed release gate. Such a tool must consume the exact approved text and destination, verify both implementations immediately before dispatch, and fail closed if the protected revocation registry or any signed field differs.

## Current activation state

The register, protected revocation registry, and two mechanical implementations are fail-closed. No release row exists. Adding one requires an owner-reviewed approver key and signature receipt, five independent components, and exact agreement between both implementations. The current tooling authorizes no outbound action; it only validates repository state.
