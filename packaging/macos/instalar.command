#!/bin/bash
# Instalador do BC Drums: dê dois cliques (ou rode no Terminal).
set -e
cd "$(dirname "$0")"
P="$HOME/Library/Audio/Plug-Ins"
mkdir -p "$P/VST3" "$P/CLAP" "$P/Components" "$HOME/Music/BC Drums/Packs"
rm -rf "$P/VST3/BC Drums.vst3" "$P/CLAP/BC Drums.clap" "$P/Components/BC Drums.component"
cp -R "BC Drums.vst3" "$P/VST3/"
cp -R "BC Drums.clap" "$P/CLAP/"
[ -d "BC Drums.component" ] && cp -R "BC Drums.component" "$P/Components/"
rm -rf "/Applications/BC Drums.app"; cp -R "BC Drums.app" /Applications/ 2>/dev/null || true
cp -R Packs/* "$HOME/Music/BC Drums/Packs/" 2>/dev/null || true
xattr -dr com.apple.quarantine "$P/VST3/BC Drums.vst3" "$P/CLAP/BC Drums.clap" "$P/Components/BC Drums.component" "/Applications/BC Drums.app" 2>/dev/null || true
killall -9 AudioComponentRegistrar 2>/dev/null || true
echo "BC Drums instalado."
