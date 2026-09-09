//! Script de build de Faraday.
//!
//! Intègre l'icône applicative et les métadonnées Windows (version, produit)
//! dans les exécutables via `winres`. L'icône est générée par
//! `packaging/make-icon.ps1` → `resources/icons/faraday.ico`.

#[cfg(target_os = "windows")]
fn main() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").map(|s| s == "windows").unwrap_or(false) {
        let mut res = winres::WindowsResource::new();

        // Icône du navigateur (générée par packaging/make-icon.ps1).
        res.set_icon("resources/icons/faraday.ico");

        // Métadonnées de version / produit (affichées par Explorer, SmartScreen…).
        let version = env!("CARGO_PKG_VERSION");
        res.set("FileVersion", version);
        res.set("ProductVersion", version);
        res.set("ProductName", "Faraday");
        res.set("FileDescription", "Faraday — navigateur respectueux de la vie privée");
        res.set("OriginalFilename", "faraday.exe");
        res.set("InternalName", "faraday");
        res.set("LegalCopyright", "© Faraday");

        // Manifeste : exécution sans élévation (asInvoker) + compatibilité
        // Windows 10/11 (indispensable pour les exécutables modernes).
        res.set_manifest(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="*" name="Faraday.App" type="win32"/>
  <description>Faraday</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>"#,
        );

        if let Err(e) = res.compile() {
            eprintln!("[faraday] build.rs: échec winres ({e})");
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
