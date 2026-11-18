#!/usr/bin/env python3
"""Extract the Microsoft Ignite template artwork, survey slide and Segoe UI fonts for the stage.

The artwork, survey QR code and fonts are licensed to Microsoft and are not in this public
repository. Run this once on the presenting machine; it writes them to ui/public/template/
(git-ignored), which the web app serves at /template/. Without them the stage still works: it
uses the template's solid Dark Blue background and the system font, and the presenter desk
says the template assets are missing.

    python3 ops/template-assets.py [--template FILE.potx] [--survey FILE.pptx] [--fonts DIR] [--out DIR]

The walk-in slide is rendered from the template with LibreOffice (soffice) and PyMuPDF. If
either is missing, the walk-in falls back to its background artwork only, and this script
says so.
"""

import argparse
import hashlib
import json
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

DEMO = Path(__file__).resolve().parent.parent
HOME = Path.home()
DEFAULTS = {
    "template": HOME / "Downloads/Microsoft Ignite FY27_16-9 Event-template_v05.potx",
    "survey": HOME / "Downloads/Session Survey Slides.pptx",
    "fonts": Path("/Applications/Microsoft PowerPoint.app/Contents/Resources/sdx/FA000000002/OfflineFiles"),
    "out": DEMO / "ui/public/template",
}
# Segoe UI weights, as PowerPoint caches them (segoeui-<weight>_<hash>.woff).
FONTS = {"regular": "segoeui.woff", "semibold": "segoeui-semibold.woff",
         "semilight": "segoeui-semilight.woff", "light": "segoeui-light.woff"}
# What the stage needs; the presenter desk checks these against manifest.json.
REQUIRED = ["walkin.svg", "title-bg.jpg", "survey-bg.jpg", "survey-frame.svg",
            "survey-qr.png", "segoeui.woff", "segoeui-semibold.woff"]


class Package:
    """A .potx/.pptx file: resolves layouts, slides and their images through relationships."""

    def __init__(self, path):
        self.path = path
        self.zip = zipfile.ZipFile(path)

    def read(self, name):
        return self.zip.read(name).decode("utf8")

    def rels(self, part):
        folder, name = posixpath.split(part)
        rels = f"{folder}/_rels/{name}.rels"
        result = {}
        for rid, target in re.findall(r'Id="(rId\d+)"[^>]*?Target="([^"]+)"', self.read(rels)):
            result[rid] = posixpath.normpath(posixpath.join(folder, target))
        return result

    def layout(self, name, master=2):
        """The slide layout with this name, in the given master (2 is the template's dark master)."""
        for part in sorted(n for n in self.zip.namelist() if re.fullmatch(r"ppt/slideLayouts/slideLayout\d+\.xml", n)):
            found = re.search(r'<p:cSld name="([^"]*)"', self.read(part))
            if found and found.group(1) == name and any(t.endswith(f"slideMaster{master}.xml") for t in self.rels(part).values()):
                return part
        raise SystemExit(f"{self.path.name}: no layout named {name!r} in master {master}")

    def background(self, part):
        """The image a layout or slide uses as its background."""
        found = re.search(r"<p:bg>.*?r:embed=\"(rId\d+)\".*?</p:bg>", self.read(part), re.S)
        if not found:
            raise SystemExit(f"{self.path.name}: {part} has no background image")
        return self.rels(part)[found.group(1)]

    def slide_layout(self, part):
        return next(t for t in self.rels(part).values() if "/slideLayouts/" in t)

    def pictures(self, part):
        """The images placed on a slide, in document order."""
        rels = self.rels(part)
        return [rels[rid] for rid in re.findall(r'(?:<a:blip|:svgBlip)\b[^>]*?r:embed="(rId\d+)"', self.read(part))]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def render_walkin(template, out):
    """Render the template's first slide (the walk-in) to SVG, without its hidden title text."""
    soffice = shutil.which("soffice") or "/Applications/LibreOffice.app/Contents/MacOS/soffice"
    try:
        import pymupdf
    except ImportError:
        try:
            import fitz as pymupdf
        except ImportError:
            return "PyMuPDF is not installed (pip install pymupdf)"
    if not Path(soffice).exists():
        return "LibreOffice (soffice) is not installed (brew install --cask libreoffice)"
    with tempfile.TemporaryDirectory() as temp:
        source = Path(temp) / "template.potx"
        shutil.copyfile(template, source)
        pdf_filter = 'pdf:impress_pdf_Export:{"PageRange":{"type":"string","value":"1"}}'
        result = subprocess.run([soffice, "--headless", f"-env:UserInstallation=file://{temp}/profile",
                                 "--convert-to", pdf_filter, "--outdir", temp, str(source)],
                                capture_output=True, text=True, timeout=300)
        pdf = Path(temp) / "template.pdf"
        if not pdf.exists():
            return f"LibreOffice could not render the template: {result.stderr.strip() or result.stdout.strip()}"
        page = pymupdf.open(pdf)[0]
        svg = page.get_svg_image(text_as_path=False)
    # The walk-in layout carries a hidden title placeholder; keep only the artwork.
    svg = re.sub(r"<text\b.*?</text>", "", svg, flags=re.S)
    svg = re.sub(r'<svg([^>]*?)width="[^"]*" height="[^"]*"', r'<svg\1width="1920" height="1080" preserveAspectRatio="xMidYMid slice"', svg, count=1)
    (out / "walkin.svg").write_text(svg)
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--template", type=Path, default=DEFAULTS["template"], help="Ignite session template (.potx)")
    parser.add_argument("--survey", type=Path, default=DEFAULTS["survey"], help="Session survey slides (.pptx)")
    parser.add_argument("--fonts", type=Path, default=DEFAULTS["fonts"], help="folder with PowerPoint's segoeui-*.woff files")
    parser.add_argument("--out", type=Path, default=DEFAULTS["out"], help="output folder, served at /template/")
    args = parser.parse_args()
    for name in ["template", "survey"]:
        if not getattr(args, name).is_file():
            raise SystemExit(f"{name} not found: {getattr(args, name)} (pass --{name})")

    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    for old in out.iterdir():
        if old.is_file():
            old.unlink()
    files, warnings = {}, []

    def copy(package, member, name):
        (out / name).write_bytes(package.zip.read(member))
        files[name] = f"{package.path.name}:{member}"

    template = Package(args.template)
    copy(template, template.background(template.layout("Title Slide")), "title-bg.jpg")
    problem = render_walkin(args.template, out)
    if problem:
        # The walk-in's artwork without the Microsoft logo and wordmark: better than nothing.
        member = template.background(template.layout("Walkin_Pathways"))
        copy(template, member, "walkin" + Path(member).suffix)
        warnings.append(f"walk-in rendered as background only: {problem}")
    else:
        files["walkin.svg"] = f"{args.template.name}: slide 1 (Walkin_Pathways), rendered"

    # The survey deck's dark slide: the one whose layout uses the dark background.
    survey = Package(args.survey)
    slides = sorted((n for n in survey.zip.namelist() if re.fullmatch(r"ppt/slides/slide\d+\.xml", n)),
                    key=lambda n: int(re.search(r"(\d+)\.xml", n).group(1)))
    dark = [s for s in slides if "How did we do?" in survey.read(s)]
    if not dark:
        raise SystemExit(f"{args.survey.name}: no 'How did we do?' slide")
    slide = dark[-1]
    pictures = survey.pictures(slide)
    frame = next((p for p in pictures if p.endswith(".svg")), None)
    code = next((p for p in pictures if p.endswith(".png")), None)
    if not frame or not code:
        raise SystemExit(f"{args.survey.name}: {slide} has no frame (SVG) and QR code (PNG)")
    copy(survey, survey.background(survey.slide_layout(slide)), "survey-bg.jpg")
    copy(survey, frame, "survey-frame.svg")
    copy(survey, code, "survey-qr.png")

    for weight, name in FONTS.items():
        found = sorted(args.fonts.glob(f"segoeui-{weight}*.woff")) if args.fonts.is_dir() else []
        if found:
            shutil.copyfile(found[0], out / name)
            files[name] = str(found[0])
        elif name in REQUIRED:
            warnings.append(f"Segoe UI {weight} not found in {args.fonts}: the stage uses the system font")

    missing = [name for name in REQUIRED if not (out / name).exists()]
    manifest = {
        "created": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "template": str(args.template), "survey": str(args.survey),
        "files": {name: {"from": source, "sha256": digest(out / name)} for name, source in sorted(files.items())},
        "required": REQUIRED, "missing": missing, "warnings": warnings,
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for name in sorted(files):
        print(f"  {name}")
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    print(f"{len(files)} files in {out}" + (f"; missing: {', '.join(missing)}" if missing else ""))
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
