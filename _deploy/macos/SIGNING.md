# Signing and notarizing the macOS app

The workflow `.github/workflows/build-release.yaml` can sign the app with a *Developer ID Application*
certificate and notarize it with Apple, so macOS opens it without any warning. It needs a paid Apple
Developer account.

Everything is optional:

| Secrets set | Result |
| --- | --- |
| none | Signed ad hoc. After downloading, macOS blocks the app. Run `xattr -dr com.apple.quarantine "ABWRT.app"` once. |
| certificate only | Signed with your Developer ID, but not notarized. macOS still warns on downloaded copies. |
| certificate and notarization | Signed and notarized. Opens without a warning. |

## Secrets

Add these in the repository under *Settings → Secrets and variables → Actions → New repository secret*.
Never commit them or paste them into issues and chats.

| Secret | Content |
| --- | --- |
| `MACOS_CERTIFICATE` | The *Developer ID Application* certificate with its private key as a base64 encoded `.p12` file |
| `MACOS_CERTIFICATE_PASSWORD` | The password you chose when exporting the `.p12` file |
| `APPLE_ID` | The e-mail address of your Apple ID |
| `APPLE_APP_SPECIFIC_PASSWORD` | An app-specific password for that Apple ID |
| `APPLE_TEAM_ID` | The 10 character team ID of your developer account |

### Certificate

1. In Xcode open *Settings → Accounts*, select your team and click *Manage Certificates*. Add a
   *Developer ID Application* certificate with the `+` button. (It can also be created on
   <https://developer.apple.com/account/resources/certificates>. Only the account holder can create
   Developer ID certificates.)
2. Open *Keychain Access*, find the certificate under *My Certificates* (it must have a private key below it),
   right click it and choose *Export...*. Save it as a `.p12` file with a password.
3. Convert it to base64 and copy it to the clipboard:
   `base64 -i Certificates.p12 | pbcopy`
   and paste it as the value of `MACOS_CERTIFICATE`.

### Notarization

1. On <https://account.apple.com> open *Sign-In and Security → App-Specific Passwords* and create one.
2. Your team ID is shown on <https://developer.apple.com/account> under *Membership details*.

## Running

Push to a `feature/**` branch or start *Build and release* manually in the *Actions* tab. Notarization usually
takes a few minutes. The result is the artifact `abwrt-macos-apple-silicon` (and `-intel`).
Secrets are not available to workflows triggered from pull requests of forks.
