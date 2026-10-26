# Turns on automatic sign-in of a test user, so the Windows test machine has a desktop session after each start
# (an ssh session has none). Run it once, by hand, in an elevated PowerShell on that machine:
#   autologon.ps1 [-User <name>]      (default: the user running it)
# Asks for the password at a hidden prompt, checks it, then writes the Winlogon keys. Windows keeps that password in
# the registry in plain text: use it only on a test machine, with a password used nowhere else.
# Undo: Set-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' AutoAdminLogon '0'
#       Remove-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' DefaultPassword
param([string]$User = $env:USERNAME)
$ErrorActionPreference = 'Stop'
$sec = Read-Host -AsSecureString "Password of $User"
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec)
$plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
[Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
Add-Type -AssemblyName System.DirectoryServices.AccountManagement
$ctx = New-Object System.DirectoryServices.AccountManagement.PrincipalContext('Machine')
if (-not $ctx.ValidateCredentials($User, $plain)) { Write-Host 'Wrong password - nothing changed.'; exit 1 }
$k = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
Set-ItemProperty $k 'AutoAdminLogon' '1'
Set-ItemProperty $k 'DefaultUserName' $User
Set-ItemProperty $k 'DefaultDomainName' $env:COMPUTERNAME
Set-ItemProperty $k 'DefaultPassword' $plain
# Windows Hello's "passwordless only" setting blocks automatic sign-in.
Set-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\PasswordLess\Device' 'DevicePasswordLessBuildVersion' 0 -Type DWord
Write-Host "Automatic sign-in is on for $User (takes effect at the next restart)."
