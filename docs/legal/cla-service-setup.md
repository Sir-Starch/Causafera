# CLA Service Setup — Maintainer Checklist

Status of the CLA acceptance service: **configured and linked, not yet verified.** CLA Assistant is
configured and linked to the repository. End-to-end verification and merge enforcement are still
pending, so external contributions cannot yet be merged.

Concretely: steps 1 and 2 below are done — the CLA is published as a public Gist and CLA Assistant is
linked to `Sir-Starch/Causafera`. Steps 3 to 6 are not. Nothing has yet been tested against a real
pull request, no acceptance record has been produced or inspected, the CLA check is not a required
check on `main`, bot handling is untested, and no records have been exported. A configuration page
that reports an active link is not the same as a verified workflow.

This document is the repository-side record of the manual work required to enable CLA acceptance.
Every step happens **outside this repository**, in a GitHub Gist, in GitHub settings, and in the
hosted [CLA Assistant](https://cla-assistant.io/) service. Do not update the contribution-status text
in [`CONTRIBUTING.md`](../../CONTRIBUTING.md), [`README.md`](../../README.md),
[`docs/development/contributing.md`](../development/contributing.md), or
[`.github/pull_request_template.md`](../../.github/pull_request_template.md) to say contributions can
be merged until the workflow has been tested end to end.

> **[`CLA.md`](../../CLA.md) deliberately carries no operational status.** It is the text people
> sign, and it must read identically before and after the service is configured. Putting a
> "not yet enabled" banner in it would mean editing it the moment the service goes live, breaking
> the Gist's byte-identity and requiring everyone to re-accept a CLA they had just accepted. Status
> belongs in the four files listed above; the agreement text stays stable.
>
> For the same reason, links inside `CLA.md` are absolute URLs. Relative links resolve against the
> Gist, not the repository, and would be dead for the person reading the agreement.

Related: [`CLA.md`](../../CLA.md), [`GOVERNANCE.md`](../../GOVERNANCE.md), and `TODO-LEGAL-001` in
[`docs/development/todo-backlog.md`](../development/todo-backlog.md).

## 1. Publish the CLA as a public Gist

CLA Assistant reads the agreement text from a **GitHub Gist** and tracks acceptance against a
specific **Gist revision**. This is the mechanism the service actually supports; release assets,
arbitrary commit permalinks, and other generic immutable URLs are not alternatives here.

- [x] Create a **public** Gist containing the exact text of [`CLA.md`](../../CLA.md).
- [x] Use the filename `CLA.md` and the description `Causafera Contributor License Agreement v1.1`.
- [x] Confirm the Gist content is byte-identical to the repository copy — the Gist is what
      contributors legally accept, and a drift between the two means people accepted something other
      than what the repository documents.
- [x] Note the Gist URL and the **specific revision hash** shown under the Gist's Revisions tab.
- [x] Record both in the table below.

| CLA version | Gist URL | Gist revision | Recorded |
| --- | --- | --- | --- |
| 1.1 | https://gist.github.com/Sir-Starch/eb32d78ea648f989831f7aa0a3bac81c | `7c6daa72020318c47d14bca27655097cce236d6b` | 2026-07-27 |

Gist ID `eb32d78ea648f989831f7aa0a3bac81c`, public. Integrity evidence for the published revision,
which matches the branch copy of `CLA.md`:

```text
Size: 15587 bytes
SHA-256: d32a10dfd75d4efd6ee42792632d002e78a13fe4d1a0f549bd81267afc5b6e36
Git blob SHA: 3c89692912e7d645e376cded4d6547ca1f874fc7
```

Reproduce the repository side with `wc -c < CLA.md`, `sha256sum CLA.md`, and `git hash-object CLA.md`.
Any future change to `CLA.md` must go through step 7, not through an unrecorded edit.

Editing the Gist creates a **new revision**; it does not rewrite the old one. That is what makes
versioned acceptance work, and why the revision hash — not just the URL — must be recorded.

## 2. Connect the repository

Configuration page: <https://cla-assistant.io/Sir-Starch/Causafera>

- [x] Sign in to CLA Assistant with the maintainer GitHub account (user ID `281476371`).
- [x] Authorize the app against `Sir-Starch/Causafera` only, not the whole account, and review the
      permissions it requests before granting them.
- [x] Create the CLA configuration linked to the Gist from step 1, using the `CLA.md` file. The
      service follows that Gist and picks up its later revisions; there is no per-revision
      configuration to maintain.
- [x] Leave **Shared Gist** disabled — this agreement covers one repository, not an account-wide set.
- [x] Configure **no** minimum file-count or line-count exemption, so no contribution is small enough
      to bypass the CLA.
- [x] Confirm the service reports the repository/Gist link as active.
- [ ] Confirm the resulting status check name, so it can be required in step 4. **Not done** — the
      check name cannot be confirmed until it has actually appeared on a pull request in step 3.

## 3. Test with a real pull request

**Not started.** This is the step that turns a configured service into a verified one, and until it
passes, external contributions cannot be merged.

- [ ] Open a throwaway pull request from a **second** account that has never accepted the CLA. The
      maintainer account cannot test this path, because it will typically be treated as exempt.
- [ ] Confirm the CLA check appears and reports **failing/pending** before acceptance.
- [ ] Confirm the service posts its pull-request comment linking to the Gist.
- [ ] Accept through the service as the test account.
- [ ] Confirm the check flips to **passing**, and that this happens without a new push.
- [ ] Confirm the acceptance record carries the fields required by section 9 of
      [`CLA.md`](../../CLA.md): authenticated contributor GitHub identity (with its numeric user ID
      where exported), repository identity, accepted Gist revision, and timestamp.
- [ ] Note how the pull request is associated with the acceptance — service comment, status check, or
      the private register described in step 6 — since this is not necessarily an exported field.
- [ ] Close the test pull request without merging.

## 4. Require the check on `main`

- [ ] Confirm the check is visible on the pull request page, not only in the service dashboard.
- [ ] Confirm a contributor who has already accepted the current revision passes without being asked
      again.
- [ ] In branch protection or the repository ruleset for `main`, add the CLA status check to the
      **required** checks, alongside the existing `rust` and `ui` CI jobs.
- [ ] Confirm that a pull request without CLA acceptance is blocked from merging by the branch rule
      itself, not merely visually marked.
- [ ] Decide deliberately whether administrators are included in the restriction.

## 5. Handle bots and exempt accounts

Automation accounts cannot meaningfully accept a CLA, and their pull requests must not be blocked
forever by a check they can never satisfy.

- [ ] Add `dependabot[bot]` to the service's allowlist, if Dependabot is enabled.
- [ ] Add any other automation account that opens pull requests against this repository.
- [ ] Verify with a real bot pull request that the check passes rather than hangs.
- [ ] Keep the allowlist minimal and review it whenever a new automation is added — an allowlisted
      identity is an identity whose contributions are merged with no CLA record at all.

## 6. Export and retain acceptance records

- [ ] Export the acceptance records from the service after the first real acceptance.
- [ ] Store them privately, outside this repository and outside the service, in a durable location
      with a backup. Do not publish them; they contain contributor identity data.
- [ ] Keep a private maintainer register mapping each accepting identity to the first pull request it
      was associated with, if the service's own export does not carry that association.
- [ ] Set a recurring reminder to re-export, so the records survive the service becoming unavailable
      or changing terms.
- [ ] Establish what happens if CLA Assistant is discontinued: the records, and the Gist revisions
      they refer to, must remain usable evidence independently of it.

## 7. Updating the CLA later

A material change to the CLA is not just a repository edit.

- [ ] Edit [`CLA.md`](../../CLA.md), bump the version and effective date, and add a version-history
      entry.
- [ ] Update the Gist with the same text, producing a **new revision**.
- [ ] Record the new revision hash in the table in step 1.
- [ ] Confirm that CLA Assistant detects the new revision of the linked Gist. The service is linked
      to the Gist, not to a hand-picked revision — it follows the Gist and asks for a fresh signature
      on the next pull request. The recorded hash is for the maintainer's own audit trail.
- [ ] Re-run the step 3 test to verify that contributors who accepted the earlier revision are asked
      to sign again, and that the new acceptance is recorded against the new revision.
- [ ] If the service cannot enforce re-acceptance, record that as a limitation and handle
      re-acceptance manually before merging further external work.

## 8. Only then, enable external contribution merging

After — and only after — steps 3 to 6 are complete and verified. Steps 1 and 2 alone are not
sufficient; the current documentation state already reflects them.

- [ ] Replace the "configured but unverified" notice in [`CONTRIBUTING.md`](../../CONTRIBUTING.md)
      with the operational status.
- [ ] Leave [`CLA.md`](../../CLA.md) untouched — it carries no status by design, so the Gist stays
      byte-identical and nobody is asked to re-accept.
- [ ] Update the contribution paragraph in [`README.md`](../../README.md).
- [ ] Update the status note in [`docs/development/contributing.md`](../development/contributing.md).
- [ ] Update the summary line in [`.github/pull_request_template.md`](../../.github/pull_request_template.md).
- [ ] Update the status line at the top of this document.
- [ ] Close `TODO-LEGAL-001` in [`docs/development/todo-backlog.md`](../development/todo-backlog.md),
      and record the change in [`CHANGELOG.md`](../../CHANGELOG.md).

Do not perform step 8 partially. Contradictory contribution-status text across these files is worse
than a consistent statement that the workflow is not yet verified.

## Optional: professional legal review

Legal review is **not** a prerequisite for any step above, and is not required before accepting
external contributions. This is a hobby project, and the CLA is project policy written to be clear
rather than litigated.

> Consider professional legal review before entering material commercial licensing agreements,
> accepting substantial corporate contributions, assigning project rights, or relying on the CLA in a
> legal dispute.
