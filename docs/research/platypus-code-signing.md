# Code signing the packaged executable (research, nothing is signed by Platypus)

Status: research only (2026-10-21). `platypus-build package` signs nothing. It has one hook, `--sign-cmd`, that runs the user's own
signing tool on the finished file. Facts about certificate authorities, prices and Microsoft's reputation rules change often: the
statements marked (verify) are from memory of 2024-2025 announcements and need a check before anyone relies on them.

## 1. Why sign, and the one rule about order

An unsigned exe downloaded from the internet gets Windows SmartScreen's "Windows protected your PC" page, and antivirus programs
treat an unknown, self-extracting, JIT-compiling program with suspicion. A signature names the publisher and lets reputation carry
over from one build to the next (an unsigned exe starts again from zero with every new file hash).

**Rule: embed the resources and the payload first, sign last.**

- Authenticode stores the signature in the PE **certificate table** (data directory 4, `IMAGE_DIRECTORY_ENTRY_SECURITY`). Unlike
  other directories its address is a *file offset*, not an RVA, and the table sits at the very end of the file, 8-byte aligned.
- The signed hash covers the headers (except the checksum and the certificate directory entry), every section, and any data after the
  last section (the "overlay"), but not the certificate table itself. Our payload is overlay data, so a payload appended **before**
  signing is covered and the signature stays valid.
- Changing anything the hash covers after signing (a resource edit, a new payload, a different version string) makes the signature
  invalid. PE resource editors rebuild the `.rsrc` section and move later sections and the checksum, and most of them leave a stale
  certificate table behind or drop it. So the order is fixed: copy host, edit resources (`pe.rs`), append payload, sign.
- Appending **after** signing puts bytes behind the certificate table. The PE specification expects the table to be last, those bytes
  are outside the hash, and some verifiers and scanners are stricter than `WinVerifyTrust`. Do not rely on it.
- A signed exe no longer ends with our trailer: the certificate (and its padding) follows it. `platypus_core::payload::Archive` therefore
  also looks for the trailer just before the certificate table when the end of the file has none (tested by `M9-API-1005` with a
  synthetic PE; not yet with a real signature, see TODO). An exe that was packaged, signed and then *re-packaged* is handled the same
  way: `host_len` strips the old payload and a re-signed file is a new file.
- The payload's own integrity is a CRC-32 per entry. That detects damage, not tampering; the signature over the whole file is what
  protects the payload (it is in the hashed overlay). The extracted cache folder is not protected: anyone who can write to the user's
  `%LOCALAPPDATA%` can already run code as that user.

## 2. Windows (the target that matters most)

| Option | Notes |
|---|---|
| `signtool sign` (Windows SDK) | The reference tool: `signtool sign /fd sha256 /tr <RFC 3161 URL> /td sha256 /f cert.pfx /p ... game.exe` or `/sha1 <thumbprint>` for a store or token certificate. Windows only. Always add a timestamp (`/tr`), otherwise the signature dies with the certificate. |
| `osslsigncode` | Open-source, cross-platform (OpenSSL). Signs PE files from Linux/macOS, supports PFX, PKCS#11 tokens and timestamping. Cannot sign in place: `-in game.exe -out game.signed.exe`, then rename (the hook below needs a tiny wrapper for that). Good for CI on Linux. |
| Azure Trusted Signing (renamed Artifact Signing, verify) | Microsoft's managed service: a short-lived certificate from a Microsoft-run CA, key in Azure, billed monthly (about ten dollars for the basic tier, verify). Used through `signtool` with the `Azure.CodeSigning.Dlib` (`/dlib ... /dmdf metadata.json`) or a GitHub Action. Needs an Azure subscription and an identity validation of the organisation (and, for individuals, a country that is eligible, verify). No hardware token to manage. |
| OV certificate (organisation validated) | From a CA (DigiCert, Sectigo, SSL.com, ...). Since June 2023 the CA/Browser Forum baseline requires the private key to live on a FIPS 140-2 level 2 / EAL 4+ device (USB token or cloud HSM), so there is no downloadable PFX any more for new certificates; CI needs a cloud HSM or the CA's signing service. Vetting takes days. |
| EV certificate | Stricter vetting, same hardware rule. It used to give an instant SmartScreen pass; Microsoft has reportedly dropped that special treatment, so EV and OV now both build reputation with downloads (verify). Still worth it for some organisations (kernel drivers need EV; stores and enterprise customers ask for it). |
| Self-signed | Useful only to test the pipeline (`signtool` with a certificate from `New-SelfSignedCertificate`): Windows shows "unknown publisher". |

**SmartScreen reputation** is per file hash for unsigned files and per signing identity (certificate or service identity) for signed
ones, and builds with downloads and the absence of malware reports. A new certificate starts with a warning even when valid; a
new build under an old, reputable identity does not. Submitting a build to Microsoft for analysis helps false positives.

**What Platypus could do without proprietary SDKs.** Everything up to the signature is already Rust and cross-platform (`editpe`,
`flate2`, `image`). Authenticode signing itself needs PKCS#7/CMS signing with the user's key, a timestamp request and the PE digest
rules; a Rust implementation is possible (the `authenticode` crate parses and hashes PE files, CMS crates exist) but certificates on
HSMs and tokens are reached through PKCS#11 or vendor libraries, so shelling out to `signtool` / `osslsigncode` is the realistic
path. That is what the hook does.

## 3. The hook: `platypus-build package --sign-cmd "<command>"` (implemented, tiny)

After the output file is written, `{exe}` in the command is replaced by the quoted path and the command runs through `cmd /S /C`
(Windows) or `sh -c`. A failing command fails the package; the unsigned file stays on disk. Examples:

```
--sign-cmd "signtool sign /fd sha256 /tr http://timestamp.digicert.com /td sha256 /sha1 THUMBPRINT {exe}"
--sign-cmd "signtool sign /fd sha256 /tr http://timestamp.acs.microsoft.com /td sha256 /dlib Azure.CodeSigning.Dlib.dll /dmdf trusted.json {exe}"
--sign-cmd "sign.cmd {exe}"        (a script that runs osslsigncode -in/-out and moves the result back)
```

Secrets (passwords, tokens) belong in the environment or the tool's own store, not on the command line. Left for the hook: an
`{out}` placeholder (or built-in in/out handling) so `osslsigncode` needs no wrapper; verifying after signing (`signtool verify /pa`
or `osslsigncode verify`) and printing the result; a `package.json` `platypus.signCommand` so the command is not retyped.

## 4. macOS

- A `.app` bundle (`Contents/MacOS/<exe>`, `Contents/Resources/platypus payload`, `Info.plist`, `.icns`), signed with a **Developer ID
  Application** certificate (Apple Developer Program, 99 USD a year), with the **hardened runtime** (`codesign --options runtime`).
- Platypus's Cranelift JIT needs `com.apple.security.cs.allow-jit` (and probably `allow-unsigned-executable-memory`) in the entitlements,
  or the hardened runtime kills the game when it compiles. The `--aot` object avoids this.
- **Notarization**: upload the zipped or dmg'd app with `xcrun notarytool submit --wait` (an App Store Connect API key or an app
  password), then `xcrun stapler staple`. Gatekeeper refuses un-notarized apps downloaded from the internet. Apple Silicon refuses to
  run any unsigned arm64 binary, so even local builds need at least an ad-hoc signature (`codesign -s -`).
- From Rust without Apple's tools: **`apple-codesign` (`rcodesign`)** is a pure-Rust implementation of `codesign`, notarization
  upload, stapling and `.app`/`.dmg` handling, usable from Linux and Windows, with a `.p12` certificate and an API key. It is the one
  signing tool in this list that could be a library dependency of `platypus-build`. Needs the user's Apple account, certificate and key.
- As with Windows: bundle contents and the payload first, sign last (the signature seals the whole bundle; changing a resource
  afterwards breaks it). On macOS the payload would be a file in `Resources/`, not appended to the Mach-O (appended data in a signed
  Mach-O is checked against the code directory).

## 5. Android and iOS

- **Android**: an APK is signed with `apksigner` (Android SDK build-tools; schemes v2/v3, v4 for incremental installs) after
  zip-aligning (`zipalign`), the other way round from jar signing. An **AAB** for Google Play is signed with `jarsigner` (v1) using
  the **upload key**, and Play re-signs the delivered APKs with the app signing key (Play App Signing). Needs a keystore (keep the
  upload key safe) and a Play developer account (25 USD once). `apksigner` is Java; the format is documented and implemented in
  Rust crates of uneven quality, so shelling out to the SDK is the safe route. Debug builds are signed with an auto-made debug key.
- **iOS**: apps run only when signed with a certificate (Apple Development for devices, Apple Distribution for TestFlight/App Store)
  and an embedded **provisioning profile** (device list or distribution profile, entitlements, bundle id = `platypus.identifier`).
  `codesign` (or `rcodesign sign` with the `.p12` and profile) seals the `.app`; the `.ipa` is uploaded with `xcrun altool` /
  `notarytool`-family tools or Transporter. Needs the Apple Developer Program, a Mac for the Xcode tools in practice, and no JIT
  (see the mobile plan in `docs/platypus-build.md`).

## 6. Linux

No platform signing requirement. Distribution signing is by channel: GPG-signed packages or `AppImage` signatures, Flatpak/Flathub
and Steam handle their own. Steam's DRM wrapper rewrites the exe, so Windows signing is done by Steam's tools on the wrapped file or
skipped.

## 7. What needs the user

| Thing | For |
|---|---|
| A Windows code-signing certificate (OV/EV on a token or cloud HSM, a few hundred USD a year) **or** an Azure subscription with an approved Trusted/Artifact Signing identity | Windows Authenticode |
| Apple Developer Program membership, a Developer ID Application certificate, an App Store Connect API key | macOS signing, notarization; iOS provisioning |
| A Play developer account and an upload keystore | Android release builds |
| A decision on the publisher name (it is the identity in the certificate: it should equal `platypus.company`) | all |
| A Windows test machine with a fresh profile (SmartScreen reputation is per user machine and per download source) | checking the result |

## 8. TODO

1. Sign a real build with `signtool` and `osslsigncode` once a certificate exists: confirm `Archive::open` finds the payload behind
   the real certificate table (only a synthetic PE is tested) and that `signtool verify /pa` passes.
2. `{out}` placeholder or in/out handling in `--sign-cmd`; a verify step after signing.
3. `platypus.signCommand` in `package.json`; an environment variable for the command in CI.
4. macOS: `.app` bundle writer (`Info.plist`, `.icns` from the icon, payload in `Resources/`), entitlements file with `allow-jit`,
   `rcodesign` integration for sign and notarize.
5. Android/iOS packaging and signing hooks (after the `--aot` path covers the engine API).
6. Consider embedding the Windows application manifest (DPI awareness, UTF-8 code page, no UAC) before signing, with the resources.
7. Document SmartScreen expectations for players on the game's download page (reputation takes time even when signed).
