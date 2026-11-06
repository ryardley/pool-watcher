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

wire.ts defines all request/response types with zod; server.ts parses via RequestSchema.safeParse; keys.ts validates the shareable key. Only production cast left is the intentional structural cast to the engine TokenDataGetter class (not a wire type).

