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
// ---------------------------------------------------------------------------
// Approbation du certificat de signature (version auto-signee).
//
// Le certificat public (faraday-certificat.cer) et son utilitaire sont joints au
// lot applicatif. L'utilisateur choisit explicitement de l'approuver dans SON
// magasin de certificats (aucun droit administrateur) ; le retrait est
// automatique a la desinstallation. Sans cette approbation, Windows affiche un
// editeur inconnu et le bootstrap de Chromium refuse de demarrer (signature non
// fiable).
// ---------------------------------------------------------------------------
var
  PageCertificat: TInputOptionWizardPage;

function ScriptCertificat(): String;
begin
  Result := ExpandConstant('{app}\faraday-certificat.ps1');
end;

function CertificatDisponible(): Boolean;
begin
  Result := FileExists(ExpandConstant('{app}\faraday-certificat.cer')) and
            FileExists(ScriptCertificat());
end;

procedure InitializeWizard();
begin
  PageCertificat := CreateInputOptionPage(wpSelectTasks,
    'Certificat de signature Faraday',
    'Windows ne connait pas encore l''editeur de cette application.',
    'Cette version de Faraday est signee par un certificat propre au projet, pas encore ' +
    'delivre par une autorite de certification. Pour que Windows affiche le bon editeur, ' +
    'et pour que l''application signee puisse demarrer sur cet ordinateur, ce certificat ' +
    'doit etre approuve.' + #13#10 + #13#10 +
    'Il sera ajoute uniquement a VOS certificats de confiance (compte utilisateur courant, ' +
    'aucun droit administrateur) et retire automatiquement lors de la desinstallation.' + #13#10 + #13#10 +
    'Tu peux decocher cette case et installer le certificat plus tard en lancant ' +
    'faraday-certificat.ps1 depuis le dossier d''installation.',
    False, False);
  PageCertificat.Add('Approuver le certificat Faraday sur ce compte utilisateur (recommande)');
  PageCertificat.Values[0] := True;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  // Rien a approuver si le certificat n'est pas joint au lot.
  Result := (PageID = PageCertificat.ID) and not CertificatDisponible();
end;

function ApprobationDemandee(): Boolean;
begin
  Result := (PageCertificat <> nil) and PageCertificat.Values[0] and CertificatDisponible();
end;

function ExecCertificat(Options: String): Integer;
var
  Code: Integer;
begin
  Code := -1;
  Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
       '-NoProfile -ExecutionPolicy Bypass -File "' + ScriptCertificat() + '" ' + Options,
       '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := Code;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if ApprobationDemandee() then
    begin
      if ExecCertificat('-Install') <> 0 then
      begin
        MsgBox('Le certificat Faraday n''a pas pu etre approuve automatiquement.' + #13#10 + #13#10 +
               'Tu peux le faire plus tard, sans droits administrateur :' + #13#10 +
               '  powershell -ExecutionPolicy Bypass -File "' + ScriptCertificat() + '"',
               mbInformation, MB_OK);
      end;
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    // Sans effet si le certificat n'est pas present : on ne touche qu'a celui-ci.
    ExecCertificat('-Remove');
  end;
end;
