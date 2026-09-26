; installer/windows/liphia.iss
;
; Windows installer for Liphia (Inno Setup 6).
;
; Built by the release-engine workflow, which passes:
;   /DAppVersion=2.0.0            engine version (from the tag)
;   /DStageDir=<folder>          folder with liphia.exe, liphia-gui.exe,
;                                 LICENSE.txt, THIRD_PARTY_LICENSES.txt, README.md
;
; Local build (from the repo root, after `cargo build --release` and
; `cargo build -p liphia_cli_gui --release`, with the files gathered in
; one folder):
;   iscc /DAppVersion=2.0.0 /DStageDir=C:\tmp\liphia-dist installer\windows\liphia.iss
;
; Install modes: per user by default (no admin, %LOCALAPPDATA%\Programs\Liphia);
; the first page lets the user choose "all users" instead (Program Files,
; requires admin). The bin folder is added to that scope's PATH.

#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif
#ifndef StageDir
  #define StageDir "..\..\dist\staging"
#endif

#define AppName      "Liphia"
#define AppPublisher "Sergio H. Ferreira"
#define AppURL       "https://github.com/shferreira-lab/liphia"

[Setup]
; AppId identifies the product for upgrades and uninstall; never change it.
AppId={{E7A851EE-5036-4D35-9C3B-39275671A1DD}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE.txt
ChangesEnvironment=yes
ChangesAssociations=yes
UninstallDisplayIcon={app}\bin\liphia.exe
OutputDir=..\..\dist
OutputBaseFilename=liphia-{#AppVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
#ifexist "liphia.ico"
SetupIconFile=liphia.ico
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Types]
Name: "full"; Description: "CLI and GUI"
Name: "cli"; Description: "CLI only"
Name: "custom"; Description: "Custom"; Flags: iscustom

[Components]
Name: "cli"; Description: "liphia — compiler, VM, REPL and package manager"; Types: full cli custom; Flags: fixed
Name: "gui"; Description: "liphia-gui — runtime for windowed programs"; Types: full

[Tasks]
Name: "addtopath"; Description: "Add Liphia to PATH (run 'liphia' from any terminal or editor)"
Name: "assoc"; Description: "Show the Liphia icon on .lph files"

[Files]
Source: "{#StageDir}\liphia.exe"; DestDir: "{app}\bin"; Components: cli; Flags: ignoreversion
Source: "{#StageDir}\liphia-gui.exe"; DestDir: "{app}\bin"; Components: gui; Flags: ignoreversion
Source: "{#StageDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#StageDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
#ifexist "liphia.ico"
Source: "liphia.ico"; DestDir: "{app}"; Flags: ignoreversion
#endif

; .lph files get an icon and a description, but double-click does not run
; them: running code on double-click is a risk and the expected workflow is
; a terminal or an editor.
[Registry]
Root: HKA; Subkey: "Software\Classes\.lph"; ValueType: string; ValueName: ""; ValueData: "Liphia.Source"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\Liphia.Source"; ValueType: string; ValueName: ""; ValueData: "Liphia source file"; Flags: uninsdeletekey; Tasks: assoc
#ifexist "liphia.ico"
Root: HKA; Subkey: "Software\Classes\Liphia.Source\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\liphia.ico"; Tasks: assoc
#endif

[Messages]
english.FinishedLabel=Liphia was installed.%n%nOpen a new terminal and run "liphia version" to check it. Terminals and editors that were already open (including VS Code) must be restarted to see the updated PATH.
brazilianportuguese.FinishedLabel=O Liphia foi instalado.%n%nAbra um novo terminal e rode "liphia version" para conferir. Terminais e editores que já estavam abertos (inclusive o VS Code) precisam ser reiniciados para enxergar o PATH atualizado.

[Code]
// PATH handling. The bin folder is appended to the user PATH (per-user
// install) or the system PATH (all-users install), and removed again on
// uninstall. Existing entries are compared case-insensitively so running
// the installer twice never duplicates the entry.

const
  UserEnvKey   = 'Environment';
  SystemEnvKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';

function EnvRoot(): Integer;
begin
  if IsAdminInstallMode then
    Result := HKEY_LOCAL_MACHINE
  else
    Result := HKEY_CURRENT_USER;
end;

function EnvKey(): String;
begin
  if IsAdminInstallMode then
    Result := SystemEnvKey
  else
    Result := UserEnvKey;
end;

function BinDir(): String;
begin
  Result := ExpandConstant('{app}\bin');
end;

function PathContains(Path, Dir: String): Boolean;
begin
  Result := Pos(';' + Lowercase(Dir) + ';', ';' + Lowercase(Path) + ';') > 0;
end;

procedure AddToPath();
var
  Path: String;
begin
  if not RegQueryStringValue(EnvRoot(), EnvKey(), 'Path', Path) then
    Path := '';
  if PathContains(Path, BinDir()) then
    exit;
  if (Path <> '') and (Copy(Path, Length(Path), 1) <> ';') then
    Path := Path + ';';
  RegWriteExpandStringValue(EnvRoot(), EnvKey(), 'Path', Path + BinDir());
end;

procedure RemoveFromPath();
var
  Path, Dir: String;
  P: Integer;
begin
  if not RegQueryStringValue(EnvRoot(), EnvKey(), 'Path', Path) then
    exit;
  Dir := BinDir();
  Path := ';' + Path + ';';
  P := Pos(';' + Lowercase(Dir) + ';', Lowercase(Path));
  if P = 0 then
    exit;
  Delete(Path, P, Length(Dir) + 1);
  Path := Copy(Path, 2, Length(Path) - 2);
  RegWriteExpandStringValue(EnvRoot(), EnvKey(), 'Path', Path);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath();
end;
