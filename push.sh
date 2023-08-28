#/bin/sh
cargo fmt
cargo test
if [ $? -ne 0 ]; then
  exit 1
fi
git push 
