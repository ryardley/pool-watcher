---
id: pw-m6k3
status: closed
deps: []
links: []
created: 2026-11-06T15:45:27Z
type: task
priority: 2
tags: [scanner]
---
# scanner: zod wire types + no as casts in production code

## Description

`wire.ts` defines all request/response types with zod; `server.ts` parses via
`RequestSchema.safeParse`; `keys.ts` validates the shareable key. The only
production cast left is the intentional structural cast to the engine's
`TokenDataGetter` class (a type-system boundary, not a wire type).

## Acceptance Criteria

- All wire types in `wire.ts` are zod schemas; no `as` casts in production
  code except the documented `TokenDataGetter` structural cast.
- `npm run build` and `npm test` pass in `scanner/`.

## References

- `scanner/src/wire.ts`, `scanner/src/server.ts`, `scanner/src/keys.ts`,
  `scanner/src/token-data-getter.ts`

