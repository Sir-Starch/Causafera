# CLA Service Setup — Maintainer Checklist

Status of the CLA acceptance service: **not configured.** External pull requests can be prepared and
discussed, but cannot be merged until every step below is complete and verified.

This document is the repository-side record of the manual work required to enable CLA acceptance.
Every step happens **outside this repository**, in GitHub settings and in the hosted
[CLA Assistant](https://cla-assistant.io/) service. Nothing here has been performed. Do not update
the contribution-status text in [`CONTRIBUTING.md`](../../CONTRIBUTING.md),
[`README.md`](../../README.md), [`docs/development/contributing.md`](../development/contributing.md),
or [`.github/pull_request_template.md`](../../.github/pull_request_template.md) until the workflow
has been tested end to end.

Related: [`CLA.md`](../../CLA.md), [`GOVERNANCE.md`](../../GOVERNANCE.md), and `TODO-LEGAL-001` in
[`docs/development/todo-backlog.md`](../development/todo-backlog.md).

## 0. Legal review first

- [ ] Have [`CLA.md`](../../CLA.md) version 1.1 and the electronic acceptance process reviewed by a
      Netherlands-qualified lawyer experienced in intellectual property and open-source licensing.
- [ ] Put the questions listed under "Questions reserved for legal review" at the end of
      [`CLA.md`](../../CLA.md) to that lawyer explicitly.
- [ ] Apply the resulting changes, bump the CLA version and effective date, and record the change in
      the version history.
- [ ] Record that the review happened, and when. Until then, no repository text may state or imply
      that the CLA has professional legal approval.

Steps 1–8 should not be treated as complete before this one is.

## 1. Publish an immutable CLA source

CLA Assistant records acceptance against a specific document. That document must not change silently
underneath contributors who have already accepted it.

- [ ] Publish the CLA text at a **public, versioned, immutable URL** — a permalink pinned to a commit
      SHA, a tagged release asset, or a Gist revision. A `main`-branch URL is not acceptable, because
      its content can change after acceptance.
- [ ] Confirm the URL renders the complete text, including the version number and effective date.
- [ ] Record the exact URL and the version it corresponds to below, once chosen.

| CLA version | Immutable source URL | Recorded |
| --- | --- | --- |
| 1.1 | _not yet published_ | — |

## 2. Connect the repository

- [ ] Sign in to CLA Assistant with the `Sir-Starch` GitHub account.
- [ ] Authorize the app against `Sir-Starch/Causafera` only, not the whole account, and review the
      permissions it requests before granting them.
- [ ] Create the CLA configuration pointing at the immutable URL from step 1.
- [ ] Confirm the resulting status check name, so it can be required in step 5.

## 3. Test with a real pull request

- [ ] Open a throwaway pull request from a **second** account that has never accepted the CLA. The
      maintainer account cannot test this path, because it will typically be treated as exempt.
- [ ] Confirm the CLA check appears and reports **failing/pending** before acceptance.
- [ ] Accept through the service as the test account.
- [ ] Confirm the check flips to **passing**, and that this happens without a new push.
- [ ] Confirm the acceptance record contains all five elements required by section 9 of
      [`CLA.md`](../../CLA.md): contributor identity, CLA version, timestamp, associated pull
      request, and accepting party.
- [ ] Close the test pull request without merging.

## 4. Verify the status check surfaces correctly

- [ ] Confirm the check is visible on the pull request page, not only in the service dashboard.
- [ ] Confirm a pull request from a contributor who has already accepted the current version passes
      without being asked again.
- [ ] Confirm that a contributor who accepted an **earlier** version is asked again after a material
      CLA version bump. If the service cannot enforce this, record it as a limitation and handle
      re-acceptance manually.

## 5. Require the check on `main`

- [ ] In branch protection or the repository ruleset for `main`, add the CLA status check to the
      **required** checks, alongside the existing `rust` and `ui` CI jobs.
- [ ] Confirm that a pull request without CLA acceptance is blocked from merging by the branch rule
      itself, not merely visually marked.
- [ ] Confirm the maintainer cannot merge past it by accident; decide deliberately whether
      administrators are included in the restriction.

## 6. Handle bots and exempt accounts

Automation accounts cannot meaningfully accept a CLA, and their pull requests must not be blocked
forever by a check they can never satisfy.

- [ ] Add `dependabot[bot]` to the service's allowlist, if Dependabot is enabled.
- [ ] Add any other automation account that opens pull requests against this repository.
- [ ] Verify with a real bot pull request that the check passes rather than hangs.
- [ ] Keep the allowlist minimal and review it whenever a new automation is added — an allowlisted
      identity is an identity whose contributions are merged with no CLA record at all.

## 7. Export and retain acceptance records

- [ ] Export the acceptance records from the service after the first real acceptance.
- [ ] Store them privately, outside this repository and outside the service, in a durable location
      with a backup. Do not publish them; they contain contributor identity data.
- [ ] Set a recurring reminder to re-export, so the records survive the service becoming unavailable
      or changing terms.
- [ ] Establish what happens if CLA Assistant is discontinued: the records must remain usable
      evidence independently of it.
- [ ] Handle the records consistently with applicable data-protection obligations. Whether GDPR
      duties apply to this retention, and in what form, is one of the questions for legal review.

## 8. Only then, update repository status text

After — and only after — steps 0–7 are complete and verified:

- [ ] Replace the "preparing" notice in [`CONTRIBUTING.md`](../../CONTRIBUTING.md) with the
      operational status.
- [ ] Update the contribution paragraph in [`README.md`](../../README.md).
- [ ] Update the status note in [`docs/development/contributing.md`](../development/contributing.md).
- [ ] Update the summary line in [`.github/pull_request_template.md`](../../.github/pull_request_template.md).
- [ ] Update the status line at the top of this document.
- [ ] Close `TODO-LEGAL-001` in [`docs/development/todo-backlog.md`](../development/todo-backlog.md),
      and record the change in [`CHANGELOG.md`](../../CHANGELOG.md).

Do not perform step 8 partially. Contradictory contribution-status text across these files is worse
than a stale but consistent "not yet configured".
