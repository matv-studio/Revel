# Code signing policy

Status: Revel has applied, or is about to apply, to the SignPath Foundation open source program. Until the
application is accepted, Windows installers are **not** code signed. Once it is accepted, the Windows
installers attached to the [releases](https://github.com/matv-studio/Revel/releases) will be signed as
follows:

> Free code signing provided by [SignPath.io](https://signpath.io/), certificate by
> [SignPath Foundation](https://signpath.org/).

The publisher shown by Windows will then be "SignPath Foundation".

## Scope

- Signed: the Windows installers of Revel (`.exe` NSIS installer, `.msi`) and the executables they contain
  that are built from this repository.
- Not signed with this certificate: third-party binaries, macOS and Linux packages.
- Update packages are, in addition, signed with Revel's own update key (minisign, checked by the application
  before any update is installed). This is independent from Windows code signing.

## Source and build

- Every signed file is built by GitHub Actions (GitHub-hosted runners) from the public repository
  [`matv-studio/Revel`](https://github.com/matv-studio/Revel), workflow
  [`.github/workflows/release.yml`](../.github/workflows/release.yml), at a tagged commit of `main`.
- No binary built on a personal machine is signed.
- Releases are prepared as drafts; a signing request is approved manually for each release.

## Team roles

| Role | Members |
| --- | --- |
| Authors (committers) | [Alunites](https://github.com/Alunites) |
| Reviewers | [Alunites](https://github.com/Alunites) |
| Approvers | [Alunites](https://github.com/Alunites) |

Changes proposed by anyone else go through a pull request reviewed by a reviewer. All team members use
multi-factor authentication on GitHub and SignPath.

## Privacy

Revel has no account, no telemetry and no analytics. Sites are stored on the user's computer. Revel connects
to other systems only in the cases below.

- **Update check**: at startup and from the settings, Revel reads the public release list of this repository
  on GitHub (`github.com`) to know whether a newer version exists. Nothing about the user or their sites is
  sent; as for any web request, GitHub sees the IP address. An update is downloaded and installed only when
  the user asks for it.
- **Actions requested by the user**: publishing a site to the host the user configured (SFTP, FTP, Git),
  importing a site from an address the user gives, downloading fonts (Google Fonts), translation models
  (Hugging Face) or Node.js (nodejs.org) when the user uses the corresponding function.

Third-party services reached by these functions apply their own privacy policies:
[GitHub](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement),
[Google Fonts](https://developers.google.com/fonts/faq/privacy),
[Hugging Face](https://huggingface.co/privacy), [Node.js](https://nodejs.org/).

## Uninstall

Revel is removed with the uninstaller registered in Windows (Settings › Apps), which deletes the installed
files. Sites created by the user are left untouched.

## Licence

Revel is distributed under the [MIT licence](../LICENSE). Third-party components and their licences are listed
in [LICENCES-TIERCES.md](LICENCES-TIERCES.md).

## Reporting

A signed file that looks wrong, or a suspected misuse of the certificate, can be reported in the
[issues](https://github.com/matv-studio/Revel/issues) of the repository or to
[SignPath Foundation](https://signpath.org/).
