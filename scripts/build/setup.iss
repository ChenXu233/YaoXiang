; YaoXiang Inno Setup Script

#define MyAppName "YaoXiang"
#define MyAppVersion "0.7.0"
#define MyAppPublisher "ChenXu233"
#define MyAppURL "https://github.com/ChenXu233/yaoxiang"
#define MyAppExeName "yaoxiang.exe"

[Setup]
; Application identity
AppId={{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; Install location
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes

; Output settings
OutputDir=..\dist
OutputBaseFilename=YaoXiang-Setup-{#MyAppVersion}
; SetupIconFile= ; 请在此处填写图标路径，如果不需要可留空

; Compression
Compression=lzma2
SolidCompression=yes

; UI settings
WizardStyle=modern
WizardSizePercent=100

; Privileges
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
ChangesEnvironment=yes

; Uninstaller
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

[Languages]
Name: "chinesesimplified"; MessagesFile: "ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; OnlyBelowVersion: 6.1; Check: not IsAdminInstallMode
Name: "addtopath"; Description: "Add to system PATH"; GroupDescription: "Other options:"; Flags: unchecked

[Files]
; Main executable
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]

// 兼容性函数：StrSplit (如果 Inno Setup 版本较旧，使用此实现)
function StrSplit(const S, Separator: string): TArrayOfString;
var
  I, L, StartPos: Integer;
  Count: Integer;
begin
  // 预估数组大小以优化性能，这里简单处理
  SetArrayLength(Result, 0);
  if S = '' then Exit;
  
  StartPos := 1;
  L := Length(S);
  I := 1;
  Count := 0;
  
  while I <= L do
  begin
    if Copy(S, I, Length(Separator)) = Separator then
    begin
      SetArrayLength(Result, Count + 1);
      Result[Count] := Copy(S, StartPos, I - StartPos);
      StartPos := I + Length(Separator);
      I := StartPos;
      Count := Count + 1;
    end
    else
      I := I + 1;
  end;
  
  // 添加最后一部分
  SetArrayLength(Result, Count + 1);
  Result[Count] := Copy(S, StartPos, L - StartPos + 1);
end;

// 外部函数声明：修正了参数类型以支持 64 位
function SendMessageTimeout(hWnd: HWND; Msg: Cardinal; wParam: Longint; lParam: string; fuFlags: Cardinal; uTimeout: Cardinal; var lpdwResult: Cardinal): Cardinal;
  external 'SendMessageTimeoutW@user32.dll stdcall';

const
  SMTO_ABORTIFHUNG = 2;
  WM_WININICHANGE = $001A;
  WM_SETTINGCHANGE = WM_WININICHANGE;

// 刷新环境变量，使 PATH 修改立即生效
procedure RefreshEnvironment;
var
  Res: Cardinal;
begin
  // 传递 "Environment" 字符串以通知系统刷新环境变量
  SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, 0,
    'Environment', SMTO_ABORTIFHUNG, 5000, Res);
end;

// 检查当前 PATH 中是否已包含指定路径（不区分大小写）
function PathContains(CurrentPath, AppPath: string): Boolean;
var
  List: TArrayOfString;
  I: Integer;
begin
  Result := False;
  if CurrentPath = '' then Exit;
  List := StrSplit(CurrentPath, ';');
  for I := 0 to GetArrayLength(List) - 1 do
    if CompareText(Trim(List[I]), AppPath) = 0 then
    begin
      Result := True;
      Exit;
    end;
end;

function GetEnvKeyByRoot(RegRoot: Integer): string;
begin
  if RegRoot = HKLM then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

procedure WritePathValue(RegRoot: Integer; const Data: string);
begin
  RegWriteExpandStringValue(RegRoot, GetEnvKeyByRoot(RegRoot), 'Path', Data);
end;

function TryReadPathValue(RegRoot: Integer; var Data: string): Boolean;
begin
  Result := RegQueryStringValue(RegRoot, GetEnvKeyByRoot(RegRoot), 'Path', Data);
end;

// 安装后处理：如果用户选择了“添加到 PATH”，则安全追加
procedure CurStepChanged(CurStep: TSetupStep);
var
  OldPath: string;
  AppPath: string;
  RegRoot: Integer;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
  begin
    AppPath := ExpandConstant('{app}');
    
    // 修改：根据安装模式决定修改用户环境变量还是系统环境变量
    if IsAdminInstallMode then
      RegRoot := HKLM
    else
      RegRoot := HKCU;

    // 尝试读取 PATH
    if TryReadPathValue(RegRoot, OldPath) then
    begin
      if not PathContains(OldPath, AppPath) then
      begin
        if OldPath <> '' then
          OldPath := OldPath + ';' + AppPath
        else
          OldPath := AppPath;
        WritePathValue(RegRoot, OldPath);
        RefreshEnvironment;
      end;
    end
    else
    begin
      // 如果 PATH 变量不存在（极少见），直接创建
      WritePathValue(RegRoot, AppPath);
      RefreshEnvironment;
    end;
  end;
end;

// 卸载时安全移除应用程序路径
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  CurrentPath: string;
  AppPath: string;
  NewPath: string;
  List: TArrayOfString;
  I: Integer;
  RegRoot: Integer;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    AppPath := ExpandConstant('{app}');
    
    // 卸载时也需要判断是 HKLM 还是 HKCU，通常卸载程序继承了安装时的权限上下文，
    // 但为了保险起见，这里也加上判断。
    // 注意：卸载时 IsAdminInstallMode 可能不可靠，通常建议检查注册表中是否存在该键值
    // 或者直接尝试 HKLM，失败再尝试 HKCU。这里为了逻辑清晰，尝试两者。
    
    // 尝试系统路径
    if RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', CurrentPath) then
      RegRoot := HKLM
    // 尝试用户路径
    else if RegQueryStringValue(HKCU, 'Environment', 'Path', CurrentPath) then
      RegRoot := HKCU
    else
      Exit; // 都找不到，退出

    List := StrSplit(CurrentPath, ';');
    NewPath := '';
    for I := 0 to GetArrayLength(List) - 1 do
    begin
      // 跳过与应用程序路径完全匹配的项（忽略前后空格）
      if CompareText(Trim(List[I]), AppPath) <> 0 then
      begin
        if NewPath <> '' then
          NewPath := NewPath + ';' + Trim(List[I])
        else
          NewPath := Trim(List[I]);
      end;
    end;
    
    // 写回或删除
    if NewPath = '' then
    begin
      RegDeleteValue(RegRoot, GetEnvKeyByRoot(RegRoot), 'Path');
    end
    else
    begin
      WritePathValue(RegRoot, NewPath);
    end;
    
    RefreshEnvironment;
  end;
end;
