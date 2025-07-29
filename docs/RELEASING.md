# Release Process

This document describes the release process for Union Square.

## Overview

Union Square uses an automated release process that:
1. Publishes to crates.io via release-plz
2. Creates GitHub releases with binary artifacts for multiple platforms
3. Builds and publishes Docker images to GitHub Container Registry
4. Generates Homebrew formula for macOS/Linux installation

## Automated Release Workflow

### 1. Crate Publishing (release-plz)

The `release-plz` workflow runs on every push to `main` and:
- Creates PRs to update version numbers and changelogs
- Publishes to crates.io when release PRs are merged
- Creates GitHub releases with generated release notes

### 2. Binary Releases

When a GitHub release is published (automatically by release-plz), the binary release workflow:

#### Builds Native Binaries
- **Linux**: x86_64, aarch64
- **macOS**: x86_64 (Intel), aarch64 (Apple Silicon)
- **Windows**: x86_64, aarch64

#### Creates Release Artifacts
- Compressed archives (`.tar.gz` for Unix, `.zip` for Windows)
- SHA256 checksums for each artifact
- Combined checksums file

#### Docker Images
- Multi-platform images (linux/amd64, linux/arm64)
- Published to `ghcr.io/jwilger/union_square`
- Tagged with version numbers and git SHA

## Installation Methods

### From Crates.io
```bash
cargo install union_square
```

### From GitHub Releases
```bash
# Linux x86_64
curl -L https://github.com/jwilger/union_square/releases/latest/download/union_square-x86_64-linux.tar.gz | tar xz

# macOS Apple Silicon
curl -L https://github.com/jwilger/union_square/releases/latest/download/union_square-aarch64-macos.tar.gz | tar xz
```

### Using Docker
```bash
docker pull ghcr.io/jwilger/union_square:latest
docker run ghcr.io/jwilger/union_square:latest
```

### Via Homebrew (Future)
```bash
# Once tap is created
brew tap jwilger/union_square
brew install union_square
```

## Manual Release Process

If you need to trigger a release manually:

1. **Create a release tag**:
   ```bash
   git tag -a v0.3.0 -m "Release v0.3.0"
   git push origin v0.3.0
   ```

2. **Create GitHub release**:
   - Go to GitHub Releases page
   - Click "Draft a new release"
   - Select the tag
   - Let GitHub generate release notes
   - Publish the release

3. **Monitor workflows**:
   - Check Actions tab for binary-release workflow
   - Verify all artifacts are uploaded
   - Check Docker images at ghcr.io

## Troubleshooting

### Failed Builds

If a platform build fails:
1. Check the workflow logs in GitHub Actions
2. Common issues:
   - Missing cross-compilation tools
   - Dependency issues for specific platforms
   - Resource limits (especially for ARM builds)

### Docker Build Issues

- Ensure Dockerfile is up to date with dependencies
- Check for architecture-specific issues
- Verify base image compatibility

### Release Asset Upload Failures

- Check GitHub token permissions
- Verify file paths and patterns
- Ensure artifacts were created successfully

## Security Considerations

1. **Checksums**: All artifacts include SHA256 checksums
2. **Signing**: Consider adding GPG signing in the future
3. **SBOM**: Consider generating Software Bill of Materials
4. **Container Scanning**: Docker images are scanned by GitHub

## Future Enhancements

1. **GPG Signing**: Sign releases with GPG keys
2. **Homebrew Tap**: Create official Homebrew tap
3. **Package Managers**:
   - AUR for Arch Linux
   - Snap/Flatpak for Linux
   - Chocolatey/Scoop for Windows
4. **Release Notes**: Automated changelog generation
5. **Beta Releases**: Support for pre-release channels
