// THE one extension -> modality table (2026-11-01, review doc §13 / §17 P1.12).
//
// Shared by BOTH the host (src/types/modality.rs, via include!) and file_link
// (via #[path]) so the two can no longer disagree. Pure std, no dependencies,
// no inner doc comments (`//!`) so it can be include!'d into a module.
//
// Return values are the host's `ModalityType::path_name()` strings:
//   text code image audio video math chemistry dna eeg 3d sound biology
//   proteomics haptic thermal depth imu geospatial electromagnetic bci
//   parametric_cad kinematics control_systems network_topology radar sonar
//   hyperspectral uidocument
// plus FOUR documented METADATA-ONLY buckets that have NO modality pipeline and
// therefore get a file node but no graph (honest, never faked analysis):
//   "data"     csv/tsv and similar tabular files
//   "config"   env / ignore / editorconfig style dotfiles
//   "archive"  zip tar gz rar 7z bz2 xz
//   "document" xls xlsx ppt pptx (office files without a text extractor)
//
// COLLISION DECISIONS (each documented; first column wins everywhere):
//   ts          -> code   (MPEG-TS video is unreachable by extension on purpose:
//                          .ts is overwhelmingly TypeScript; a TS video can be
//                          forced by the caller, never guessed from the name)
//   svg         -> uidocument (an element tree with exact vector geometry; both
//                          entry paths — file_link and the host — now agree)
//   sdf         -> chemistry, UNLESS the path says urdf/robot/gazebo/sim/world
//                          (see modality_hint_for_path) -> kinematics
//   urdf mjcf bvh c3d osim kin dh -> kinematics (the host table used to shadow
//                          urdf/mjcf as 3d, making Kinematics unreachable)
//   world       -> 3d
//   pdb cif mol2 mae maegz -> chemistry (proteomics-only: mmcif mrc map dx ccp4
//                          xplor mzml mzxml wiff)
//   xyz         -> chemistry (depth point clouds: pcd las laz pts e57 zdf; ply->3d)
//   raw         -> image  (camera RAW; proteomics .raw is reachable by path hint)
//   tif tiff    -> image  (thermal / geospatial via path hint)
//   wav flac ogg opus aiff -> audio (sound reconstruction via path hint)
//   set         -> eeg    (bci via path hint); xdf bcilab -> bci
//   m           -> math   (MATLAB/Octave scripts; control models are slx mdl fmu
//                          fmi simulink)
//   h5 hdf5     -> haptic (host first-match order); nc -> geospatial;
//                          hdr bil bip bsq h4 hdf4 img -> hyperspectral
//   iq          -> electromagnetic (radar: cphd sicd nitf ntf rs2)
//   bin         -> NONE   (a bare .bin is not evidence of radar; the old host
//                          table's radar claim was wrong and is dropped)
//   json jsonl yaml yml toml xml ini conf cfg -> code  (the host's canonical
//                          meaning: structured text analyzed by the code pipeline)
//   csv         -> data   (metadata-only; IMU only via path hint)
//   html htm xhtml css scss sass less -> uidocument (pipeline 127)
//   pdf doc docx rtf odt ods odp pptx ppt -> text (via the document MCP; xls/xlsx stay metadata-only: numeric cells are not extracted)
//   (legacy)  pdf doc docx rtf odt -> text (the text pipeline extracts real text; scanned
//                          or encrypted files give an honest error, never text)
//   tex latex   -> math

// Map a file extension (case-insensitive, leading dot optional) to a modality
// key or a metadata-only bucket. None = unknown.
pub fn modality_for_extension(ext: &str) -> Option<&'static str> {
    let lower = ext.to_lowercase();
    let e = lower.trim_start_matches('.');
    Some(match e {
        // ── text ─────────────────────────────────────────────────────────────
        "txt" | "md" | "markdown" | "rst" | "adoc" | "org" | "pdf" | "doc" | "docx" | "rtf"
        | "odt" | "ods" | "odp" | "pptx" | "ppt" => "text",

        // ── code (incl. structured text the host treats as code) ─────────────
        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp"
        | "cc" | "h" | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "r" | "jl"
        | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "sql" | "graphql"
        | "vue" | "svelte" | "elm" | "clj" | "ex" | "exs" | "erl" | "hs" | "ml" | "fs" | "nim"
        | "zig" | "v" | "d" | "ada" | "pas" | "pl" | "pm" | "tcl" | "awk" | "sed" | "makefile"
        | "cmake" | "dockerfile" | "yaml" | "yml" | "toml" | "ini" | "conf" | "cfg" | "json"
        | "jsonl" | "xml" => "code",

        // ── ui documents (pipeline 127) ──────────────────────────────────────
        "html" | "htm" | "xhtml" | "css" | "scss" | "sass" | "less" | "svg" => "uidocument",

        // ── image ────────────────────────────────────────────────────────────
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "ico" | "heic"
        | "heif" | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "ai" | "eps" => "image",

        // ── audio ────────────────────────────────────────────────────────────
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "aiff" | "aif" | "opus"
        | "mid" | "midi" | "ra" | "ram" => "audio",

        // ── video ────────────────────────────────────────────────────────────
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "mpeg" | "mpg" | "3gp"
        | "ogv" | "mts" | "m2ts" => "video",

        // ── math ─────────────────────────────────────────────────────────────
        "tex" | "latex" | "nb" | "m" | "mpl" | "maple" | "mw" | "wxm" | "wxmx" | "sage"
        | "ipynb" | "rmd" | "qmd" => "math",

        // ── chemistry ────────────────────────────────────────────────────────
        "mol" | "sdf" | "pdb" | "xyz" | "cif" | "mol2" | "cml" | "cdx" | "cdxml" | "rxn"
        | "smi" | "smiles" | "inchi" | "ct" | "mae" | "maegz" => "chemistry",

        // ── dna ──────────────────────────────────────────────────────────────
        "fasta" | "fa" | "fna" | "ffn" | "faa" | "frn" | "fastq" | "fq" | "gbk" | "gb"
        | "genbank" | "gff" | "gff3" | "gtf" | "vcf" | "sam" | "bam" | "cram" | "bed" | "wig"
        | "bigwig" | "bw" | "2bit" | "nex" | "nexus" | "phy" | "phylip" | "aln" => "dna",

        // ── eeg / bci ────────────────────────────────────────────────────────
        "edf" | "bdf" | "gdf" | "set" | "fif" | "vhdr" | "vmrk" | "eeg" | "cnt" | "avg" | "mff"
        | "ncs" | "nev" | "ns2" | "ns3" | "ns4" | "ns5" | "ns6" | "plx" => "eeg",
        "xdf" | "bcilab" => "bci",

        // ── 3d ───────────────────────────────────────────────────────────────
        "blend" | "obj" | "glb" | "gltf" | "fbx" | "dae" | "stl" | "ply" | "usd" | "usda"
        | "usdc" | "usdz" | "3ds" | "c4d" | "ma" | "mb" | "world" => "3d",

        // ── parametric cad ───────────────────────────────────────────────────
        "step" | "stp" | "iges" | "igs" | "brep" | "occ" | "f3d" | "fcstd" | "sat" | "x_t"
        | "x_b" | "sldprt" | "sldasm" => "parametric_cad",

        // ── kinematics ───────────────────────────────────────────────────────
        "urdf" | "mjcf" | "bvh" | "c3d" | "osim" | "kin" | "dh" => "kinematics",

        // ── control systems ──────────────────────────────────────────────────
        "slx" | "mdl" | "simulink" | "fmu" | "fmi" => "control_systems",

        // ── sensors / signals ────────────────────────────────────────────────
        "rjpeg" | "fff" | "seq" | "csq" | "ir2" => "thermal",
        "pcd" | "las" | "laz" | "pts" | "e57" | "zdf" => "depth",
        "imu" | "bag" | "mcap" => "imu",
        "geojson" | "kml" | "kmz" | "shp" | "dbf" | "shx" | "prj" | "gpx" | "osm" | "pbf"
        | "dem" | "asc" | "grd" | "nc" => "geospatial",
        "iq" | "rf" | "sigmf" | "cf32" | "cs16" => "electromagnetic",
        "cphd" | "sicd" | "nitf" | "ntf" | "rs2" => "radar",
        "xtf" | "jsf" | "s7k" | "gsf" | "all" | "wcd" | "xse" => "sonar",
        "hdr" | "bil" | "bip" | "bsq" | "h4" | "hdf4" | "img" => "hyperspectral",
        "hdf5" | "h5" | "haptic" | "force" | "mat" => "haptic",
        "graphml" | "gexf" | "dot" | "gml" | "net" | "pajek" | "cyjs" => "network_topology",

        // ── biology / proteomics ─────────────────────────────────────────────
        "sbml" | "owl" | "obo" | "biopax" | "rxnml" | "hmdb" | "kegg" => "biology",
        "mmcif" | "mrc" | "map" | "dx" | "ccp4" | "xplor" | "mzml" | "mzxml" | "wiff" => {
            "proteomics"
        }

        // ── metadata-only buckets (no modality pipeline; honest) ─────────────
        "csv" | "tsv" => "data",
        "env" | "gitignore" | "dockerignore" | "editorconfig" => "config",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "archive",
        "xls" | "xlsx" => "document", // numeric cells are not extracted -> metadata-only (honest)

        _ => return None,
    })
}

// True for the four metadata-only buckets (no modality pipeline exists).
pub fn is_metadata_only_bucket(key: &str) -> bool {
    matches!(key, "data" | "config" | "archive" | "document")
}

// Path hints for the genuinely ambiguous extensions. WORD-BOUNDARY: the path is
// split into tokens on every non-alphanumeric character (so "/data/research/x"
// yields research, never "sar"), and a hint fires only on a whole-token match.
// Directory names count (a file under /sonar/ is sonar data) — but only as
// whole tokens, so "simulation" can never mean "imu".
pub fn modality_hint_for_path(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    let toks: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has = |w: &str| toks.iter().any(|t| *t == w);
    if has("radar") || has("sar") {
        return Some("radar");
    }
    if has("sonar") || has("bathymetry") || has("bathymetric") {
        return Some("sonar");
    }
    if has("thermal") || has("infrared") || has("flir") {
        return Some("thermal");
    }
    if has("hyperspectral") || has("multispectral") {
        return Some("hyperspectral");
    }
    if has("imu") || has("accelerometer") || has("gyroscope") {
        return Some("imu");
    }
    None
}

// sdf is chemistry by default; these tokens flip it to kinematics (Gazebo /
// Simulation Description Format files live under robot/sim style paths).
pub fn sdf_is_kinematics(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| matches!(t, "urdf" | "robot" | "robots" | "gazebo" | "mjcf" | "sim" | "world"))
}

// Full resolution: path hints first (only for hint-eligible extensions, never
// for text/code/uidocument/metadata files), then the extension table, then the
// sdf rule. None = unknown.
pub fn resolve_modality(path: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let base = modality_for_extension(ext);
    let hint_eligible = matches!(
        base,
        Some("image")
            | Some("audio")
            | Some("depth")
            | Some("haptic")
            | Some("data")
            | Some("hyperspectral")
            | Some("geospatial")
            | Some("eeg")
            | Some("imu")
            | Some("chemistry")
            | None
    );
    if hint_eligible {
        if let Some(h) = modality_hint_for_path(path) {
            return Some(h);
        }
    }
    if ext.eq_ignore_ascii_case("sdf") && sdf_is_kinematics(path) {
        return Some("kinematics");
    }
    base
}
