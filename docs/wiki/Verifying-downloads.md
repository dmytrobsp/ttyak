Every file of a release from 0.11.0 on (the zips, the `.deb` and `.rpm`, the recipes zip) has its line in the
release's `SHA256SUMS` and a GitHub attestation of where it was built. The install script and ttyak's update screen
check the SHA256 themselves (on macOS the update also wants the new ttyak signed by the same Apple developer as the
one that runs, when that one is signed).

## Checksums

For files you downloaded, put `SHA256SUMS` next to them and run

```sh
sha256sum -c SHA256SUMS --ignore-missing           # Linux
shasum -a 256 -c SHA256SUMS --ignore-missing       # macOS
```

which says `OK` for each of them. On Windows, PowerShell's `Get-FileHash <file>` prints a file's SHA256, to compare
with its line in `SHA256SUMS`.

## Attestations

With the [GitHub CLI](https://cli.github.com) (for version 0.12.1):

```sh
gh attestation verify ttyak-0.12.1-x86_64-linux.zip -R dmytrobsp/ttyak --source-ref refs/tags/v0.12.1 \
  --signer-workflow dmytrobsp/ttyak/.github/workflows/release.yml
```

passes only for a file this repository's release workflow (`.github/workflows/release.yml`) built from the tag
`v0.12.1`; with another tag it fails ("expected SourceRepositoryRef to be refs/tags/…").

## If it doesn't work

- [Windows: SmartScreen warns about ttyak.exe](Troubleshooting#windows-smartscreen-warns-about-ttyakexe)
- [Reporting a bug](Troubleshooting#reporting-a-bug)
- everything else: [Troubleshooting](Troubleshooting)
