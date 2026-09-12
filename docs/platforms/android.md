## Running on Android

`liphia_cli_gui` compiles to a native Android APK (built with `cargo-apk`)
in addition to Windows/Linux/macOS. The Android app has no terminal, so it
doesn't take a `.lph` path as an argument — instead, it opens with an "Open
File" button that lets you pick any `.lph` script already on the device.
Output (`print()` calls, compile/runtime errors) is shown in an in-app
console panel at the bottom of the window, since there's no visible stdout
on Android.

No GUI-specific natives (`gui_heading`, `gui_button`, etc.) are required —
a plain script using only `print()` runs exactly as it would from
`liphia_cli`, just rendered inside the app's console panel.

---

## Environment setup (one-time, per machine)

Required: Android SDK, Android NDK, a JDK, `cargo-apk`, and the Android
Rust targets. Versions below must match what's declared in
`liphia_cli_gui/Cargo.toml` — if either side changes, update the other.

- **Android SDK** — installed via Android Studio's SDK Manager (SDK
  Platforms tab). `target_sdk_version` in `Cargo.toml`
  (`[package.metadata.android.sdk]`) must have a matching platform
  installed (currently `36`).
- **Android NDK** — installed via SDK Manager, SDK Tools tab → "NDK (Side
  by side)". Note the exact version folder name under
  `<sdk>/ndk/<version>/` — that's the value `ANDROID_NDK_ROOT` must point to.
- **JDK** — any recent JDK works (`keytool`/`apksigner` just need Java on
  disk). `JAVA_HOME` must point to the JDK root, not the `bin` folder.
- **`cargo-apk`**:
```bash
  cargo install cargo-apk
```
- **Rust Android targets**:
```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### Environment variables

Set once via `[System.Environment]::SetEnvironmentVariable(..., "User")`
on Windows (or the shell profile on Linux/macOS). A new terminal window is
required after setting these before they take effect.

```powershell
[System.Environment]::SetEnvironmentVariable("JAVA_HOME", "C:\Program Files\Java\jdk-25", "User")
[System.Environment]::SetEnvironmentVariable("ANDROID_HOME", "$env:LOCALAPPDATA\Android\Sdk", "User")
[System.Environment]::SetEnvironmentVariable("ANDROID_NDK_ROOT", "$env:LOCALAPPDATA\Android\Sdk\ndk\<version>", "User")
```

Replace `<version>` with the actual folder name found under
`<sdk>/ndk/`. Verify all three after opening a new terminal:

```powershell
$env:JAVA_HOME
$env:ANDROID_HOME
$env:ANDROID_NDK_ROOT
```

---

## Debug build (fastest way to test on a device)

No signing setup needed — `cargo-apk` signs it automatically with an
auto-generated debug keystore (`~/.android/debug.keystore` on first run).
Fine for testing on your own device, not for distribution.

```bash
cd src
cargo apk build -p liphia_cli_gui
```
APK output: `target/debug/apk/liphia_cli_gui.apk`

---

## Release build

Requires your own signing keystore. `cargo-apk` 0.10.0 only accepts the
keystore password as a plain string in `Cargo.toml` — it does not support
reading it from an environment variable. Because of this, the password sits
in `Cargo.toml` **only for the duration of the build**, then gets removed
(see last step below).

### One-time: generate the keystore

```bash
mkdir C:\keys
cd C:\keys
keytool -genkey -v -keystore liphia-release.jks -keyalg RSA -keysize 2048 -validity 10000 -alias liphia
```
`keytool` ships with the JDK — if it's not on `PATH`, call it via
`"$env:JAVA_HOME\bin\keytool.exe"` instead. You'll be prompted for a
keystore password and some identity fields (any values are fine for
personal use). **Back up the resulting `.jks` file and its password** —
losing either means future release builds can't be signed with the same
identity.

### Every release build

1. Add the signing block to `liphia_cli_gui/Cargo.toml`, pointing at where
   the keystore actually lives, with the real password:
```toml
   [package.metadata.android.signing.release]
   path = "PATH_TO_YOUR_KEYSTORE/liphia-release.jks"
   keystore_password = "your_password_here"
```
2. Build:
```bash
   cargo apk build --release -p liphia_cli_gui
```
   APK output: `target/release/apk/liphia_cli_gui.apk`
3. **Remove the `[package.metadata.android.signing.release]` block (with
   the password) from `Cargo.toml` once the build finishes.** The block is
   only needed at build time — leaving it in the file is what risks it
   getting committed.

### `.gitignore`

```gitignore
# Android release signing — keystore files must never be committed
*.jks
*.keystore
```

The password itself isn't caught by `.gitignore` since it lives inside
`Cargo.toml` — that's exactly why step 3 above (remove the block after
building) matters more than the ignore rule here.



### Signing block template

For reference, the shape of the block to add before a release build
(placeholders only — never commit real values):

```toml
[package.metadata.android.signing.release]
path = "PATH_TO_YOUR_KEYSTORE/liphia-release.jks"
keystore_password = "TYPE_YOUR_PASSWORD_HERE_THEN_DELETE_AFTER_BUILD"
```


---

## Known limitations

- The in-app file picker ("Open File") only works on desktop builds for
  now. `rfd` 0.15.4 has no Android backend implementation, so on Android
  the button is currently a no-op. A script can still be tested by pushing
  it to the app's storage via `adb push` and loading it manually — proper
  Storage Access Framework integration is planned.