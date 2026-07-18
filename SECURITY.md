# Security Policy

## Supported versions

Causafera is **Experimental pre-alpha** software. There is no stable or production-supported release
line, supported binary distribution, or operated production service. Security fixes are applied to
the current `main` branch when maintainers accept them; older commits and local forks are not
supported versions.

## Known pre-alpha limitations

The locked desktop-observer dependency graph currently includes known advisories in
`quick-xml 0.38.4` (RUSTSEC-2026-0194 and RUSTSEC-2026-0195 denial-of-service cases),
`time 0.3.45` (RUSTSEC-2026-0009 stack exhaustion), `serde_with 3.17.0`
(GHSA-7gcf-g7xr-8hxj serialization panic), and `glib 0.18.5` (RUSTSEC-2024-0429
unsoundness), plus unmaintained transitive packages. These are non-blocking for source visibility
because no supported binary is distributed. They should be resolved before a supported binary
distribution and reassessed before production use.

Snapshot persistence currently reads the input file before applying the decoder's 256 MiB bound,
and its predictable sibling temporary filename can follow a pre-existing symlink. Public source
visibility does not expose a running persistence service, but untrusted snapshot input and shared
write directories are outside the current threat model. Bound the pre-decode read and use a
no-follow, exclusive temporary-file strategy before untrusted or production use.

## Reporting a vulnerability

Report suspected vulnerabilities privately to **starch@velx.cc** with a subject beginning
`Causafera security`. This is an existing maintainer-controlled address.

Do not open a public issue for an undisclosed vulnerability. Do not include live credentials,
personal data, or exploit traffic against systems you do not own. Include only what is needed to
reproduce and assess the problem:

- affected commit SHA and component;
- impact and realistic attack preconditions;
- minimal reproduction steps or proof of concept;
- whether the issue is already public or actively exploited;
- a safe way to contact you for follow-up.

While active development continues, security reports are handled on a best-effort basis without
a bug-bounty program. The project does not currently promise a specific response time or
remediation deadline. The maintainer will coordinate disclosure when practical and may request
additional evidence before classifying a report.

## Scope

Security reports may cover source code, the CLI, snapshot handling, observer protocol and desktop
application, build and release automation, dependency or supply-chain risk, and accidental exposure
of secrets or private data in repository history.

Simulation-model disagreements, unsupported emergence claims, ordinary bugs without a security
impact, and feature requests belong in the issue tracker.

## Repository security controls

Contributors and maintainers should:

- never commit credentials or private environment files;
- use least-privilege GitHub Actions permissions and immutable action commit pins;
- review dependency advisories and license reports;
- run a full-history secret scanner before changing repository visibility;
- revoke a credible exposed credential before discussing history remediation;
- keep vulnerability details private until a fix or coordinated disclosure decision exists.

GitHub secret scanning, push protection, private vulnerability reporting, Dependabot alerts, and
branch rules are repository settings. Their configuration is not implied by the presence of this
file and must be verified separately before public release.
