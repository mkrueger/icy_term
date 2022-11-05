VER=$(cat Cargo.toml | grep "version"  | awk -F"\"" '{print $2}' | xargs)

rm IcyTerm-Installer-*.dmg
for ARCH in aarch64-apple-darwin x86_64-apple-darwin
do
  cargo bundle --release --target $ARCH
  codesign --force --deep --verbose --sign "mkrueger@posteo.de" "target/$ARCH/release/bundle/osx/Icy Term.app/"
  create-dmg \
    --volname "Icy Term Installer" \
    --volicon "target/$ARCH/release/bundle/osx/Icy Term.app/Contents/Resources/Icy Term.icns" \
    --window-pos 200 120 \
    --window-size 800 400 \
    --icon-size 100 \
    --hide-extension "Icy Term.app" \
    --app-drop-link 600 185 \
    "IcyTerm-Installer-$VER-$ARCH.dmg" \
    "target/$ARCH/release/bundle/osx/Icy Term.app/"
done
