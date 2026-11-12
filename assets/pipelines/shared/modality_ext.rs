// THE one extension -> modality table (2026-11-01, review doc §13 / §17 P1.12).
//
// ACCEPTANCE == READABILITY (operator 2026-11-11: "there should be NO unreadable files, because only accepted modalities/extensions link at
// all"). An extension is in this table ONLY if src/orchestrator/file_reader.rs can turn it into its modality pipeline's input without inventing
// a parameter the file does not state. Everything else is NOT ACCEPTED: `accept()` rejects it AT LINK TIME with the exact reason, and the graph
// worker never sees it. The unit test in file_reader.rs asserts every extension below prepares without error. Extensions that were in the old
// table but have no reader are listed (with the reason) in `rejection_reason`; extensions whose files are plain text with no dedicated reader
// (cif mol2 cml rxn, gff vcf bed, graphml dot, sbml owl, csv env …) are accepted AS TEXT — structure-agnostic, nothing invented.
//
// Shared by BOTH the host (src/types/modality.rs, via include!) and file_link
// (via #[path]) so the two can no longer disagree. Pure std, no dependencies,
// no inner doc comments (`//!`) so it can be include!'d into a module.
//
// Return values are the host's `ModalityType::path_name()` strings:
//   text code image audio video math chemistry dna eeg 3d proteomics thermal depth
//   geospatial parametric_cad kinematics control_systems
// (sound biology haptic imu electromagnetic bci network_topology radar sonar hyperspectral uidocument have NO accepted extension: their files
// need parameters they do not state, or are read as text/code — see `rejection_reason`.)
//
// COLLISION DECISIONS (first column wins everywhere):
//   ts          -> code   (MPEG-TS video is unreachable by extension on purpose: .ts is overwhelmingly TypeScript)
//   svg html htm xhtml css scss sass less -> code (read as source text; the pipeline-127 library has no binary)
//   sdf         -> chemistry, UNLESS the path says urdf/robot/gazebo/sim/world -> kinematics
//   raw         -> image (camera RAW)      tif tiff -> image
//   m           -> math   (MATLAB/Octave scripts)
//   json jsonl yaml yml toml xml ini conf cfg -> code
//   csv tsv env gitignore dockerignore editorconfig -> text
//   pdf doc docx rtf odt ods odp pptx ppt -> text (extracted by the document tool through the text pipeline; a missing tool is a real failure
//                          that pauses and retries, never empty text)
//   tex latex   -> math

// Map a file extension (case-insensitive, leading dot optional) to a modality
// key. None = NOT ACCEPTED (see `rejection_reason`).
pub fn modality_for_extension(ext: &str) -> Option<&'static str> {
    let lower = ext.to_lowercase();
    let e = lower.trim_start_matches('.');
    Some(match e {
        // ── text: documents (binary ones are extracted by the document tool through the text pipeline), plain text, and text formats with no dedicated reader
        "txt" | "md" | "markdown" | "rst" | "adoc" | "org" | "pdf" | "doc" | "docx" | "rtf"
        | "odt" | "ods" | "odp" | "pptx" | "ppt"
        | "csv" | "tsv" | "env" | "gitignore" | "dockerignore" | "editorconfig"
        | "cif" | "mol2" | "cml" | "cdxml" | "rxn" | "ct"
        | "gff" | "gff3" | "gtf" | "vcf" | "sam" | "bed" | "wig" | "nex" | "nexus" | "phy" | "phylip" | "aln"
        | "graphml" | "gexf" | "dot" | "gml" | "net" | "pajek" | "cyjs"
        | "sbml" | "owl" | "obo" | "biopax" | "rxnml" | "hmdb" | "kegg" => "text",

        // ── code (incl. structured text the host treats as code, and ui documents read as source text)
        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp"
        | "cc" | "h" | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "r" | "jl"
        | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "sql" | "graphql"
        | "vue" | "svelte" | "elm" | "clj" | "ex" | "exs" | "erl" | "hs" | "ml" | "fs" | "nim"
        | "zig" | "v" | "d" | "ada" | "pas" | "pl" | "pm" | "tcl" | "awk" | "sed" | "makefile"
        | "cmake" | "dockerfile" | "yaml" | "yml" | "toml" | "ini" | "conf" | "cfg" | "json"
        | "jsonl" | "xml"
        | "html" | "htm" | "xhtml" | "css" | "scss" | "sass" | "less" | "svg" => "code",

        // ── image / audio / video: the pipeline opens the file by path
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "ico" | "heic"
        | "heif" | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "ai" | "eps" => "image",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "aiff" | "aif" | "opus"
        | "mid" | "midi" | "ra" | "ram" => "audio",
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "mpeg" | "mpg" | "3gp"
        | "ogv" | "mts" | "m2ts" => "video",

        // ── math
        "tex" | "latex" | "nb" | "m" | "mpl" | "maple" | "mw" | "wxm" | "wxmx" | "sage"
        | "ipynb" | "rmd" | "qmd" => "math",

        // ── chemistry (MolFile / PDB / XYZ / SMILES / InChI content)
        "mol" | "sdf" | "pdb" | "xyz" | "smi" | "smiles" | "inchi" => "chemistry",

        // ── dna (sequence files the pipeline reads by path)
        "fasta" | "fa" | "fna" | "ffn" | "faa" | "frn" | "fastq" | "fq" | "gbk" | "gb"
        | "genbank" => "dna",

        // ── eeg (the pipeline opens the file by path and sniffs the format)
        "edf" | "bdf" | "gdf" | "set" | "fif" | "vhdr" | "vmrk" | "eeg" | "cnt" | "avg" | "mff"
        | "ncs" | "nev" | "ns2" | "ns3" | "ns4" | "ns5" | "ns6" | "plx" => "eeg",

        // ── 3d: the five formats the 3D pipeline reads
        "obj" | "glb" | "gltf" | "stl" | "ply" => "3d",

        // ── parametric cad / kinematics / control / thermal / depth / geospatial / proteomics: self-describing sources only
        "step" | "stp" | "iges" | "igs" | "brep" | "occ" | "f3d" | "fcstd" | "sat" | "x_t"
        | "x_b" | "sldprt" | "sldasm" => "parametric_cad",
        "urdf" | "mjcf" | "bvh" | "c3d" | "osim" => "kinematics",
        "slx" | "mdl" | "simulink" => "control_systems",
        "rjpeg" | "fff" => "thermal",
        "pcd" | "las" | "laz" => "depth",
        "geojson" | "kml" | "kmz" | "shp" | "gpx" | "osm" | "pbf" => "geospatial",
        "mmcif" => "proteomics",

        _ => return None,
    })
}

// The four old metadata-only buckets no longer exist (csv/tsv/env/… are read as text; archives and spreadsheets are not accepted). Kept so the
// host's `from_extension` / `from_path` call sites keep compiling; always false.
pub fn is_metadata_only_bucket(_key: &str) -> bool {
    false
}

// WHY a file's extension is not accepted (the exact reason shown to the caller / UI / MCP at link time).
pub fn rejection_reason(ext: &str) -> String {
    let lower = ext.to_lowercase();
    let e = lower.trim_start_matches('.');
    let why = match e {
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "an archive has no reader: its contents would have to be unpacked first, and nothing unpacks it",
        "xls" | "xlsx" => "spreadsheet cells are numeric and no tool extracts them",
        "bam" | "cram" | "bigwig" | "bw" | "2bit" => "a binary genomics container: no reader takes it (text formats like fasta / fastq / genbank are accepted)",
        "cdx" | "mae" | "maegz" => "a binary chemistry container: the chemistry pipeline takes MolFile / PDB / XYZ / SMILES / InChI content",
        "xdf" | "bcilab" => "BCI streams need parameters the file does not state (marker channel, paradigm, sample rate); a reader would have to invent them",
        "blend" | "fbx" | "dae" | "usd" | "usda" | "usdc" | "usdz" | "3ds" | "c4d" | "ma" | "mb" | "world" => "the 3D pipeline reads glTF / GLB / OBJ / STL / PLY only",
        "kin" | "dh" => "a DH table is a parameter list, not a chain file; the kinematics pipeline takes URDF / MJCF / BVH / C3D / OSIM (and SDF)",
        "fmu" | "fmi" => "a control model needs the model name or system variable, which the file does not state; only Simulink models are self-describing",
        "seq" | "csq" | "ir2" => "thermal video needs frame rate and radiometric coefficients the file does not state; only radiometric FLIR stills (rjpeg / fff) are self-describing",
        "pts" | "e57" | "zdf" => "depth scans need scan count, delimiter / columns or camera intrinsics the file does not state; only LAS / LAZ / PCD carry their own header",
        "dbf" | "shx" | "prj" => "a shapefile sidecar: link the .shp, which carries it",
        "dem" | "asc" | "grd" | "nc" => "a raster / grid needs a vertical datum, CRS or variable name the file does not state",
        "mrc" | "map" | "dx" | "ccp4" | "xplor" | "mzml" | "mzxml" | "wiff" => "proteomics data needs the mass-spec format and experiment type or the map / density metadata, which the file does not state",
        "imu" | "bag" | "mcap" => "IMU data needs a column map and units, the ROS topic, or the sample rate, which the file does not state",
        "iq" | "rf" | "sigmf" | "cf32" | "cs16" => "electromagnetic captures need sample rate, centre frequency and gain, which the file does not state",
        "cphd" | "sicd" | "nitf" | "ntf" | "rs2" => "radar data needs sample rate / carrier frequency or the SAR format, polarization and orbit, which the file does not state",
        "xtf" | "jsf" | "s7k" | "gsf" | "all" | "wcd" | "xse" => "sonar data needs format, frequency, beam count and swath, which the file does not state",
        "hdr" | "bil" | "bip" | "bsq" | "h4" | "hdf4" | "img" => "hyperspectral cubes need band count, wavelength range and resolution, which the file does not state",
        "hdf5" | "h5" | "haptic" | "force" | "mat" => "haptic data needs sensor model, array geometry and sample rate, which the file does not state",
        "" => "the file has no extension, so no reader can be chosen",
        _ => "no reader exists for this extension",
    };
    format!(".{e}: not accepted — {why}")
}

// Full resolution: the extension table, then the sdf rule (sdf is chemistry by default; robot / sim style paths make it kinematics).
// PATH HINTS no longer pick a modality: every hinted modality (radar sonar imu hyperspectral thermal-video) is one whose files need parameters
// the file does not state, so a hint could only route a file to a pipeline that has no reader for it. A hint is applied only when the table
// itself already accepts that extension for the hinted modality (i.e. never changes the result).
pub fn resolve_modality(path: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let base = modality_for_extension(ext);
    if let Some(h) = modality_hint_for_path(path) {
        if base == Some(h) {
            return base;
        }
    }
    if ext.eq_ignore_ascii_case("sdf") && sdf_is_kinematics(path) {
        return Some("kinematics");
    }
    base
}

// Path hints for the genuinely ambiguous extensions (whole-token match; directory names count).
pub fn modality_hint_for_path(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    let toks: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).collect();
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

pub fn sdf_is_kinematics(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| matches!(t, "urdf" | "robot" | "robots" | "gazebo" | "mjcf" | "sim" | "world"))
}

// LINK-TIME ACCEPTANCE (the one gate; FileLink and /orchestrate attached files call it). Ok(modality key) = the file is accepted AND well-formed
// for that modality. Err(reason) = rejected up front with the exact reason — never a silent skip, never a later worker-side "unreadable".
pub fn accept(path: &str) -> Result<&'static str, String> {
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(key) = resolve_modality(path) else { return Err(rejection_reason(ext)) };
    validate_file(path, key, &ext.to_ascii_lowercase())?;
    Ok(key)
}

// Well-formedness of an ACCEPTED file, checked from the bytes (streamed — no size ceiling, nothing loaded whole).
pub fn validate_file(path: &str, key: &str, ext: &str) -> Result<(), String> {
    use std::io::Read;
    let meta = std::fs::metadata(path).map_err(|e| format!("{path}: cannot be read: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{path}: not a regular file"));
    }
    let mut f = std::fs::File::open(path).map_err(|e| format!("{path}: cannot be opened: {e}"))?;
    let binary_document = matches!(ext, "pdf" | "doc" | "docx" | "rtf" | "odt" | "ods" | "odp" | "pptx" | "ppt");
    let text_shaped = !binary_document && matches!(key, "text" | "code" | "math" | "chemistry");
    if text_shaped {
        // streamed UTF-8 validation: a chunk boundary may split a multi-byte character, so the incomplete tail is carried into the next read
        let mut buf = vec![0u8; 64 * 1024];
        let mut carry: Vec<u8> = Vec::new();
        let mut offset: u64 = 0;
        loop {
            let n = f.read(&mut buf).map_err(|e| format!("{path}: read failed at byte {offset}: {e}"))?;
            if n == 0 {
                if !carry.is_empty() {
                    return Err(format!("{path}: not valid UTF-8 text (truncated character at the end of the file)"));
                }
                return Ok(());
            }
            let mut data = std::mem::take(&mut carry);
            data.extend_from_slice(&buf[..n]);
            match std::str::from_utf8(&data) {
                Ok(_) => {}
                Err(e) if e.error_len().is_none() => carry = data[e.valid_up_to()..].to_vec(),
                Err(e) => return Err(format!("{path}: not valid UTF-8 text (invalid byte near offset {})", offset + e.valid_up_to() as u64)),
            }
            offset += n as u64;
        }
    }
    if key == "depth" {
        let mut head = vec![0u8; 4096];
        let n = f.read(&mut head).map_err(|e| format!("{path}: header not readable: {e}"))?;
        head.truncate(n);
        return match ext {
            "las" | "laz" if head.len() < 105 || &head[0..4] != b"LASF" => Err(format!("{path}: not a LAS/LAZ file — the LASF signature is missing")),
            "pcd" if !String::from_utf8_lossy(&head).lines().any(|l| l.trim().starts_with("DATA")) => Err(format!("{path}: malformed PCD — the header has no DATA line")),
            _ => Ok(()),
        };
    }
    Ok(())
}
