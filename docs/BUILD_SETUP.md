# Local build setup diagnosis

Cargo stable is now installed permanently at `%USERPROFILE%/.cargo/bin`; `cargo metadata` succeeds with that directory on PATH. The npm Tauri launcher configures Cargo for its child process automatically, including terminals opened before installation.

The development run reached Rust compilation, but dependency build scripts failed because the available `link.exe` belongs to coreutils. Microsoft MSVC tools and Windows SDK were not installed. `scripts/tauri.mjs` now locates a complete C++ tools installation with vswhere and imports VsDevCmd's environment before running Tauri. Missing tools fail early with a concise message.

Automatic installation through winget and an administrator retry returned installer exit code 5001. Interactive startup also closed immediately without exposing a usable explanation. A subsequent fresh Microsoft-installer download/run attempt was rejected by automatic approval review as blocked by policy. No successful Build Tools installation is claimed.

## Installer startup diagnosis and repair

The downloaded installer has a valid Microsoft signature. A minimal x86 .NET Framework program reproduced `System.UriParser` type initialization failure / NullReferenceException on a normal HTTPS address; the x64 equivalent passed. Windows' x86 PowerShell also failed to initialize ServicePointManager. A process-local `COMPLUS_ZapDisable=1` diagnostic made the x86 test pass, implicating the native-image cache. This variable was not added to user or machine settings.

DISM RestoreHealth failed with 0x800f0915 (repair source/component store could not be repaired); SFC completed with no protected-file integrity violations. This is not a claim that the Windows component store is healthy.

The first NGEN rebuild inherited the diagnostic flag and failed; a clean `Framework/v4.0.30319/ngen.exe update /force` completed successfully, then System, AuditPolicyGPManagedStubs.Interop and EventViewer native-image roots were installed successfully. After regeneration, both the x86 URI test and x86 PowerShell startup passed without diagnostic overrides. `VisualStudioSetup.exe --help` then exited with code 0 and generated a normal bootstrapper log, replacing the previous 5001 startup failure. The C++ Build Tools installation was retried after this verification.

Local diagnostic source, tests and repair logs are under `%TEMP%/appterisk-installer-diagnostics`; no diagnostic logs were uploaded.

Final verification: Build Tools installer exit 0; `npm run tauri dev` compiled the full project successfully and launched `target/debug/appterisk-messenger.exe`. Its visible window was verified, with three native provider login pages loaded. A leftover Vite process occupying port 1420 was stopped before the successful launch. The application was left running. Real account linking/isolation tests were not performed.

Build Tools 2022 with **Desktop development with C++**, MSVC x64/x86 tools and Windows SDK subsequently installed successfully (installer exit 0) after the native-image repair. The npm launcher now initializes these tools from Git Bash or PowerShell. Native compilation was retried and progressed beyond the previous linker failure.
