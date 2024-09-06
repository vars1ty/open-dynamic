# /bin/bash
# cargo +nightly build -Z build-std=std --target x86_64-pc-windows-gnu --release <-- No XWin
#RUSTFLAGS="-Z threads=1 -C target-feature=-crt-static -C link-arg=-Wl" cargo +nightly xwin build --release --target x86_64-pc-windows-msvc $@
RUSTFLAGS="-Clink-args=-fuse-ld=lld -Clink-args=-Wl,--icf=all -Z threads=8 -Z location-detail=none" cargo +nightly xwin build --release --target x86_64-pc-windows-msvc $@
