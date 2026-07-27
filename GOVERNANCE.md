# Causafera Governance

Causafera is an independent, author-led free and open-source simulation engine. This document is the
authoritative statement of how the project is governed. Where any other repository document is less
specific about decision-making authority, this document governs.

## Maintainer authority

The project is maintained by the natural person who controlled GitHub user ID `281476371` on
**27 July 2026**, then using the login [`Sir-Starch`](https://github.com/Sir-Starch). That person
remains the maintainer under any later username.

The account is an evidentiary anchor to a specific date, not a transferable token. Authority follows
the person, not the account: a username change transfers nothing, a later holder of a released
`Sir-Starch` login is not the maintainer, someone who acquires or seizes control of the account does
not thereby acquire authority, and moving or forking the repository does not carry governance with
it. [CLA.md](CLA.md) anchors licensing rights to the same person on the same date.

The maintainer holds final authority over the canonical repository, currently at
`https://github.com/Sir-Starch/Causafera`, including:

- simulation philosophy and foundational assumptions;
- architecture and technical direction;
- project scope and roadmap sequencing;
- accepted behaviour and its canonical interpretation;
- releases, and the acceptance or rejection of contributions.

Causafera is not community-owned, community-governed, consensus-driven, or democratically governed.
There is no contributor vote, no steering committee, no seat earned by contribution volume, and no
commons administered collectively by contributors. The engine's source is public; its direction is
not shared.

The maintainer develops Causafera primarily to satisfy a specific vision of causal simulation. That
vision, not aggregate contributor preference, is the acceptance criterion.

## Contributions and governance rights

External contributors are welcome to propose ideas, report defects, discuss design, and submit
changes. Contributions are evaluated on whether they support the canonical vision and satisfy the
project's architectural, determinism, provenance, and evidence requirements.

Contributing does not automatically create governance or decision-making rights. Neither the number,
size, nor significance of accepted contributions confers proportional influence over direction —
there is no threshold of merged work that earns a say.

Authority over any part of the project exists only where the maintainer has **explicitly delegated
it**. Such delegation is scoped to what it says it covers, may be changed or revoked at any time,
and does not accumulate through continued contribution. Final authority remains with the maintainer
unless it is separately and explicitly transferred.

The maintainer may reject a contribution that is technically correct, well tested, and useful, on
the sole ground that it does not fit the project's vision. That is a normal outcome, not a defect in
the contribution or a judgement of the contributor. Where the reason is scope or philosophy rather
than quality, it will be stated as such.

To keep that outcome cheap for everyone, discuss substantial changes in an issue before implementing
them. See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow.

## Forks and alternative interpretations

Causafera is published under free and open-source licences precisely so that disagreement has a
constructive outlet. Anyone who wants materially different simulation behaviour, different
foundational assumptions, or a different roadmap is free to use, modify, and fork the engine under
those licences.

Forks and alternative interpretations are welcome and are a legitimate way to explore a design
disagreement. They do not redefine the canonical Causafera project, and the canonical repository is
under no obligation to adopt their conclusions.

## Why the engine is free and open source

Public source availability is not a growth strategy or a governance model here. It is a
methodological requirement. A simulation engine that makes claims about causality, determinism, and
emergence is only meaningful if others can:

- inspect the implementation independently of any statement made about it;
- reproduce simulation results from a stated seed, commit, and configuration;
- verify causal traces, provenance chains, and documented assumptions against the code;
- adapt the engine and run experiments the maintainer did not anticipate;
- avoid relying on unverifiable "trust the author" claims.

**Source availability is not validation.** That the code is readable does not make its results
correct, its abstractions physically justified, or its emergence claims real. Validation comes from
deterministic replay, tests, recorded provenance, documented assumptions, reproducible experiments,
and representative benchmarks — the evidence requirements described in
[CONTRIBUTING.md](CONTRIBUTING.md) and the
[architecture invariants](docs/architecture/invariants.md). Repository documentation must keep those
two things distinct and must not present openness as evidence of correctness.

Current maturity, and the gap between implemented contracts and validated depth, are tracked in the
[domain coverage matrix](docs/ontology/domain-coverage-matrix.md) and the
[roadmap](docs/roadmap/roadmap.md).

## Licensing, ownership, and sustainability

Six things are distinct and are deliberately not merged in project documentation:

| Concept | What it means here |
| --- | --- |
| Public FOSS licensing | Functional software material is under `AGPL-3.0-only`; prose and non-functional explanatory documentation are under `CC BY-SA 4.0` |
| Governance authority | The maintainer decides direction and acceptance; this document |
| Copyright ownership | Contributors retain copyright in their contributions; nothing here transfers it |
| CLA-granted rights | The additional licensing rights contributors grant the maintainer under [CLA.md](CLA.md) |
| Commercial licensing | Possible alternative outbound terms the CLA preserves, alongside — never instead of — the public licence granted to each release |
| Scientific or technical validation | Independent of all of the above; established by evidence, not by licence or authority |

The public engine is released under its applicable FOSS licences. The
[Contributor License Agreement](CLA.md) requires that every public release containing an accepted
contribution licenses it under the applicable public licence, and that a licence once granted cannot
later be revoked by commercial licensing.

That is a licensing commitment, not a promise to host files forever. The maintainer may modify,
replace, or remove a contribution in later versions, and is not obliged to keep any repository,
release, or distribution channel available indefinitely. Removing something from a later version
does not retroactively affect the licence granted to the releases that contained it.

The CLA additionally grants the maintainer sufficient rights to offer alternative commercial or
proprietary licensing in the future. Causafera represents years of unpaid work, and the project is
not intended to convert that work into an open-ended obligation governed by community expectations.
The maintainer may pursue commercial opportunities. Doing so cannot revoke or replace the FOSS
licence already granted to any public release.

## Conduct

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md). It sets behavioural
standards for people interacting in project spaces. It is not a governance mechanism and does not
grant decision-making authority.

## Related documents

| Document | Purpose |
| --- | --- |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow, architectural requirements, and validation |
| [CLA.md](CLA.md) | Terms under which contributions are accepted |
| [docs/legal/cla-service-setup.md](docs/legal/cla-service-setup.md) | Maintainer checklist for enabling CLA acceptance |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Behavioural standards in project spaces |
| [SECURITY.md](SECURITY.md) | Vulnerability reporting and known limitations |
| [SUPPORT.md](SUPPORT.md) | What the project supports and how to ask |
