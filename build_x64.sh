# /bin/bash
# cargo +nightly build -Z build-std=std --target x86_64-pc-windows-gnu --release <-- No XWin
#RUSTFLAGS="-Z threads=1 -C target-feature=-crt-static -C link-arg=-Wl" cargo +nightly xwin build --release --target x86_64-pc-windows-msvc $@
RUSTFLAGS="-Z threads=8" cargo +nightly xwin build --release --target x86_64-pc-windows-msvc $@
