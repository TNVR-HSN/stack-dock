[Setup]
AppName=StackDock Native
AppVersion=0.2.0 beta
AppPublisher=Stackdock by Tanveer
VersionInfoVersion=0.2.0.0
DefaultDirName={autopf}\StackDock Native
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
DisableDirPage=no
DisableProgramGroupPage=yes
UsePreviousAppDir=no
SetupLogging=yes
Uninstallable=no
CreateUninstallRegKey=no
WizardStyle=modern
SetupIconFile={#NativeIcon}
WizardSmallImageFile=wizard-header.bmp
WizardImageFile=wizard-side.bmp
OutputDir={#OutputFolder}
OutputBaseFilename=StackDock-Native-0.2.0-beta-x64-Setup
Compression=none
DiskSpanning=no

[Files]
Source: "{#Msi}"; DestDir: "{tmp}"; DestName: "StackDock-Native.msi"; Flags: deleteafterinstall

[Code]
var
  ShortcutBox: TNewCheckBox;
  RebootNeeded: Boolean;

function MsiEnumRelatedProducts(UpgradeCode: String; Reserved, Index: Cardinal; ProductCode: String): Cardinal;
  external 'MsiEnumRelatedProductsW@msi.dll stdcall';

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ProductCode: String;
  Status: Cardinal;
begin
  SetLength(ProductCode, 39);
  Status := MsiEnumRelatedProducts('{B52B1CA2-917A-4539-BD02-29BA02D0A6B2}', 0, 0, ProductCode);
  if Status = 0 then
    Result := 'StackDock Native is already installed. To install this beta in another folder, uninstall the existing copy from Windows Installed apps first.'
  else if Status <> 259 then
    Result := Format('Could not check the existing Windows Installer registration (%d).', [Status])
  else
    Result := '';
end;

procedure InitializeWizard;
begin
  ShortcutBox := TNewCheckBox.Create(WizardForm);
  ShortcutBox.Parent := WizardForm.FinishedPage;
  ShortcutBox.Left := WizardForm.FinishedLabel.Left;
  ShortcutBox.Width := WizardForm.FinishedLabel.Width;
  ShortcutBox.Height := ScaleY(24);
  ShortcutBox.Caption := 'Create a desktop shortcut for all users';
  ShortcutBox.Checked := True;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpFinished then begin
    WizardForm.AdjustLabelHeight(WizardForm.FinishedLabel);
    ShortcutBox.Top := WizardForm.FinishedLabel.Top + WizardForm.FinishedLabel.Height + ScaleY(12);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Status: Integer;
  Args: String;
begin
  if CurStep <> ssPostInstall then Exit;
  WizardForm.StatusLabel.Caption := 'Installing StackDock Native...';
  Args := '/i "' + ExpandConstant('{tmp}\StackDock-Native.msi') +
          '" /qn /norestart INSTALLFOLDER="' + RemoveBackslashUnlessRoot(ExpandConstant('{app}')) +
          '" /L*v "' + ExpandConstant('{log}') + '.msi.log"';
  if not Exec(ExpandConstant('{sys}\msiexec.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, Status) then
    RaiseException(SysErrorMessage(Status));
  if (Status <> 0) and (Status <> 3010) then
    RaiseException(Format('Windows Installer returned %d. See %s.msi.log', [Status, ExpandConstant('{log}')]));
  RebootNeeded := Status = 3010;
end;

function NeedRestart: Boolean;
begin
  Result := RebootNeeded;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  Link, Target: String;
begin
  Result := True;
  if (CurPageID <> wpFinished) or WizardSilent or not ShortcutBox.Checked then Exit;
  Target := ExpandConstant('{app}\stack-dock-native.exe');
  Link := ExpandConstant('{commondesktop}\StackDock Native.lnk');
  try
    CreateShellLink(Link, 'StackDock Native', Target, '', ExpandConstant('{app}'), Target, 0, SW_SHOWNORMAL);
  except
    MsgBox('StackDock Native was installed. The desktop shortcut could not be created: ' + GetExceptionMessage, mbError, MB_OK);
  end;
end;
