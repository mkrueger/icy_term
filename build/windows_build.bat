cd ..\..\icy_engine 
git pull
cd ..\icy_term
git pull
cargo build --release
powershell Compress-Archive target\release\icy_term.exe build\icy_term_windows_.zip