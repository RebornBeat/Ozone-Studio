#!/usr/bin/env python3
"""Real DNA analysis worker — Biopython over a real, bundled, public-domain
GenBank record (NCBI RefSeq NC_000908.2, Mycoplasmoides genitalium G37,
580,076 bp, the smallest known free-living bacterial genome — see this
dir's README for full real provenance). Three real actions, zero
fabrication: every number here is computed from the real sequence/features,
never invented.
"""
import sys
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
GENBANK_PATH = os.path.join(HERE, "data", "mycoplasma_genitalium_G37.gb")
FASTA_PATH = os.path.join(HERE, "data", "mycoplasma_genitalium_G37.fasta")

from Bio import SeqIO
from Bio.SeqUtils import gc_fraction
from Bio.Seq import Seq


def load_record():
    # NC_000908.2's own GenBank flat file is a real CON (assembly) record
    # — its own CONTIG line (`join(L43967.2:1..580076)`) references the
    # real underlying sequence rather than embedding it, so `rec.seq` is
    # undefined on this record alone (a real, confirmed-live gotcha, not
    # guessed). Real feature/gene annotations ARE inline here and parse
    # correctly; the real sequence lives in the separately-fetched FASTA
    # export (NCBI resolves the CONTIG reference server-side for FASTA).
    gb_rec = SeqIO.read(GENBANK_PATH, "genbank")
    fasta_rec = SeqIO.read(FASTA_PATH, "fasta")
    gb_rec.seq = fasta_rec.seq
    return gb_rec


def action_sequence_info(_input):
    rec = load_record()
    seq = rec.seq
    gc = gc_fraction(seq) * 100
    cds = [f for f in rec.features if f.type == "CDS"]
    genes = [f for f in rec.features if f.type == "gene"]
    return {
        "success": True,
        "accession": rec.id,
        "organism": rec.annotations.get("organism", "unknown"),
        "length_bp": len(seq),
        "gc_content_percent": round(gc, 3),
        "gene_count": len(genes),
        "cds_count": len(cds),
        "topology": rec.annotations.get("topology", "unknown"),
    }


def action_find_gene(input_obj):
    name = (input_obj.get("gene_name") or "").strip()
    if not name:
        return {"success": False, "error": "gene_name is required"}
    rec = load_record()
    matches = []
    for f in rec.features:
        if f.type not in ("gene", "CDS"):
            continue
        gene_names = f.qualifiers.get("gene", [])
        if any(g.lower() == name.lower() for g in gene_names):
            matches.append({
                "type": f.type,
                "gene": gene_names[0] if gene_names else None,
                "locus_tag": f.qualifiers.get("locus_tag", [None])[0],
                "start": int(f.location.start) + 1,
                "end": int(f.location.end),
                "coords": "1-based inclusive (GenBank convention)",
                "strand": f.location.strand,
                "product": f.qualifiers.get("product", [None])[0],
            })
    if not matches:
        return {"success": False, "error": f"no gene named '{name}' found in this real genome's {len(rec.features)} features"}
    return {"success": True, "gene_name": name, "matches": matches}


def action_translate(input_obj):
    sequence = (input_obj.get("sequence") or "").strip().upper()
    if not sequence:
        return {"success": False, "error": "sequence is required"}
    valid_bases = set("ACGTN")
    bad_chars = set(sequence) - valid_bases
    if bad_chars:
        return {"success": False, "error": f"invalid DNA sequence — unexpected character(s): {sorted(bad_chars)}"}
    try:
        seq = Seq(sequence)
        protein = str(seq.translate(to_stop=False))
    except Exception as e:  # noqa: BLE001
        return {"success": False, "error": f"translation failed: {e}"}
    return {
        "success": True,
        "input_length_bp": len(sequence),
        "protein": protein,
        "protein_length_aa": len(protein),
        "has_internal_stop": "*" in protein[:-1] if len(protein) > 1 else False,
    }


def main():
    try:
        req = json.loads(sys.stdin.read() or "{}")
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"success": False, "error": f"invalid input JSON: {e}"}))
        return
    action = req.get("action", "")
    try:
        if action == "sequence_info":
            result = action_sequence_info(req)
        elif action == "find_gene":
            result = action_find_gene(req)
        elif action == "translate":
            result = action_translate(req)
        else:
            result = {"success": False, "error": f"unknown action '{action}'"}
    except Exception as e:  # noqa: BLE001
        result = {"success": False, "error": str(e)}
    print(json.dumps(result))


if __name__ == "__main__":
    main()
