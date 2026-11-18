#!/usr/bin/env python3
"""Install Somewhere's reference recordings for personal use, outside the project.
The recordings are not part of the source tree or release archive.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import urllib.request

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--directory", type=Path, default=Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))) / "quickfox/sounds/reference")
args = parser.parse_args()
args.directory.mkdir(parents=True, exist_ok=True)
packs = [
    ("EG Oreo", "oreo.ogg", b"OggS", "https://mechvibes.com/sound-packs/eg-oreo"),
    ("NK Cream", "nk-cream.wav", b"RIFF", "https://mechvibes.com/sound-packs/nk-cream-original-by-ryan"),
]
records = []
for name, filename, signature, license_url in packs:
    destination = args.directory / filename
    source = "https://www.somewheretypingtest.com/sfx/" + filename
    if destination.exists():
        data = destination.read_bytes()
    else:
        with urllib.request.urlopen(source, timeout=30) as response:
            data = response.read(16 * 1024 * 1024 + 1)
        if not data.startswith(signature) or len(data) > 16 * 1024 * 1024:
            raise SystemExit(f"Invalid sound response: {filename}")
        with tempfile.NamedTemporaryFile(dir=args.directory, delete=False) as temporary:
            temporary.write(data)
            temp_path = Path(temporary.name)
        temp_path.replace(destination)
    if not data.startswith(signature) or len(data) > 16 * 1024 * 1024:
        raise SystemExit(f"Existing sound is invalid: {destination}")
    records.append({"name": name, "file": filename, "source": source, "license": "Personal use; do not redistribute", "license_url": license_url, "sha256": hashlib.sha256(data).hexdigest()})
    print(f"Installed {name}: {destination}")
(args.directory / "provenance.json").write_text(json.dumps(records, indent=2) + "\n")
print("Select EG Oreo or NK Cream in Settings > Sound. Sound remains off until enabled.")
