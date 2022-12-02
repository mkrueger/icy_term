cd ..\..\icy_engine 
git pull
cd ..\icy_term
git pull
cargo build --release
del build\icy_term_windows_.zip
powershell Compress-Archive "target\release\icy_term.exe,build\file_id.diz" build\icy_term_windows_.zip
