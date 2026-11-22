// The setup's Windows manifest. Declaring the Windows versions it was made
// for (and that it runs with the user's normal rights) tells Windows it is a
// modern program: without it, Windows treats anything named "Setup" or
// "Uninstall" as an old installer and afterwards shows "This program might
// not have uninstalled correctly" (Program Compatibility Assistant).
const MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" />
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}" />
      <!-- Windows 7 -->
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}" />
    </application>
  </compatibility>
</assembly>"#;

fn main() {
    // the installer and the uninstaller carried inside (see the build script)
    for name in ["VS_PAYLOAD", "VS_UNINSTALLER"] {
        println!("cargo:rerun-if-env-changed={name}");
        if let Ok(path) = std::env::var(name) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    let windows = tauri_build::WindowsAttributes::new().app_manifest(MANIFEST);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build script failed");
}