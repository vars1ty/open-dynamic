# /bin/bash
# cargo +nightly build -Z build-std=std --target i686-pc-windows-gnu --release <-- No XWin
#RUSTFLAGS="-Zthreads=20"
cargo xwin build --release --target i686-pc-windows-msvc $@
