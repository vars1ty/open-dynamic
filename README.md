# dynamic
A Rust cheat engine, aimed at making the process of creating game cheats simpler.

## What is it?
dynamic is a Rust-based cheat engine which offer a variety of quality-of-life features, such as:
1. Memory-based functions for reading and writing memory
    - Integers, Unsigned Integers, Floats, Strings and Byte-arrays are all supported.
2. Windows-related functions which for example, let you find the address of a non-mangled function and returns it
3. UI Builder integration by wrapping around ImGui, with image support and an use-to-usage syntax
4. Plugin support named "Arctic Gateways", which load Rust-based DLLs where you can utilize ImGui and certain functions directly from dynamic.
    - Useful for when you need lower-level access but Rune doesn't justify. An example may be for hooking functions.
5. Server groups, you can join a custom "party"/channel via CrossCom and send Rune scripts over the network to other group members.
6. Semi-automatic internal ImGui available for you
    - All you have to do is specify the renderer within your config and you're good-to-go.
7. Community-submitted content available at your fingertips
    - Enjoy the content created by other dynamic users, whether it be entirely custom game cheats, or small game-specific scripts.
    - To top it off, all community-submitted content is encrypted on the server and decrypted on-demand.
    - This is especially useful for Rune-based content, as normally it would just be a large single string in memory.
8. Sellix integration
    - If you are a developer, you may make use of dynamic's Sellix integration which works for both Rune and plugins.
    - No cut is taken by dynamic for your product, 0%. You create what you want, submit it and if accepted, it'll be listed.

## But I can just make my own cheat in Rust?
That's correct, and it might even be better when you want to cheat in games with competent anti-cheats, like Easy Anti-Cheat as dynamic wasn't made to bypass any anti-cheat.

dynamic is intended for users that want a headstart and can build with pre-existing functions in a simple, but functional scripting language.

If you don't need to bypass any anti-cheats, or just have a very badly coded one, then dynamic will probably work just fine for you.

If you need true low-level access, beyond what Rune can offer, then you may utilize "Arctic Gateways", aka native plugins written in Rust.

## What platforms?
dynamic is developed under Linux, so it had to support WINE.

- Windows 10: Supported, 11 is not tested for but users haven't experienced any issues with it.
- Linux: Supported via WINE/Proton.
- MacOS: Unsupported, could maybe work via WINE/Proton.
- Android: Unsupported, could maybe work via WINE/Proton apps but the performance would most definitely take a hit.

## What renderers?
- DirectX 9: Working okay-ish, has some rough edges.
- DirectX 11: Working.
- DirectX 12: Working.
- OpenGL: Working.

If you can't render an UI for whatever reason, you can set the renderer as `None` and just have a terminal.

## What's the difference between open dynamic, and paid?
Open dynamic (this right here) strips out some code, such as:
1. Dynamic's own Server IP, as it's only available for the paid version for safety reasons.
2. Windows 11 checks, as they aren't needed.
3. Opera, Opera GX and Google Chrome checks, as they aren't needed.

The paid version, due to being able to connect to its own primary server, has community content available.

## Is the server open-source?
As of now, **no**, as it has to be refactored a bit more before that can happen, and community content cannot be published on the public repo.

## What is zencstr?
zencstr just stands for **Z**eroized **Encr**ypted **Str**ing.

It works like this if you use the zencstr macro:
1. Encrypts all string literals at compile-time, leaving non-literals unencrypted
2. Decrypts the string into a ZString at runtime and you're free to use it however you want
3. ZString zeroizes the string data after usage

The library for it is not yet open-source, but will be in the near future.
## How do I inject the DLL?
Use any DLL Injector that calls `LoadLibraryA`, or manually calls `DllMain`.

Note that a server is required for dynamic to work.

## Why isn't serde being used for serializing/deserializing to and from the server?
Because it exposes data in memory in form of plain-text, is slower than rkyv, uses more bandwidth and relies on primarily JSON when not needed in this case.

## I can't build it!
Correct, for now you can't build dynamic on your own until the rest of the framework has been made available, primarily `zencstr`, a custom fork of `hudhook` and `imgui-rs`.

Until that changes, you can only read the code.
