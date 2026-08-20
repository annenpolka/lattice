This directory intentionally contains no media.

`main.vel` references `../capture.mp4`, which is next to the VEL file rather
than in this directory. From the repository root, run
`./scripts/prepare-gameplay-commentary.ps1` to generate that ignored local
audio+video fixture explicitly.

Production render fails when referenced user media is missing. Tests generate
their own fixtures and do not rely on a silent test-source fallback.
