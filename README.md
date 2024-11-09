# dynamic
A Rust cheat framework, aimed at making the process of creating game cheats simpler.

## What is it?
dynamic is a Rust-based cheat engine which offer a variety of quality-of-life features, such as:
1. Memory-based functions for reading and writing memory
    - Integers, Unsigned Integers, Floats, Strings and Byte-arrays are all supported.
2. Windows-related functions which for example, let you find the address of a non-mangled function and returns it
3. UI Builder integration by wrapping around ImGui, with image support and an use-to-usage syntax
4. Plugin support named "Arctic Gateways", which load Rust-based DLLs where you can utilize ImGui and certain functions directly from dynamic.
    - Useful for when you need lower-level access but Rune doesn't justify. An example may be for hooking functions.
5. Server groups, you can join a custom "party"/channel via CrossCom and send Rune scripts over the network to other group members.
6. Semi-automatic internal ImGui available for you.
7. The Rune programming language embedded, with functions that offer a variety of features, for example:
    - Creation of Custom UIs through UI Builder
    - Detouring up to 10 functions into Rune, with access up to 10 parameters from the hooked function
      - Note: This is highly unstable and prone to crashes on x86!
    - Calling function pointers with up to 10 parameters*
8. ... and a lot more

* The parameters you pass in must each have their own Rust-equivalent version. You cannot pass in Rune strctures or custom types.

## But I can just make my own cheat in Rust?
That's correct, and it might even be better when you want to cheat in games with competent anti-cheats, like Easy Anti-Cheat as dynamic wasn't made to bypass any anti-cheat.

dynamic is intended for users that want a headstart with simpler games and can build with pre-existing functions in a simple, but functional scripting language.

If you don't need to bypass any anti-cheats, or just have a very badly coded one, then dynamic will probably work just fine for you.

If you need true low-level access beyond what Rune can offer, then you may utilize plugins written in Rust.

## What platforms?
dynamic is developed under Linux, so it had to support Wine/Proton.

- Windows 10: Supported, 11 is not tested for but users haven't experienced any issues with it.
- Linux: Supported via Wine/Proton.
- MacOS: Unsupported, could maybe work via Wine/Proton.
- Android: Unsupported, could maybe work via Wine/Proton apps but the performance would most definitely take a hit.

## What renderers?
- DirectX 9: Working okay-ish, has some rough edges.
- DirectX 11: Working.
- DirectX 12: Working.
- OpenGL: Working.

If you can't render an UI for whatever reason, you can set the renderer as `None` and just have a terminal.

## Can I compile this and run it?
No. Dynamic is source-available, but the server and certain custom-made libraries **are not** as of yet.

Technically you could strip those parts out and make your own server, then it would work but it's not recommended.

## What is zencstr?
zencstr just stands for **Z**eroized **Encr**ypted **Str**ing.

It works like this if you use the zencstr macro:
1. Encrypts all string literals at compile-time, leaving non-literals unencrypted
2. Decrypts the string into a ZString at runtime and you're free to use it however you want
3. ZString zeroizes the string data after usage

The library for it is not yet open-source, but will be in the future.
## How do I inject the DLL?
Use any DLL Injector that calls `LoadLibraryA`, or manually calls `DllMain`.

Note that a server is required for dynamic to work.

## Why isn't serde being used for serializing/deserializing to and from the server?
Because it exposes data in memory in form of plain-text, is slower than rkyv, uses more bandwidth and relies on primarily JSON when not needed in this case.
