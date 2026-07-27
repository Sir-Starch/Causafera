# CLA Service Configuration — Maintainer Reference

Status of the CLA acceptance workflow: **configured and verified.**

External contributions may be merged once the contributor has accepted the CLA, the required checks
pass, and the maintainer approves the change. Opening a pull request does not by itself accept the
CLA.

This document records how the acceptance workflow is wired, so it can be audited, reproduced, or
updated. The moving parts live outside this repository: a public GitHub Gist holding the agreement
text, the hosted [CLA Assistant](https://cla-assistant.io/) service, and the `main` branch ruleset.

Related: [`CLA.md`](../../CLA.md), [`GOVERNANCE.md`](../../GOVERNANCE.md), and
[`CONTRIBUTING.md`](../../CONTRIBUTING.md).

## Published CLA source

CLA Assistant reads the agreement text from a **GitHub Gist** and tracks acceptance against a
specific **Gist revision**. This is the mechanism the service supports; release assets, arbitrary
commit permalinks, and other generic immutable URLs are not alternatives.

| CLA version | Gist | Gist revision |
| --- | --- | --- |
| 1.1 | <https://gist.github.com/Sir-Starch/eb32d78ea648f989831f7aa0a3bac81c> | `7c6daa72020318c47d14bca27655097cce236d6b` |

The Gist is public, uses the filename `CLA.md`, and is byte-identical to
[`CLA.md`](../../CLA.md) in this repository:

```text
Size: 15587 bytes
SHA-256: d32a10dfd75d4efd6ee42792632d002e78a13fe4d1a0f549bd81267afc5b6e36
Git blob SHA: 3c89692912e7d645e376cded4d6547ca1f874fc7
```

Verify the repository side with `wc -c < CLA.md`, `sha256sum CLA.md`, and `git hash-object CLA.md`.

> [`CLA.md`](../../CLA.md) deliberately carries no operational status, and its links are absolute
> URLs. It is the text contributors sign, published to a Gist where relative links would be dead and
> where any edit creates a new revision. Status belongs in `CONTRIBUTING.md`, `README.md`,
> `docs/development/contributing.md`, and the pull request template — never in the agreement itself.

## Service configuration

Configuration page: <https://cla-assistant.io/Sir-Starch/Causafera>

| Setting | Value |
| --- | --- |
| Linked repository | `Sir-Starch/Causafera` |
| Linked file | `CLA.md`, from the Gist above |
| Shared Gist | disabled — this agreement covers one repository, not an account-wide set |
| Minimum file/line-count exemption | none — no contribution is small enough to bypass the CLA |
| Status check context | `license/cla` |
| Webhook | `pull_request` and `merge_group` events |

The service is linked to the Gist, not to a hand-picked revision: it follows the Gist, detects a
later revision, and asks for a fresh signature on the next pull request. The recorded revision hash
is the maintainer's audit trail, not a service setting.

## Merge enforcement

`license/cla` is a **required status check** on `main`, configured in the `Protect main` repository
ruleset alongside the existing checks:

| Required check | Purpose |
| --- | --- |
| `rust` | Rust build, lint, and test gate |
| `ui` | Frontend lint, typecheck, and build gate |
| `license/cla` | CLA acceptance gate |

The ruleset also blocks branch deletion and non-fast-forward pushes, and requires branches to be up
to date before merging. It carries **no bypass actors**, so the rules apply to every pull request
including the maintainer's.

## Exemptions

Exemptions are configured in CLA Assistant's own allowlist, which is the narrowest mechanism
available: it stops the service from requesting a signature from a named account, and changes
nothing else. It does not touch branch protection, does not weaken any other required check, and
does not affect ordinary contributors.

| Exempt identity | Reason |
| --- | --- |
| `Sir-Starch` (user ID `281476371`) | The maintainer is the party the CLA grants rights *to*. Requiring them to sign their own agreement with themselves is meaningless, and would block every maintainer pull request. |
| `dependabot[bot]` | An automation account cannot meaningfully accept an agreement, and would otherwise be blocked forever by a check it can never satisfy. |

Deliberately **not** exempt: every other contributor, human or otherwise. Keep the allowlist minimal
and review it whenever new automation is added — an allowlisted identity is one whose contributions
merge with no CLA record at all. A repository-admin ruleset bypass was deliberately *not* used for
the maintainer exemption, because it would bypass `rust` and `ui` as well.

## Updating the CLA later

A material change to the CLA is not just a repository edit.

1. Edit [`CLA.md`](../../CLA.md), bump the version and effective date, and add a version-history
   entry.
2. Update the Gist with the same text, producing a **new revision**.
3. Record the new revision hash in the table above, and re-verify byte identity.
4. Confirm CLA Assistant detects the new revision of the linked Gist and asks for a fresh signature
   on the next pull request.
5. Verify that contributors who accepted the earlier revision are asked to sign again, and that the
   new acceptance is recorded against the new revision.
6. If the service cannot enforce re-acceptance, record that as a limitation and handle re-acceptance
   manually before merging further external work.

## Acceptance records

Acceptance records carry the fields required by section 9 of [`CLA.md`](../../CLA.md): authenticated
contributor GitHub identity with its numeric user ID where exported, repository identity, accepted
Gist revision, and timestamp. Association with a particular pull request comes from the service's
pull-request comment and status check, from GitHub's pull-request history, or from a private
maintainer register.

Export the records from the service periodically and store them privately, outside this repository
and outside the service, with a backup. Do not publish them; they contain contributor identity data.
The records and the Gist revisions they refer to must remain usable evidence if CLA Assistant becomes
unavailable or changes terms.

## Optional: professional legal review

Legal review is **not** a prerequisite for accepting external contributions. This is a hobby project,
and the CLA is project policy written to be clear rather than litigated.

> Consider professional legal review before entering material commercial licensing agreements,
> accepting substantial corporate contributions, assigning project rights, or relying on the CLA in a
> legal dispute.
