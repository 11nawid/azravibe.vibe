"""Build install/azra-vibe.vsix from editors/vscode without npm/vsce."""

import os
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
EXT = os.path.join(ROOT, "editors", "vscode")
OUT = os.path.join(HERE, "azra-vibe.vsix")

MANIFEST = """<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="azra-vibe" Version="0.0.5" Publisher="azravibe" />
    <DisplayName>Azra (.vibe) Language Support</DisplayName>
    <Description>Syntax highlighting and language server support for Azra.</Description>
    <Tags>azra,vibe,programming-language</Tags>
    <Categories>Programming Languages</Categories>
  </Metadata>
  <Installation><InstallationTarget Id="Microsoft.VisualStudio.Code" /></Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
  </Assets>
</PackageManifest>
"""

CONTENT_TYPES = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json" />
  <Default Extension="js" ContentType="application/javascript" />
  <Default Extension="md" ContentType="text/markdown" />
  <Default Extension="vsixmanifest" ContentType="application/xml" />
</Types>
"""

with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.writestr("extension.vsixmanifest", MANIFEST)
    zf.writestr("[Content_Types].xml", CONTENT_TYPES)
    for base, _dirs, files in os.walk(EXT):
        for name in files:
            path = os.path.join(base, name)
            rel = os.path.relpath(path, EXT).replace("\\", "/")
            zf.write(path, "extension/" + rel)

print("wrote", OUT, os.path.getsize(OUT))
