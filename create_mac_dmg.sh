cargo bundle --release --target aarch64-apple-darwin

rm IcyTerm-Installer-aarch64-apple-darwin.dmg
codesign --force --deep --verbose --sign "mkrueger@posteo.de" "target/aarch64-apple-darwin/release/bundle/osx/Icy Term.app/"

create-dmg \
  --volname "Icy Term Installer" \
  --volicon "target/aarch64-apple-darwin/release/bundle/osx/Icy Term.app/Contents/Resources/Icy Term.icns" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --hide-extension "Icy Term.app" \
  --app-drop-link 600 185 \
  "IcyTerm-Installer-aarch64-apple-darwin.dmg" \
  "target/aarch64-apple-darwin/release/bundle/osx/Icy Term.app/"
cargo bundle --release --target x86_64-apple-darwin

rm IcyTerm-Installer-x86_64-apple-darwin.dmg
codesign --force --deep --verbose --sign "mkrueger@posteo.de" "target/x86_64-apple-darwin/release/bundle/osx/Icy Term.app/"
create-dmg \
  --volname "Icy Term Installer" \
  --volicon "target/x86_64-apple-darwin/release/bundle/osx/Icy Term.app/Contents/Resources/Icy Term.icns" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --hide-extension "Icy Term.app" \
  --app-drop-link 600 185 \
  "IcyTerm-Installer-x86_64-apple-darwin.dmg" \
  "target/x86_64-apple-darwin/release/bundle/osx/Icy Term.app/"
