---
id: pw-fwdg
status: open
deps: [pw-zooo]
links: []
created: 2026-11-06T15:44:51Z
type: task
priority: 2
parent: pw-mn26
tags: [docker]
---
# docker: repo-root docker-compose.yml

Services: scanner-b (scanner/Dockerfile, KEY_SERVICE=sealedbox, SCANNER_SOCKET=/run/scanner/scanner-b.sock, socket volume), app (app/Dockerfile, RPC_URL/WS_URL=host.docker.internal:8545, CHAIN_ID=31337, socket + .state:ro + ledger volumes, port 8080, extra_hosts host-gateway), fava (python:3-slim + pip install fava, waits for the ledger file, port 5000).

