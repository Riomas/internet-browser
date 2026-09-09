 ; Faraday — Installateur Windows (Inno Setup 6)
; -----------------------------------------------
; Compilation : ISCC.exe faraday.iss  (ou packaging\build-release.ps1 -Inno)
; Le script consomme le dossier regroupé dist\Faraday (généré par build-release.ps1).
; Installation PAR UTILISATEUR (aucun admin / UAC requis) — adapté à une
; distribution non signée. Le dossier de données reste dans %APPDATA%\Faraday.

#ifndef FaradayVersion
  #define FaradayVersion "0.1.0"
#endif

#define AppName      "Faraday"
#define AppPublisher "Faraday"
#define AppURL       "https://example.invalid/faraday"   ; TODO : page de téléchargement
#define AppExe       "faraday.exe"

[Setup]
AppId={{EE59FA4E-61F4-40FE-81FD-4F5FB911A71B}
AppName={#AppName}
AppVersion={#FaradayVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={localappdata}\Faraday
DisableProgramGroupPage=yes
; Installation sans élévation : dans le profil utilisateur.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=Faraday-Setup-{#FaradayVersion}-x64
SetupIconFile=..\crates\faraday\resources\icons\faraday.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
; Ne pas supprimer les données utilisateur à la désinstallation par défaut :
; elles sont dans %APPDATA%\Faraday et ne seront effacées que si l'utilisateur
; coche l'option dédiée.
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExe}

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "deleteuserdata"; Description: "Supprimer aussi les données de navigation (%APPDATA%\Faraday)"; GroupDescription: "Désinstallation:"; Flags: unchecked

[Files]
; Tout le dossier applicatif regroupé (binaires Faraday + CEF + locales).
Source: "..\dist\Faraday\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Suppression optionnelle des données de navigation à la désinstallation.
Type: filesandordirs; Name: "{localappdata}\Faraday"; Tasks: deleteuserdata

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    // Premier lancement : Faraday crée lui-même son profil (%APPDATA%\Faraday)
    // et sa config privacy à partir des valeurs embarquées. Rien à faire ici.
  end;
end;
