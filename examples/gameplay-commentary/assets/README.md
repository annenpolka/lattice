Media files live here.

`main.vel` references `capture.mp4` next to the VEL file. If that file is missing,
`lattice render` generates a deterministic `testsrc` clip (21s, 10 fps) beside the
output. Tests use that generated fixture instead of real gameplay.
