# Codex app-server protocol fixtures — 0.144.5

These files pin the smallest protocol surface needed by Codex Reserve T102.

- Source CLI: `codex-cli 0.144.5`
- Generated on: 2026-07-20
- Generator: `codex app-server generate-json-schema --experimental --out <temporary-directory>`
- Selected methods: `initialize`, `account/read`, `account/rateLimits/read`, and `account/usage/read`

The files under `schemas/` are copied unchanged from the local CLI generator. The request and
response fixtures are synthetic examples built from those schemas; they contain no real account,
authentication, usage, or filesystem data. Values such as `fixture@example.invalid` and
`/fixture/codex-home` are deliberately non-real.

Some compatibility fixtures intentionally exercise inputs beyond the current enum definitions,
such as a future plan name, as well as omitted optional fields and nullable Credits. These cases
protect the app from failing closed when a newer CLI adds fields or enum values.
