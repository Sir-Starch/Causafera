# Security Policy

## Supported versions

Causafera is experimental pre-alpha software. There is no stable or production-supported release
line. Security fixes are applied to the current `main` branch when maintainers accept them; older
commits and local forks are not supported versions.

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

Reports are handled on a best-effort basis. The project does not currently promise a response or
remediation deadline and does not operate a bug-bounty program. The maintainer will coordinate
disclosure when practical and may request additional evidence before classifying a report.

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
