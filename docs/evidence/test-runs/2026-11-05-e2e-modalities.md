# e2e-modalities — 2026-11-05

host: http://127.0.0.1:50051 · duration 23s · PASS 46 · FAIL 0 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | M0 | fixture project created | ws=7802784893 proj=8061349143 |
| PASS | M-a.txt | a.txt → Text | got=Text |
| PASS | M-a.md | a.md → Text | got=Text |
| PASS | M-a.rst | a.rst → Text | got=Text |
| PASS | M-a.pdf | a.pdf → Text | got=Text |
| PASS | M-a.docx | a.docx → Text | got=Text |
| PASS | M-a.odt | a.odt → Text | got=Text |
| PASS | M-a.rtf | a.rtf → Text | got=Text |
| PASS | M-a.pptx | a.pptx → Text | got=Text |
| PASS | M-a.odp | a.odp → Text | got=Text |
| PASS | M-a.rs | a.rs → Code | got=Code |
| PASS | M-a.py | a.py → Code | got=Code |
| PASS | M-a.js | a.js → Code | got=Code |
| PASS | M-a.ts | a.ts → Code | got=Code |
| PASS | M-a.go | a.go → Code | got=Code |
| PASS | M-a.sql | a.sql → Code | got=Code |
| PASS | M-a.json | a.json → Code | got=Code |
| PASS | M-a.yaml | a.yaml → Code | got=Code |
| PASS | M-a.toml | a.toml → Code | got=Code |
| PASS | M-a.csv | a.csv → Data | got=Data |
| PASS | M-a.xlsx | a.xlsx → Document | got=Document |
| PASS | M-a.html | a.html → UiDocument | got=UiDocument |
| PASS | M-a.css | a.css → UiDocument | got=UiDocument |
| PASS | M-a.svg | a.svg → UiDocument | got=UiDocument |
| PASS | M-a.png | a.png → Image | got=Image |
| PASS | M-a.jpg | a.jpg → Image | got=Image |
| PASS | M-a.webp | a.webp → Image | got=Image |
| PASS | M-a.wav | a.wav → Audio | got=Audio |
| PASS | M-a.mp3 | a.mp3 → Audio | got=Audio |
| PASS | M-a.mp4 | a.mp4 → Video | got=Video |
| PASS | M-a.mkv | a.mkv → Video | got=Video |
| PASS | M-a.tex | a.tex → Math | got=Math |
| PASS | M-a.mol | a.mol → Chemistry | got=Chemistry |
| PASS | M-a.fasta | a.fasta → DNA | got=DNA |
| PASS | M-a.edf | a.edf → EEG | got=EEG |
| PASS | M-a.glb | a.glb → 3D | got=3D |
| PASS | M-a.stl | a.stl → 3D | got=3D |
| PASS | M-a.step | a.step → CAD | got=CAD |
| PASS | M-a.urdf | a.urdf → Kinematics | got=Kinematics |
| PASS | H-noext | hostile/unknown input "noext" degrades cleanly (no crash, explicit result) | "Unknown" |
| PASS | H-a.xyzabc | hostile/unknown input "a.xyzabc" degrades cleanly (no crash, explicit result) | "Unknown" |
| PASS | H-empty.txt | hostile/unknown input "empty.txt" degrades cleanly (no crash, explicit result) | "Text" |
| PASS | H-bin.txt | hostile/unknown input "bin.txt" degrades cleanly (no crash, explicit result) | "Text" |
| PASS | H-ünï cödé 文件.md | hostile/unknown input "ünï cödé 文件.md" degrades cleanly (no crash, explicit result) | "Text" |
| PASS | H-loopA | hostile/unknown input "loopA" degrades cleanly (no crash, explicit result) | "Unknown" |
| PASS | H-missing_file.txt | hostile/unknown input "missing_file.txt" degrades cleanly (no crash, explicit result) | "Text" |
