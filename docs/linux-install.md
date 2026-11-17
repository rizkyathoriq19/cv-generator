# Installing the Linux build

The Debian package and AppImage are built against Ubuntu 24.04 LTS x64. Other
distributions may work but are not the tested baseline.

## Debian package

```bash
sudo apt install "./CV Generator_0.1.0_amd64.deb"
```

The package depends on the distribution's WebKitGTK 4.1 and GTK 3 runtime
(`libwebkit2gtk-4.1-0`, `libgtk-3-0`, resolved as `libgtk-3-0t64` on Ubuntu 24.04).
The first installation therefore needs access to the package archive, or an
already populated apt cache, to install those dependencies. After they are present
the application runs without a network connection. This package is not air-gapped
provisioning: it does not embed the system runtime libraries.

## AppImage

Make the file executable and run it:

```bash
chmod +x "CV Generator_0.1.0_amd64.AppImage"
./"CV Generator_0.1.0_amd64.AppImage"
```

The AppImage bundles the application and its non-system libraries; a system
WebKitGTK/GTK runtime is still expected on distributions other than the tested
baseline.

## Local data

CV projects, their previous revisions and the Trash live in the local application
data directory as unencrypted plain files. Exported PDFs and project backups are
unencrypted plain files wherever the file dialog points. This release has no
account, no backend, no telemetry and no automatic updater.
