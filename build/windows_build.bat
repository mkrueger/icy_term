cd ..\..\icy_engine 
git pull
cd ..\icy_term
git pull
cargo build --release
rm build\icy_term_windows_.zip
powershell Compress-Archive target\release\icy_term.exe build\icy_term_windows_.zip
