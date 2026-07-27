# CLA Service Setup — Maintainer Checklist

Status of the CLA acceptance service: **not configured.** External pull requests can be prepared and
discussed, but cannot be merged until every step below is complete and verified.

This document is the repository-side record of the manual work required to enable CLA acceptance.
Every step happens **outside this repository**, in a GitHub Gist, in GitHub settings, and in the
hosted [CLA Assistant](https://cla-assistant.io/) service. Nothing here has been performed. Do not
update the contribution-status text in [`CONTRIBUTING.md`](../../CONTRIBUTING.md),
[`README.md`](../../README.md), [`docs/development/contributing.md`](../development/contributing.md),
[`CLA.md`](../../CLA.md), or
[`.github/pull_request_template.md`](../../.github/pull_request_template.md) until the workflow has
been tested end to end.

Related: [`CLA.md`](../../CLA.md), [`GOVERNANCE.md`](../../GOVERNANCE.md), and `TODO-LEGAL-001` in
[`docs/development/todo-backlog.md`](../development/todo-backlog.md).

## 1. Publish the CLA as a public Gist

CLA Assistant reads the agreement text from a **GitHub Gist** and tracks acceptance against a
specific **Gist revision**. This is the mechanism the service actually supports; release assets,
arbitrary commit permalinks, and other generic immutable URLs are not alternatives here.

- [ ] Create a **public** Gist containing the exact text of [`CLA.md`](../../CLA.md).
- [ ] Confirm the Gist content is byte-identical to the repository copy — the Gist is what
      contributors legally accept, and a drift between the two means people accepted something other
      than what the repository documents.
- [ ] Note the Gist URL and the **specific revision hash** shown under the Gist's Revisions tab.
- [ ] Record both in the table below.

| CLA version | Gist URL | Gist revision | Recorded |
| --- | --- | --- | --- |
| 1.1 | _not yet published_ | _not yet published_ | — |

Editing the Gist creates a **new revision**; it does not rewrite the old one. That is what makes
versioned acceptance work, and why the revision hash — not just the URL — must be recorded.

## 2. Connect the repository

- [ ] Sign in to CLA Assistant with the maintainer GitHub account (user ID `281476371`).
- [ ] Authorize the app against `Sir-Starch/Causafera` only, not the whole account, and review the
      permissions it requests before granting them.
- [ ] Create the CLA configuration pointing at the Gist from step 1.
- [ ] Confirm the resulting status check name, so it can be required in step 4.

## 3. Test with a real pull request

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
- [ ] Point the CLA Assistant configuration at the new revision.
- [ ] Re-run the step 3 test to confirm that a contributor who accepted an earlier revision is asked
      again, and that a new acceptance is recorded against the new revision.
- [ ] If the service cannot enforce re-acceptance, record that as a limitation and handle
      re-acceptance manually before merging further external work.

## 8. Only then, update repository status text

After — and only after — steps 1–6 are complete and verified:

- [ ] Replace the "preparing" notice in [`CONTRIBUTING.md`](../../CONTRIBUTING.md) with the
      operational status.
- [ ] Remove the "acceptance workflow not yet enabled" status note from [`CLA.md`](../../CLA.md).
- [ ] Update the contribution paragraph in [`README.md`](../../README.md).
- [ ] Update the status note in [`docs/development/contributing.md`](../development/contributing.md).
- [ ] Update the summary line in [`.github/pull_request_template.md`](../../.github/pull_request_template.md).
- [ ] Update the status line at the top of this document.
- [ ] Close `TODO-LEGAL-001` in [`docs/development/todo-backlog.md`](../development/todo-backlog.md),
      and record the change in [`CHANGELOG.md`](../../CHANGELOG.md).

Do not perform step 8 partially. Contradictory contribution-status text across these files is worse
than a stale but consistent "not yet configured".

## Optional: professional legal review

Legal review is **not** a prerequisite for any step above, and is not required before accepting
external contributions. This is a hobby project, and the CLA is project policy written to be clear
rather than litigated.

> Consider professional legal review before entering material commercial licensing agreements,
> accepting substantial corporate contributions, assigning project rights, or relying on the CLA in a
> legal dispute.
