# RustyDB Build Information

**Build Date:** December 25, 2025  
**Rust Version:** 1.92.0 (ded5c06cf 2025-12-08)  
**Cargo Version:** 1.92.0 (344c4567c 2025-10-21)

## Binaries Included

### Linux (x86_64-unknown-linux-gnu)
- **Server:** `linux/rusty-db-server` (37 MB)
- **CLI:** `linux/rusty-db-cli` (921 KB)
- **Target:** ELF 64-bit LSB executable
- **Platform:** GNU/Linux 3.2.0+

### Windows (x86_64-pc-windows-gnu)
- **Server:** `windows/rusty-db-server.exe` (40 MB)
- **CLI:** `windows/rusty-db-cli.exe` (876 KB)
- **Target:** PE32+ executable (console) x86-64
- **Platform:** MS Windows (MinGW-w64)

## Build Configuration

### Profile
- **Type:** Release
- **Optimization:** Level 3 (full optimization)
- **LTO:** Thin LTO enabled
- **Codegen Units:** 16
- **Debug Info:** Minimal (level 1)

### Compilation Features
- Cross-compiled from Linux using MinGW-w64
- Native Linux compilation
- Full SIMD optimizations enabled
- Enterprise features included

## Build Commands Used

```bash
# Linux build
cargo build --release --target x86_64-unknown-linux-gnu

# Windows build (cross-compile)
cargo build --release --target x86_64-pc-windows-gnu
```

## Running the Binaries

### Linux
```bash
# Make executable (if needed)
chmod +x builds/linux/rusty-db-server
chmod +x builds/linux/rusty-db-cli

# Run server
./builds/linux/rusty-db-server

# Run CLI
./builds/linux/rusty-db-cli --help
```

### Windows
```cmd
REM Run server
builds\windows\rusty-db-server.exe

REM Run CLI
builds\windows\rusty-db-cli.exe --help
```

## Dependencies

### Linux Runtime Dependencies
- glibc 2.31+
- OpenSSL libraries (if not statically linked)

### Windows Runtime Dependencies
- None (static linking via MinGW)
- Compatible with Windows 7 SP1 and later

## Notes
- Windows binaries were cross-compiled using MinGW-w64 toolchain
- Both builds use the same source code and configuration
- All enterprise optimization features are enabled
- SIMD optimizations are included for both platforms
- Both builds completed successfully with 159-160 warnings (non-critical)
