Media files live here.

`main.vel` references `capture.mp4` next to the VEL file. Production render
fails with a diagnostic if that file is missing. Tests generate an explicit
audio+video fixture when they need one; they do not rely on a silent testsrc
fallback.
