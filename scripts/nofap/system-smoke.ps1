#Requires -Version 7.0
<#
.SYNOPSIS
System smoke test of the ubiqx-shield helper on Windows (docs/nofap/DESIGN.md 9.3).

.DESCRIPTION
THIS SCRIPT CHANGES THE MACHINE IT RUNS ON. It runs the real helper with administrator
rights: it writes the marked block into the hosts file and, for the resolver tier, points
the DNS of every adapter that is up at the family resolver; it checks that both hold; then
it takes everything off again and checks that the machine is as it was. It is meant for a
CI runner (the job `nofap-system-smoke`). It is not for a machine in use.

It therefore refuses to start unless ALL of this is true:
  - it runs elevated, as a member of Administrators;
  - `CI` is `true` or `UBIQX_SHIELD_SMOKE` is `1` in the environment, so nobody starts it by
    accident;
  - the machine carries nothing of the product yet: no marked block in the hosts file and no
    state directory. A machine where the product is in use would lose its protection to the
    removal step, so the script does not touch it.

What a run does:
  1. Save: the bytes of the hosts file with its access list, the DNS servers of every
     adapter that is up (as the cmdlets report them and as the registry holds them), and
     what `status` prints.
  2. `apply` a plan for the chosen tiers (owner `shield`, no extra names), passing
     `--unverified <tiers>` because the gate is closed until a run like this one is green.
  3. Check: the block is in the file, intact, with the hash and the count it declares; every
     byte in front of it is the saved file; the state file agrees and its directory under
     Program Files can be read but not written by a standard user; the canary and a name of
     the list are blocked for a program that asks the operating system (Windows answers
     "valid name, no data" where other systems answer the null address), the first such
     lookup within 10 seconds (the measurement behind the cap of names on Windows), and
     the helper's `status` reads the canary as the app's own check needs it; another name
     still resolves. Resolver
     tier: every adapter that is up uses the provider's set, the registry value the quick
     read relies on holds it, both IPv4 resolvers answer and refuse a listed name.
  4. `remove --owner all --scope all`, then check the restore: the hosts file is byte for
     byte the saved one with the same access list, DNS is what it was, the state directory
     is gone.
  5. The elevation command the app would run (printed by the helper's `elevate-argv` verb,
     which only prints) is run for real, in the four-variable environment the app gives it,
     with plans of 1,800, 4,000, 12,000 and 24,000 characters, and the longest that reaches
     the helper intact is reported. This is the measurement behind the argument budget on
     Windows. Before that, the same command with a plan argument the helper refuses must
     end with the helper's exit code: it is the only thing the app reads of the run.
  6. Steps 2 to 4 again with `windir`, `SystemRoot`, `ProgramData` and `PSModulePath`
     pointing at an empty scratch directory in the helper's environment: the result must be
     the same and nothing may appear in that directory.

If anything fails after step 1, or the run is interrupted, the script puts the saved state
back itself (hosts bytes, DNS, state directory) and says so; it reports, and does not
change, an owner or access list of the hosts file that differs. It exits 0 only when every
check passed and the helper's own removal restored the machine; 1 on any mismatch, also when
the fallback restore worked; 2 when it refused to run.

The expectations below (marker lines, hash rule, paths, the resolver's addresses, the plan's
shape) are written out again here on purpose and not read from the helper: the script
checks the helper against the design, not against itself.

.PARAMETER Tiers
Tiers to exercise: hosts, resolver (default: both). `-Tiers hosts,resolver` and
`-Tiers 'hosts,resolver'` mean the same.

.PARAMETER Helper
The helper binary (default: <target dir>\debug\ubiqx-shield-helper.exe, built with
`cargo build -p ubiqx-platform --bin ubiqx-shield-helper`). The app's own executable works
too; it is then run as `<exe> --shield-helper <verb>`.

.PARAMETER SkipElevation
Leaves out step 5.

.PARAMETER SkipHostileEnvironment
Leaves out step 6.

.EXAMPLE
pwsh -File scripts/nofap/system-smoke.ps1 -Tiers hosts,resolver
#>
[CmdletBinding()]
param(
    [string[]]$Tiers = @('hosts', 'resolver'),
    [string]$Helper = '',
    [switch]$SkipElevation,
    [switch]$SkipHostileEnvironment
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# What the design says the helper writes (docs/nofap/DESIGN.md 2.2, 2.3, 2.5). Stated here
# again, not read from the helper.
$Canary = 'canary.ubiqx-shield.invalid'
$BeginPrefix = '# >>> ubiqx-shield v1 BEGIN'
$EndMarker = '# <<< ubiqx-shield END'
$UndoHint = '# Added by the ubiqx-shield app on this computer. To undo, delete every line from BEGIN to END.'
$BeginLine = '^# >>> ubiqx-shield v1 BEGIN sha256=([0-9a-f]{64}) names=([0-9]+)$'
$NamesPerLine = 6
$CapWindows = 8000
$NullAddresses = @('0.0.0.0', '::')
# Winsock's two ways of saying that a name has no address: WSANO_DATA ("valid name, no data
# of the requested type") and WSAHOST_NOT_FOUND ("no such host is known").
$WsaNoData = 11004
$WsaHostNotFound = 11001
$NoData = 'no-data'
$Owner = 'shield'
$ResolverId = 'cloudflare_family'
$ResolverV4 = @('1.1.1.3', '1.0.0.3')
$ResolverV6 = @('2606:4700:4700::1113', '2606:4700:4700::1003')
# A name that has to keep resolving, and the name the resolver's operator publishes for
# testing its adult-content filter (answered with the null address by the family resolver only).
$Reachable = 'example.com'
$FilterTestName = 'nudity.testcategory.com'
$FirstLookupLimitSeconds = 10
$ElevationLengths = @(1800, 4000, 12000, 24000)
$ArgumentBudget = 24000
$ExitInvalidPlan = 2
$ExitGated = 5
# A `--plan-b64` value the helper refuses before it touches anything (exit $ExitInvalidPlan). Its
# characters are of the alphabet the elevation command accepts.
$RefusedPlan = 'not-a-plan'
$PaddingSuffix = '.smoke.ubiqx-shield.invalid'
$MaxExtraNames = 400

function Stop-Refused {
    param([Parameter(Mandatory)][string]$Why)
    [Console]::Error.WriteLine("system-smoke: refusing to run: $Why")
    exit 2
}

# ----------------------------------------------------------------------------------------------
# Refusals. Nothing in this section changes anything.

if (-not $IsWindows) {
    Stop-Refused 'this script is for Windows (Linux and macOS: system-smoke.sh)'
}

$TierList = @($Tiers | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' } | Select-Object -Unique)
if ($TierList.Count -eq 0 -or @($TierList | Where-Object { $_ -notin @('hosts', 'resolver') }).Count -gt 0) {
    Stop-Refused "-Tiers takes hosts, resolver or both (got: $($Tiers -join ','))"
}
$WithHosts = $TierList -contains 'hosts'
$WithResolver = $TierList -contains 'resolver'
$TierArgument = $TierList -join ','

$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Stop-Refused "it must run elevated, as an administrator (it runs the helper's privileged verbs)"
}

if ($env:CI -ne 'true' -and $env:UBIQX_SHIELD_SMOKE -ne '1') {
    Stop-Refused 'neither CI=true nor UBIQX_SHIELD_SMOKE=1 is set. This script changes the hosts file and system DNS; it is for a CI runner, not for a machine in use'
}

$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
if ($Helper -eq '') {
    $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $RepoRoot 'target' }
    $Helper = Join-Path $targetDir 'debug\ubiqx-shield-helper.exe'
}
if (-not (Test-Path -LiteralPath $Helper -PathType Leaf)) {
    Stop-Refused "no helper at $Helper (build it: cargo build -p ubiqx-platform --bin ubiqx-shield-helper)"
}
$Helper = (Resolve-Path -LiteralPath $Helper).Path

# The system directories as Windows itself reports them, never from an environment variable.
$System32 = [Environment]::SystemDirectory
$HostsPath = Join-Path $System32 'drivers\etc\hosts'
$StateDir = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'ubiqx-shield'
$StateFile = Join-Path $StateDir 'state.json'
# The whole environment the app gives the elevation command and every system tool it starts
# (design 2.5): four variables, each a path Windows itself reported. Step 5 runs the command
# with exactly these, so that what is measured is what the app does.
$WindowsDir = Split-Path -Parent $System32
$AppChildEnvironment = @{
    SystemRoot   = $WindowsDir
    windir       = $WindowsDir
    PATH         = $System32
    PSModulePath = (Join-Path $System32 'WindowsPowerShell\v1.0\Modules')
}

if (-not (Test-Path -LiteralPath $HostsPath -PathType Leaf)) {
    Stop-Refused "$HostsPath is not a file"
}
$Latin1 = [System.Text.Encoding]::GetEncoding(28591)
$SavedHosts = [System.IO.File]::ReadAllBytes($HostsPath)
$savedText = $Latin1.GetString($SavedHosts)
if ($savedText.Contains('>>> ubiqx-shield') -or $savedText.Contains('<<< ubiqx-shield')) {
    Stop-Refused "$HostsPath already carries a ubiqx-shield block. The product is in use on this machine, and this test would remove its protection"
}
if (Test-Path -LiteralPath $StateDir) {
    Stop-Refused "$StateDir exists. The product is in use on this machine, and this test would remove its protection"
}

# ----------------------------------------------------------------------------------------------
# Running a program as the app does: an argument list, never a command line put together by
# hand, and (for step 6) an environment the caller chooses.

function Start-Tool {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [hashtable]$Environment = @{},
        # The child gets $Environment and nothing else, as `Command::env_clear` does in the app.
        [switch]$OnlyEnvironment
    )
    $info = [System.Diagnostics.ProcessStartInfo]::new($FilePath)
    foreach ($argument in $Arguments) {
        $info.ArgumentList.Add($argument)
    }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    # The helper prints UTF-8 (the hosts text and adapter names inside its JSON).
    $info.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $info.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $info.CreateNoWindow = $true
    if ($OnlyEnvironment) {
        $info.Environment.Clear()
    }
    foreach ($name in $Environment.Keys) {
        $info.Environment[[string]$name] = [string]$Environment[$name]
    }
    $process = [System.Diagnostics.Process]::Start($info)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        return [pscustomobject]@{
            ExitCode = [int]$process.ExitCode
            StdOut   = [string]$stdout.GetAwaiter().GetResult()
            StdErr   = [string]$stderr.GetAwaiter().GetResult()
        }
    }
    finally {
        $process.Dispose()
    }
}

# How this binary takes a verb: the app's executable needs the helper flag in front, the
# small helper binary takes the verb directly. `status` is unprivileged and only reads. The
# bare form is tried only for a file named like the helper binary, so an app executable is
# never started as an app.
$HelperPrefix = $null
if ((Start-Tool -FilePath $Helper -Arguments @('--shield-helper', 'status')).ExitCode -eq 0) {
    $HelperPrefix = @('--shield-helper')
}
elseif ([System.IO.Path]::GetFileNameWithoutExtension($Helper) -eq 'ubiqx-shield-helper' -and (Start-Tool -FilePath $Helper -Arguments @('status')).ExitCode -eq 0) {
    $HelperPrefix = @()
}
else {
    Stop-Refused "$Helper does not answer the status verb"
}
$HelperForm = if ($HelperPrefix.Count -eq 0) { 'bare' } else { 'flag' }

function Invoke-Helper {
    param(
        [Parameter(Mandatory)][string[]]$Arguments,
        [hashtable]$Environment = @{}
    )
    return Start-Tool -FilePath $Helper -Arguments (@($HelperPrefix) + $Arguments) -Environment $Environment
}

# ----------------------------------------------------------------------------------------------
# Reporting

$script:Failures = 0
$script:Changed = $false
$script:Restored = $false
$script:Evidence = [ordered]@{}

function Write-Step {
    param([string]$Text)
    Write-Host "system-smoke: $Text"
}

function Write-Note {
    param([string]$Text)
    Write-Host "  note  $Text"
}

function Assert-That {
    param(
        [Parameter(Mandatory)][bool]$Passed,
        [Parameter(Mandatory)][string]$What,
        $Found = $null
    )
    if ($Passed) {
        Write-Host "  ok    $What"
    }
    else {
        $script:Failures += 1
        $suffix = if ($null -eq $Found) { '' } else { " (found: $(@($Found) -join ', '))" }
        Write-Host "  FAIL  $What$suffix"
    }
}

function Get-Field {
    # $Value.a.b... or $null when a step is missing (strict mode makes a plain read throw).
    param($Value, [string[]]$Path)
    foreach ($name in $Path) {
        if ($null -eq $Value) { return $null }
        $property = $Value.PSObject.Properties[$name]
        if ($null -eq $property) { return $null }
        $Value = $property.Value
    }
    return $Value
}

function Get-Status {
    param([hashtable]$Environment = @{})
    $result = Invoke-Helper -Arguments @('status') -Environment $Environment
    if ($result.ExitCode -ne 0) {
        Assert-That $false "status exits 0" $result.ExitCode
        return $null
    }
    try {
        return ($result.StdOut | ConvertFrom-Json)
    }
    catch {
        Assert-That $false 'status prints JSON' $_.Exception.Message
        return $null
    }
}

# ----------------------------------------------------------------------------------------------
# Bytes and the hosts block

function Test-StartsWith {
    param([byte[]]$Bytes, [byte[]]$Prefix)
    if ($Bytes.Length -lt $Prefix.Length) { return $false }
    for ($i = 0; $i -lt $Prefix.Length; $i++) {
        if ($Bytes[$i] -ne $Prefix[$i]) { return $false }
    }
    return $true
}

function Test-SameBytes {
    param([byte[]]$Left, [byte[]]$Right)
    return ($Left.Length -eq $Right.Length) -and (Test-StartsWith -Bytes $Left -Prefix $Right)
}

function Get-Sha256Hex {
    param([byte[]]$Bytes)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return -join ($sha.ComputeHash($Bytes) | ForEach-Object { $_.ToString('x2') })
    }
    finally {
        $sha.Dispose()
    }
}

function Get-ShieldBlock {
    # The one block of a hosts file: what it declares and what it holds. $null, after a FAIL
    # line, when the file does not hold exactly one block of the shape the design describes.
    param([byte[]]$Bytes)
    $lines = @($Latin1.GetString($Bytes) -split "`n" | ForEach-Object { $_.TrimEnd("`r") })
    $begins = @(for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i].StartsWith($BeginPrefix)) { $i } })
    $ends = @(for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -ceq $EndMarker) { $i } })
    if ($begins.Count -ne 1 -or $ends.Count -ne 1 -or $ends[0] -le $begins[0]) {
        Assert-That $false 'hosts: one BEGIN line followed by one END line' "$($begins.Count) BEGIN, $($ends.Count) END"
        return $null
    }
    $declared = [regex]::Match($lines[$begins[0]], $BeginLine)
    if (-not $declared.Success) {
        Assert-That $false "hosts: the BEGIN line is '$BeginPrefix sha256=<64 hex> names=<n>'" $lines[$begins[0]]
        return $null
    }
    $body = @()
    if ($ends[0] - $begins[0] -gt 1) {
        $body = @($lines[($begins[0] + 1)..($ends[0] - 1)])
    }
    $after = ''
    if ($ends[0] + 1 -lt $lines.Count) {
        $after = (@($lines[($ends[0] + 1)..($lines.Count - 1)]) -join '').Trim()
    }

    # The names, in order: the undo hint, then pairs of a `0.0.0.0` line and a `::` line.
    $names = [System.Collections.Generic.List[string]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $shape = ''
    if ($body.Count -eq 0 -or $body[0] -cne $UndoHint) {
        $shape = 'the first line of the block is not the undo hint'
    }
    elseif (($body.Count - 1) % 2 -ne 0) {
        $shape = 'the block does not hold one `::` line per `0.0.0.0` line'
    }
    else {
        for ($i = 1; $i -lt $body.Count; $i += 2) {
            $v4 = @($body[$i] -split ' ')
            $v6 = @($body[$i + 1] -split ' ')
            $group = @($v4 | Select-Object -Skip 1)
            if ($v4[0] -ne '0.0.0.0' -or $v6[0] -ne '::' -or (($v4 | Select-Object -Skip 1) -join ' ') -cne (($v6 | Select-Object -Skip 1) -join ' ')) {
                $shape = "line $($i + 1) of the block is not a ``0.0.0.0`` line followed by a ``::`` line with the same names"
                break
            }
            if ($group.Count -lt 1 -or $group.Count -gt $NamesPerLine -or $group -contains '') {
                $shape = "line $($i + 1) of the block holds $($group.Count) names (1 to $NamesPerLine expected)"
                break
            }
            foreach ($name in $group) {
                $names.Add($name)
                if (-not $seen.Add($name)) { $shape = "the name $name is written twice" }
            }
            if ($shape -ne '') { break }
        }
    }
    if ($shape -ne '') {
        Assert-That $false "hosts: the block is shaped as the design describes" $shape
        return $null
    }
    return [pscustomobject]@{
        Sha256      = $declared.Groups[1].Value
        Names       = [int]$declared.Groups[2].Value
        NameList    = $names.ToArray()
        # The rule of the design: SHA-256 over the lines between the markers, joined with "\n".
        BodySha256  = Get-Sha256Hex -Bytes ([System.Text.Encoding]::ASCII.GetBytes(($body -join "`n")))
        After       = $after
    }
}

# ----------------------------------------------------------------------------------------------
# Lookups

function Resolve-Through {
    # What a name resolves to through the operating system (`getaddrinfo`, as every program
    # asks): its addresses; $NoData alone when Winsock says "valid name, no data"; nothing
    # when the name is not known at all. Windows never hands out the null address: for a
    # name whose only answer is the null address, from the hosts file or from DNS, it
    # answers "no data" (the first complete run of this script showed it; the DNS client
    # itself holds the `0.0.0.0` and `::` records, and `Resolve-DnsName` prints them).
    param([Parameter(Mandatory)][string]$Name)
    try {
        return @([System.Net.Dns]::GetHostAddresses($Name) | ForEach-Object { $_.ToString() })
    }
    catch {
        if ((Get-SocketError $_) -eq $WsaNoData) { return @($NoData) }
        return @()
    }
}

function Get-SocketError {
    # The Winsock error number behind a failed lookup; 0 when the failure is something else.
    param([Parameter(Mandatory)]$ErrorRecord)
    $inner = $ErrorRecord.Exception
    while ($null -ne $inner.InnerException) { $inner = $inner.InnerException }
    if ($inner -is [System.Net.Sockets.SocketException]) { return [int]$inner.NativeErrorCode }
    return 0
}

function Get-LookupError {
    # The Winsock error number a lookup of the name fails with; 0 when it resolves.
    param([Parameter(Mandatory)][string]$Name)
    try {
        $null = [System.Net.Dns]::GetHostAddresses($Name)
        return 0
    }
    catch {
        return (Get-SocketError $_)
    }
}

function Test-Blocked {
    # Blocked for a program that asks the operating system: the null address and nothing
    # else, or "no data".
    param([string[]]$Addresses)
    return ($Addresses.Count -gt 0) -and (@($Addresses | Where-Object { $_ -notin $NullAddresses -and $_ -ne $NoData }).Count -eq 0)
}

function Test-Reachable {
    param([string[]]$Addresses)
    return @($Addresses | Where-Object { $_ -notin $NullAddresses -and $_ -ne $NoData }).Count -gt 0
}

function Wait-Lookup {
    # Looks a name up until $Wanted says the answer will do or $Seconds passed; the last answer.
    # Call it as @(Wait-Lookup ...): no address at all comes back as nothing.
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Wanted,
        [int]$Seconds = 10
    )
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    while ($true) {
        $found = @(Resolve-Through -Name $Name)
        if ((& $Wanted $found) -or $clock.Elapsed.TotalSeconds -ge $Seconds) {
            # One address per pipeline object: every caller collects them with @(...). A unary
            # comma here would hand such a caller an array inside an array, which reads as
            # one address that is never the null address and never "no answer".
            return $found
        }
        Start-Sleep -Milliseconds 250
    }
}

function Write-LookupDetail {
    # For the record, when a lookup through the operating system did not give what was
    # expected: what each way of asking for the name says, and how long it takes.
    param([Parameter(Mandatory)][string]$Name)
    foreach ($family in @('Unspecified', 'InterNetwork', 'InterNetworkV6')) {
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            $found = @([System.Net.Dns]::GetHostAddresses($Name, [System.Net.Sockets.AddressFamily]$family) | ForEach-Object { $_.ToString() })
            Write-Note "lookup detail: getaddrinfo($Name, $family) gave [$($found -join ' ')] in $([int]$clock.Elapsed.TotalMilliseconds) ms"
        }
        catch {
            $inner = $_.Exception
            while ($null -ne $inner.InnerException) { $inner = $inner.InnerException }
            $code = if ($inner -is [System.Net.Sockets.SocketException]) { "$($inner.SocketErrorCode) ($($inner.NativeErrorCode))" } else { $inner.GetType().Name }
            Write-Note "lookup detail: getaddrinfo($Name, $family) failed with $code in $([int]$clock.Elapsed.TotalMilliseconds) ms: $($inner.Message)"
        }
    }
    try {
        $ping = Start-Tool -FilePath (Join-Path $System32 'ping.exe') -Arguments @('-n', '1', '-w', '500', $Name)
        $said = @($ping.StdOut -split "`r?`n" | Where-Object { $_.Trim() -ne '' } | Select-Object -First 2)
        Write-Note "lookup detail: ping.exe $Name exited $($ping.ExitCode) and said: $($said -join ' | ')"
    }
    catch {
        Write-Note "lookup detail: ping.exe could not be run: $($_.Exception.Message)"
    }
    try {
        $records = @(Resolve-DnsName -Name $Name -ErrorAction Stop | ForEach-Object { "$(Get-Field $_ @('Type')) $(Get-Field $_ @('IPAddress')) ($(Get-Field $_ @('Section')))" })
        Write-Note "lookup detail: Resolve-DnsName $Name gave: $($records -join '; ')"
    }
    catch {
        Write-Note "lookup detail: Resolve-DnsName $Name failed: $($_.Exception.Message)"
    }
    try {
        $cache = Start-Tool -FilePath (Join-Path $System32 'ipconfig.exe') -Arguments @('/displaydns')
        $lines = @($cache.StdOut -split "`r?`n")
        $at = -1
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i].Trim() -eq $Name) { $at = $i; break }
        }
        if ($at -lt 0) {
            Write-Note "lookup detail: ipconfig /displaydns lists no entry for $Name ($($lines.Count) lines)"
        }
        else {
            $entry = @($lines[$at..([Math]::Min($at + 16, $lines.Count - 1))] | Where-Object { $_.Trim() -ne '' } | ForEach-Object { $_.Trim() })
            Write-Note "lookup detail: ipconfig /displaydns for $Name`: $($entry -join ' | ')"
        }
    }
    catch {
        Write-Note "lookup detail: ipconfig /displaydns could not be read: $($_.Exception.Message)"
    }
}

function Get-DirectAnswer {
    # The A records one resolver gives for a name, asked of it directly; Answered is false
    # when it did not answer.
    param([Parameter(Mandatory)][string]$Server, [Parameter(Mandatory)][string]$Name)
    try {
        $records = @(Resolve-DnsName -Name $Name -Server $Server -Type A -DnsOnly -NoHostsFile -ErrorAction Stop)
        $addresses = @($records | Where-Object { $_.Type -eq 'A' } | ForEach-Object { [string]$_.IPAddress })
        return [pscustomobject]@{ Answered = $true; Addresses = $addresses }
    }
    catch {
        return [pscustomobject]@{ Answered = $false; Addresses = @() }
    }
}

# ----------------------------------------------------------------------------------------------
# The DNS configuration, as a value that can be saved, compared and put back

function Get-StaticServers {
    # The value the quick read of the app relies on: `NameServer` under the interface's key.
    param([Parameter(Mandatory)][string]$Stack, [Parameter(Mandatory)][string]$Guid)
    $key = "HKLM:\SYSTEM\CurrentControlSet\Services\$Stack\Parameters\Interfaces\$Guid"
    $item = Get-ItemProperty -LiteralPath $key -Name 'NameServer' -ErrorAction SilentlyContinue
    if ($null -eq $item) { return '' }
    return [string](Get-Field $item @('NameServer'))
}

function Get-UpAdapters {
    # One adapter per pipeline object; call it as @(Get-UpAdapters).
    return @(Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | Sort-Object -Property ifIndex)
}

function Get-AdapterDns {
    # The DNS client's configuration of one adapter: one object per address family it is
    # bound to, none when the DNS client does not know the adapter (it is up, but TCP/IP is
    # not bound to it: a network card behind a virtual switch, the accelerated-networking
    # twin of a cloud adapter). Asking for such an adapter by its interface index is an
    # error, silenced here; and where the adapter carries the index of another adapter
    # (seen on a runner), the question answers with that one's objects, so only the
    # objects filed under the adapter's own name count.
    # One object per pipeline object; call it as @(Get-AdapterDns ...).
    param([Parameter(Mandatory)]$Adapter)
    $name = [string]$Adapter.Name
    return @(Get-DnsClientServerAddress -InterfaceIndex ([int]$Adapter.ifIndex) -ErrorAction SilentlyContinue | Where-Object { [string]$_.InterfaceAlias -eq $name })
}

function Get-DnsSnapshot {
    # Every adapter that is up and has a DNS client configuration: the adapters on which
    # there is something to set and to put back.
    $rows = @(foreach ($adapter in @(Get-UpAdapters)) {
        $index = [int]$adapter.ifIndex
        $guid = [string]$adapter.InterfaceGuid
        $dns = @(Get-AdapterDns -Adapter $adapter)
        if ($dns.Count -eq 0) { continue }
        [pscustomobject]@{
            Index    = $index
            Guid     = $guid
            Name     = [string]$adapter.Name
            V4       = @($dns | Where-Object { [int]$_.AddressFamily -eq 2 } | ForEach-Object { $_.ServerAddresses })
            V6       = @($dns | Where-Object { [int]$_.AddressFamily -eq 23 } | ForEach-Object { $_.ServerAddresses })
            StaticV4 = Get-StaticServers -Stack 'Tcpip' -Guid $guid
            StaticV6 = Get-StaticServers -Stack 'Tcpip6' -Guid $guid
        }
    })
    # The comma keeps the list whole for `$x = Get-DnsSnapshot` and `(Get-DnsSnapshot)`, also
    # when it holds one adapter or none. Never wrap a call in @(...): that would nest it.
    return , $rows
}

function Get-AdaptersWithoutDns {
    # "<index> (<name>)" of every adapter that is up and unknown to the DNS client. One per
    # pipeline object; call it as @(Get-AdaptersWithoutDns).
    return @(foreach ($adapter in @(Get-UpAdapters)) {
        if (@(Get-AdapterDns -Adapter $adapter).Count -eq 0) { "$([int]$adapter.ifIndex) ($([string]$adapter.Name))" }
    })
}

function ConvertTo-DnsText {
    param($Snapshot)
    return (@($Snapshot) | ForEach-Object { "$($_.Index) $($_.Guid) v4=[$(@($_.V4) -join ' ')] v6=[$(@($_.V6) -join ' ')] static4=[$($_.StaticV4)] static6=[$($_.StaticV6)]" }) -join '; '
}

function Test-SameAddresses {
    # Whether $Servers is exactly the address set $Wanted: nothing missing, nothing more.
    param([string[]]$Servers, [string[]]$Wanted)
    if ($null -eq $Servers -or $Servers.Count -ne $Wanted.Count) { return $false }
    try {
        $found = @($Servers | ForEach-Object { [System.Net.IPAddress]::Parse(($_ -split '%')[0]).ToString() } | Sort-Object)
        $want = @($Wanted | ForEach-Object { [System.Net.IPAddress]::Parse($_).ToString() } | Sort-Object)
        return ($found -join ' ') -eq ($want -join ' ')
    }
    catch {
        return $false
    }
}

function Split-Servers {
    # A `NameServer` registry value: addresses separated by commas or blanks. Windows pads
    # the value it writes with NUL characters after the text; what counts is the text in
    # front of the first one. One address per pipeline object; call it as
    # @(Split-Servers ...).
    param([string]$Value)
    $text = $Value.Split([char]0)[0]
    return @($text -split '[,\s]+' | Where-Object { $_ -ne '' })
}

function Get-RegNameServers {
    # What the quick read of the app sees: the output of
    # `reg.exe query <Interfaces key> /s /v NameServer`, run with the environment the app
    # gives it, as a table from interface GUID to the value's text.
    param([Parameter(Mandatory)][string]$Stack)
    $key = "HKLM\SYSTEM\CurrentControlSet\Services\$Stack\Parameters\Interfaces"
    $query = Start-Tool -FilePath (Join-Path $System32 'reg.exe') -Arguments @('query', $key, '/s', '/v', 'NameServer') -Environment $AppChildEnvironment -OnlyEnvironment
    $table = @{}
    $guid = ''
    foreach ($line in @($query.StdOut -split "`r?`n")) {
        if ($line.StartsWith('HKEY_')) {
            $guid = ($line.TrimEnd() -split '\\')[-1]
        }
        elseif ($guid -ne '' -and $line -match '^\s+NameServer\s+REG_SZ(\s+(?<data>.*))?$') {
            $table[$guid] = [string]$Matches['data']
        }
    }
    return $table
}

# ----------------------------------------------------------------------------------------------
# Plans

function ConvertTo-PlanArgument {
    # The `--plan-b64` value: base64url without padding of the plan's compact JSON, with all
    # six members and `resolver` written as null when there is none.
    param([bool]$Hosts, [bool]$Resolver, [string[]]$Extra = @())
    $extraJson = (@($Extra) | ForEach-Object { '"' + $_ + '"' }) -join ','
    $hostsJson = if ($Hosts) { 'true' } else { 'false' }
    $resolverJson = if ($Resolver) { '"' + $ResolverId + '"' } else { 'null' }
    $json = '{"v":1,"owner":"' + $Owner + '","hosts":' + $hostsJson + ',"extra":[' + $extraJson + '],"resolver":' + $resolverJson + ',"policies":false}'
    return [System.Convert]::ToBase64String([System.Text.Encoding]::ASCII.GetBytes($json)).TrimEnd('=').Replace('+', '-').Replace('/', '_')
}

function New-PaddedPlan {
    # A hosts-only plan whose argument is as long as it can be without passing $Target
    # characters: extra names under a reserved top-level domain that resolves nowhere.
    param([Parameter(Mandatory)][int]$Target)
    $names = [System.Collections.Generic.List[string]]::new()
    while ($names.Count -lt $MaxExtraNames) {
        $candidate = 'p{0:d3}-{1}{2}' -f $names.Count, ('x' * 13), $PaddingSuffix
        if ((ConvertTo-PlanArgument -Hosts $true -Resolver $false -Extra (@($names) + $candidate)).Length -gt $Target) { break }
        $names.Add($candidate)
    }
    if ($names.Count -eq 0) {
        throw "no padded plan fits in $Target characters"
    }
    # The last name grows one character at a time, up to the longest label a hostname may have.
    $last = $names.Count - 1
    $head = 'p{0:d3}-' -f $last
    for ($fill = 14; $head.Length + $fill -le 63; $fill++) {
        $longer = $head + ('x' * $fill) + $PaddingSuffix
        $trial = @($names)
        $trial[$last] = $longer
        if ((ConvertTo-PlanArgument -Hosts $true -Resolver $false -Extra $trial).Length -gt $Target) { break }
        $names[$last] = $longer
    }
    return [pscustomobject]@{
        Argument = ConvertTo-PlanArgument -Hosts $true -Resolver $false -Extra $names.ToArray()
        Extra    = $names.ToArray()
    }
}

# ----------------------------------------------------------------------------------------------
# The checks after an apply and after a removal

function Test-StateDirectoryAccess {
    # Under Program Files a standard user can read and cannot write (design 2.5).
    $standardUsers = @('S-1-5-32-545', 'S-1-5-11', 'S-1-1-0')   # Users, Authenticated Users, Everyone
    $writing = [int][System.Security.AccessControl.FileSystemRights]'WriteData, AppendData, WriteExtendedAttributes, DeleteSubdirectoriesAndFiles, WriteAttributes, Delete, ChangePermissions, TakeOwnership'
    $genericWriteOrAll = 0x50000000
    $genericRead = [int]0x80000000
    foreach ($path in @($StateDir, $StateFile)) {
        if (-not (Test-Path -LiteralPath $path)) {
            Assert-That $false "$path exists"
            continue
        }
        $item = Get-Item -LiteralPath $path -Force
        Assert-That (-not ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) "$path is not a link or a junction"
        $rules = @((Get-Acl -LiteralPath $path).GetAccessRules($true, $true, [System.Security.Principal.SecurityIdentifier]) |
                Where-Object { $_.AccessControlType -eq [System.Security.AccessControl.AccessControlType]::Allow -and $_.IdentityReference.Value -in $standardUsers })
        $canWrite = @($rules | Where-Object { (([int]$_.FileSystemRights) -band $writing) -ne 0 -or (([int]$_.FileSystemRights) -band $genericWriteOrAll) -ne 0 })
        $canRead = @($rules | Where-Object { (([int]$_.FileSystemRights) -band [int][System.Security.AccessControl.FileSystemRights]::ReadData) -ne 0 -or (([int]$_.FileSystemRights) -band $genericRead) -ne 0 })
        Assert-That ($canWrite.Count -eq 0) "$path cannot be written by a standard user" (@($canWrite | ForEach-Object { "$($_.IdentityReference.Value): $($_.FileSystemRights)" }))
        Assert-That ($canRead.Count -gt 0) "$path can be read by a standard user"
    }
}

function Test-Applied {
    # $Extra: names the plan carried, which must be in the block (step 5). $AfterApply runs
    # since the helper returned.
    param(
        [bool]$Hosts,
        [bool]$Resolver,
        [string[]]$Extra = @(),
        [Parameter(Mandatory)][System.Diagnostics.Stopwatch]$AfterApply,
        [hashtable]$Environment = @{}
    )
    $facts = [pscustomobject]@{ Listed = $null; FilterTest = $false }
    # The first lookup comes before anything else, so that what is timed is the time the
    # DNS Client needs to take the new file in.
    $firstLookup = $null
    $firstLookupMs = 0
    if ($Hosts) {
        $one = [System.Diagnostics.Stopwatch]::StartNew()
        $found = @(Wait-Lookup -Name $Canary -Wanted { param($a) Test-Blocked -Addresses $a } -Seconds 30)
        $firstLookupMs = [int]$one.Elapsed.TotalMilliseconds
        $firstLookup = [pscustomobject]@{
            Found      = $found
            AfterApply = $AfterApply.Elapsed.TotalSeconds
            AfterWrite = ([DateTime]::UtcNow - (Get-Item -LiteralPath $HostsPath).LastWriteTimeUtc).TotalSeconds
        }
    }

    $status = Get-Status -Environment $Environment
    Assert-That ($null -eq (Get-Field $status @('state_error'))) 'status: the state file is readable' (Get-Field $status @('state_error'))
    $failed = @(@(Get-Field $status @('state', 'report')) | Where-Object { $null -ne $_ -and -not (Get-Field $_ @('ok')) } | ForEach-Object { "$(Get-Field $_ @('tier')): $(Get-Field $_ @('reason'))" })
    Assert-That ($failed.Count -eq 0) 'state: no tier is reported as failed' $failed
    Test-StateDirectoryAccess

    if ($Hosts) {
        $current = [System.IO.File]::ReadAllBytes($HostsPath)
        $inFront = Test-StartsWith -Bytes $current -Prefix $SavedHosts
        Assert-That $inFront 'hosts: every byte of the saved file is still in front, unchanged'
        $block = Get-ShieldBlock -Bytes $current
        if ($null -ne $block) {
            if ($inFront) {
                $added = $Latin1.GetString($current, $SavedHosts.Length, $current.Length - $SavedHosts.Length)
                # A saved file that does not end with a line break gets one in front of the block.
                $endedLine = ($SavedHosts.Length -eq 0) -or ($SavedHosts[$SavedHosts.Length - 1] -eq 10)
                $startsOnOwnLine = if ($endedLine) { $added.StartsWith($BeginPrefix) } else { $added.TrimStart("`r", "`n").StartsWith($BeginPrefix) -and -not $added.StartsWith($BeginPrefix) }
                Assert-That $startsOnOwnLine 'hosts: the block starts right after the saved bytes, on a line of its own'
            }
            Assert-That ($block.After -eq '') 'hosts: nothing follows the END line'
            Write-Note "hosts: the block declares sha256=$($block.Sha256) names=$($block.Names)"
            Assert-That ($block.BodySha256 -eq $block.Sha256) 'hosts: the declared hash is the hash of the lines between the markers' $block.BodySha256
            Assert-That ($block.Names -eq $block.NameList.Count) "hosts: the declared count is the number of names ($($block.Names))" $block.NameList.Count
            Assert-That ($block.NameList.Count -ge 2 -and $block.NameList.Count -le $CapWindows) "hosts: between 2 and $CapWindows names" $block.NameList.Count
            Assert-That ($block.NameList[0] -ceq $Canary) 'hosts: the canary is the first name'
            $stamp = Get-Field $status @('state', 'hosts')
            Assert-That ((Get-Field $stamp @('sha256')) -eq $block.Sha256 -and (Get-Field $stamp @('names')) -eq $block.Names) 'state: records the hash and the count of the block'
            if ($Extra.Count -gt 0) {
                $written = [System.Collections.Generic.HashSet[string]]::new([string[]]$block.NameList)
                $missing = @($Extra | Where-Object { -not $written.Contains($_) })
                Assert-That ($missing.Count -eq 0) "hosts: all $($Extra.Count) names of the plan are in the block" "$($missing.Count) missing"
            }
            # A name of the list itself, taken from the file: the first one after the canary
            # that has two labels and is not one the plan brought.
            $listed = @($block.NameList | Select-Object -Skip 1 | Where-Object { ($_ -split '\.').Count -eq 2 -and $_ -notin $Extra } | Select-Object -First 1)
            if ($listed.Count -gt 0) { $facts.Listed = [string]$listed[0] }
            $script:Evidence['names'] = $block.Names
            $script:Evidence['block_sha256'] = $block.Sha256
        }
        $sddl = (Get-Acl -LiteralPath $HostsPath).Sddl
        Assert-That ($sddl -ceq $script:SavedHostsSddl) 'hosts: owner and access list are unchanged with the block in place' "$sddl (it was: $($script:SavedHostsSddl))"
        $text = [string](Get-Field $status @('hosts', 'text'))
        Assert-That ($text.Contains($BeginPrefix) -and $text.Contains($EndMarker)) 'status: the unprivileged read sees the block'
        Assert-That ($null -eq (Get-Field $status @('hosts', 'error'))) 'status: no hosts error' (Get-Field $status @('hosts', 'error'))
        Assert-That ((Get-Field $status @('state', 'owners', $Owner, 'hosts')) -eq $true) "state: the owner '$Owner' wants the hosts tier"
        # What the app's own check of the hosts tier reads: the helper asks for the canary
        # the way the app does and prints the answer.
        $canary = Get-Field $status @('canary')
        # Collected with @(...) around the whole `if`: an `if` used as a value hands a list of
        # one back as that one item, and a list of none as nothing.
        $canaryList = @(if ($null -ne $canary) { $canary })
        Assert-That ($canaryList.Count -gt 0 -and @($canaryList | Where-Object { $_ -notin $NullAddresses }).Count -eq 0) 'status: the canary reads as the null address, as the check in the app needs it' $canaryList

        $canaryBlocked = Test-Blocked -Addresses $firstLookup.Found
        Assert-That $canaryBlocked 'the canary is blocked for a program that asks the operating system (the null address, or "no data")' $firstLookup.Found
        if ($canaryBlocked) {
            Write-Note ('the first lookup of the canary gave [{3}] {0:n1} s after the helper returned and {1:n1} s after the hosts file was written; the lookup itself took {2} ms' -f $firstLookup.AfterApply, $firstLookup.AfterWrite, $firstLookupMs, ($firstLookup.Found -join ' '))
        }
        else {
            Write-Note ('the canary was not blocked in {0:n1} s of asking' -f ($firstLookupMs / 1000))
            Write-LookupDetail -Name $Canary
        }
        Assert-That ($firstLookup.AfterApply -lt $FirstLookupLimitSeconds) "the first lookup after the write comes within $FirstLookupLimitSeconds seconds" ('{0:n1} s' -f $firstLookup.AfterApply)
        $script:Evidence['first_lookup_s'] = '{0:n1}' -f $firstLookup.AfterApply
        if ($null -eq $facts.Listed) {
            Assert-That $false 'hosts: the block holds a name of the list after the canary'
        }
        else {
            $found = @(Wait-Lookup -Name $facts.Listed -Wanted { param($a) Test-Blocked -Addresses $a })
            Assert-That (Test-Blocked -Addresses $found) "a name of the list ($($facts.Listed)) is blocked for a program that asks the operating system" $found
            if (-not (Test-Blocked -Addresses $found)) {
                Write-LookupDetail -Name $facts.Listed
            }
        }
    }

    if ($Resolver) {
        Assert-That ((Get-Field $status @('state', 'owners', $Owner, 'resolver')) -eq $ResolverId) "state: the owner '$Owner' wants the resolver $ResolverId" (Get-Field $status @('state', 'owners', $Owner, 'resolver'))
        $now = Get-DnsSnapshot
        Assert-That (@($now).Count -gt 0) 'DNS: at least one adapter is up'
        $reg4 = Get-RegNameServers -Stack 'Tcpip'
        $reg6 = Get-RegNameServers -Stack 'Tcpip6'
        foreach ($row in @($now)) {
            Assert-That (Test-SameAddresses -Servers @(Split-Servers ([string]$reg4[$row.Guid])) -Wanted $ResolverV4) "reg.exe: the NameServer value of Tcpip for $($row.Guid) reads as the IPv4 pair" ([string]$reg4[$row.Guid])
            Assert-That (Test-SameAddresses -Servers @(Split-Servers ([string]$reg6[$row.Guid])) -Wanted $ResolverV6) "reg.exe: the NameServer value of Tcpip6 for $($row.Guid) reads as the IPv6 pair" ([string]$reg6[$row.Guid])
            Assert-That (Test-SameAddresses -Servers $row.V4 -Wanted $ResolverV4) "DNS: adapter $($row.Index) ($($row.Name)) uses the resolver's two IPv4 addresses" $row.V4
            Assert-That (Test-SameAddresses -Servers $row.V6 -Wanted $ResolverV6) "DNS: adapter $($row.Index) ($($row.Name)) uses the resolver's two IPv6 addresses" $row.V6
            Assert-That (Test-SameAddresses -Servers @(Split-Servers $row.StaticV4) -Wanted $ResolverV4) "registry: Tcpip NameServer of $($row.Guid) holds the IPv4 pair" $row.StaticV4
            Assert-That (Test-SameAddresses -Servers @(Split-Servers $row.StaticV6) -Wanted $ResolverV6) "registry: Tcpip6 NameServer of $($row.Guid) holds the IPv6 pair" $row.StaticV6
        }
        $active = @(@(Get-Field $status @('resolver', 'interfaces')) | Where-Object { $null -ne $_ -and (Get-Field $_ @('active')) })
        $seen = @($active | ForEach-Object { [string](Get-Field $_ @('id')) } | Sort-Object)
        $expected = @(@($now) | ForEach-Object { [string]$_.Index } | Sort-Object)
        Assert-That (($seen -join ' ') -ceq ($expected -join ' ')) 'status: lists exactly the adapters that are up and have a DNS client configuration' "$($seen -join ' ') (expected: $($expected -join ' '))"
        $wrong = @($active | Where-Object { -not (Test-SameAddresses -Servers @(Get-Field $_ @('servers')) -Wanted ($ResolverV4 + $ResolverV6)) } | ForEach-Object { "$(Get-Field $_ @('name')): $(@(Get-Field $_ @('servers')) -join ' ')" })
        Assert-That ($active.Count -gt 0 -and $wrong.Count -eq 0) 'status: every active interface reads as the resolver''s set' $wrong
        foreach ($server in $ResolverV4) {
            $answer = Get-DirectAnswer -Server $server -Name $Reachable
            Assert-That ($answer.Answered -and (Test-Reachable -Addresses $answer.Addresses)) "the resolver $server answers for $Reachable" $answer.Addresses
            if ($null -ne $facts.Listed) {
                $answer = Get-DirectAnswer -Server $server -Name $facts.Listed
                Assert-That ($answer.Answered -and (Test-Blocked -Addresses $answer.Addresses)) "the resolver $server refuses a listed name ($($facts.Listed)) with the null address" $answer.Addresses
            }
        }
        if ($null -eq $facts.Listed) {
            Write-Note 'without the hosts tier in this run, no listed name was asked of the resolver'
        }
        # Does the operating system really ask this resolver? Its operator's test name is
        # answered with the null address by the family resolver and by nobody else.
        $direct = Get-DirectAnswer -Server $ResolverV4[0] -Name $FilterTestName
        if ($direct.Answered -and (Test-Blocked -Addresses $direct.Addresses)) {
            $facts.FilterTest = $true
            $found = @(Wait-Lookup -Name $FilterTestName -Wanted { param($a) Test-Blocked -Addresses $a } -Seconds 15)
            Assert-That (Test-Blocked -Addresses $found) "the operating system resolves through the family resolver ($FilterTestName is refused)" $found
            if (-not (Test-Blocked -Addresses $found)) {
                Write-LookupDetail -Name $FilterTestName
            }
        }
        else {
            Write-Note "the resolver did not refuse its operator's test name $FilterTestName (answer: $($direct.Addresses -join ' ')), so whether the operating system asks it is shown by the configuration only"
        }
    }

    if ($script:BaselineResolves) {
        $found = @(Wait-Lookup -Name $Reachable -Wanted { param($a) Test-Reachable -Addresses $a } -Seconds 15)
        Assert-That (Test-Reachable -Addresses $found) "$Reachable still resolves with the tiers applied" $found
    }
    else {
        Write-Note "$Reachable did not resolve on this machine before the run, so it is not asked for"
    }
    return $facts
}

function Test-Removed {
    # True when everything is as it was saved.
    param($Facts, [hashtable]$Environment = @{})
    $before = $script:Failures
    $current = [System.IO.File]::ReadAllBytes($HostsPath)
    Assert-That (Test-SameBytes -Left $current -Right $SavedHosts) "hosts: byte for byte the saved file ($($SavedHosts.Length) bytes)" "$($current.Length) bytes"
    $sddl = (Get-Acl -LiteralPath $HostsPath).Sddl
    Assert-That ($sddl -ceq $script:SavedHostsSddl) 'hosts: owner and access list are what they were' "$sddl (it was: $($script:SavedHostsSddl))"
    Assert-That (-not (Test-Path -LiteralPath $StateDir)) 'the state directory is gone'
    $status = Get-Status -Environment $Environment
    Assert-That ($null -eq (Get-Field $status @('state')) -and $null -eq (Get-Field $status @('state_error'))) 'status: no state file'
    $text = [string](Get-Field $status @('hosts', 'text'))
    Assert-That (-not ($text.Contains('>>> ubiqx-shield') -or $text.Contains('<<< ubiqx-shield'))) 'status: the unprivileged read sees no block'
    $now = ConvertTo-DnsText (Get-DnsSnapshot)
    Assert-That ($now -ceq $script:SavedDnsText) 'DNS: every adapter has the servers it had before the run, and the registry holds what it held' $now

    Assert-That ($null -eq (Get-Field $status @('canary'))) 'status: the canary is not asked for without the block' (Get-Field $status @('canary'))
    $found = @(Wait-Lookup -Name $Canary -Wanted { param($a) $a.Count -eq 0 })
    Assert-That ($found.Count -eq 0) 'the canary no longer resolves' $found
    # The difference the helper relies on: with the block the canary is "no data", without
    # it the name is one nobody knows.
    $code = Get-LookupError -Name $Canary
    Assert-That ($code -eq $WsaHostNotFound) "without the block the canary fails as an unknown name (Winsock error $WsaHostNotFound), not as `"no data`" ($WsaNoData)" $code
    if ($null -ne $Facts -and $null -ne $Facts.Listed) {
        $found = @(Wait-Lookup -Name $Facts.Listed -Wanted { param($a) -not (Test-Blocked -Addresses $a) })
        Assert-That (-not (Test-Blocked -Addresses $found)) "the listed name ($($Facts.Listed)) is no longer pinned to the null address" $found
    }
    if ($null -ne $Facts -and $Facts.FilterTest) {
        $found = @(Wait-Lookup -Name $FilterTestName -Wanted { param($a) -not (Test-Blocked -Addresses $a) } -Seconds 15)
        Assert-That (-not (Test-Blocked -Addresses $found)) 'the operating system no longer resolves through the family resolver' $found
    }
    if ($script:BaselineResolves) {
        $found = @(Wait-Lookup -Name $Reachable -Wanted { param($a) Test-Reachable -Addresses $a } -Seconds 15)
        Assert-That (Test-Reachable -Addresses $found) "$Reachable still resolves after the removal" $found
    }
    return ($script:Failures -eq $before)
}

function Write-HelperReport {
    # What the helper recorded and what it sees, after an apply that did not exit 0.
    param([hashtable]$Environment = @{})
    $status = Get-Status -Environment $Environment
    foreach ($row in @(Get-Field $status @('state', 'report'))) {
        if ($null -ne $row) {
            Write-Note "state report: tier=$(Get-Field $row @('tier')) ok=$(Get-Field $row @('ok')) reason=$(Get-Field $row @('reason'))"
        }
    }
    Write-Note "status: hosts.error=$(Get-Field $status @('hosts', 'error')) resolver.error=$(Get-Field $status @('resolver', 'error')) state_error=$(Get-Field $status @('state_error'))"
    foreach ($interface in @(Get-Field $status @('resolver', 'interfaces'))) {
        if ($null -ne $interface) {
            Write-Note "status: interface $(Get-Field $interface @('id')) ($(Get-Field $interface @('name'))) servers=[$(@(Get-Field $interface @('servers')) -join ' ')]"
        }
    }
}

function Invoke-Removal {
    # The helper's own way back, then the proof that it led all the way.
    param($Facts, [hashtable]$Environment = @{})
    Write-Step 'remove --owner all --scope all'
    $remove = Invoke-Helper -Arguments @('remove', '--owner', 'all', '--scope', 'all') -Environment $Environment
    Assert-That ($remove.ExitCode -eq 0) 'remove exits 0' "$($remove.ExitCode) $($remove.StdErr.Trim())"
    Write-Step 'checking the restore'
    $script:Restored = Test-Removed -Facts $Facts -Environment $Environment
}

function Invoke-Cycle {
    # Steps 2 to 4: apply, check, remove, check the restore.
    param([Parameter(Mandatory)][string]$Label, [hashtable]$Environment = @{})
    $script:Restored = $false
    $script:Changed = $true
    Write-Step "apply ($Label)"
    $apply = Invoke-Helper -Arguments @('apply', '--plan-b64', $script:Plan, '--unverified', $TierArgument) -Environment $Environment
    $afterApply = [System.Diagnostics.Stopwatch]::StartNew()
    $facts = $null
    # 2 invalid plan, 3 partial (the state file holds a report per tier), 4 failed, 5 tier gated.
    Assert-That ($apply.ExitCode -eq 0) 'apply exits 0' "$($apply.ExitCode) $($apply.StdErr.Trim())"
    if ($apply.ExitCode -eq 0) {
        Write-Step 'checking what was applied'
        $facts = Test-Applied -Hosts $WithHosts -Resolver $WithResolver -AfterApply $afterApply -Environment $Environment
    }
    else {
        Write-HelperReport -Environment $Environment
    }
    Invoke-Removal -Facts $facts -Environment $Environment
}

function Invoke-ElevationMeasurement {
    # Step 5. The runner is an administrator with UAC off, so `Start-Process -Verb RunAs`
    # starts without a prompt, but the argument still goes through ShellExecuteEx. The command
    # runs in the four-variable environment the app builds for it, not in this shell's: this
    # shell's PSModulePath names PowerShell 7's modules, which Windows PowerShell cannot load.
    $longest = 0
    $control = $null
    foreach ($target in @(0) + $ElevationLengths) {
        if ($target -eq 0) {
            $plan = [pscustomobject]@{ Argument = ConvertTo-PlanArgument -Hosts $true -Resolver $false; Extra = @() }
            $label = "the plan without extra names ($($plan.Argument.Length) characters)"
        }
        else {
            $plan = New-PaddedPlan -Target $target
            $label = "a plan of $($plan.Argument.Length) characters ($($plan.Extra.Count) names; target $target)"
        }
        Write-Step "elevation command with $label"
        $printed = Invoke-Helper -Arguments @('elevate-argv', '--plan-b64', $plan.Argument, '--unverified', 'hosts')
        $argv = @()
        if ($printed.ExitCode -eq 0) {
            $argv = @($printed.StdOut | ConvertFrom-Json)
        }
        if ($argv.Count -lt 2) {
            Assert-That $false "elevate-argv prints the command for $label" "exit $($printed.ExitCode) $($printed.StdErr.Trim())"
            # Without the shortest command there is nothing to measure against.
            if ($target -eq 0) { break }
            continue
        }
        $tool = [string]$argv[0]
        Assert-That ($tool.StartsWith($System32, [System.StringComparison]::OrdinalIgnoreCase)) 'the elevation tool is a program of the system directory' $tool

        if ($target -eq 0) {
            # What the app reads is the exit code of this command, so the helper's own has to
            # come back through `Start-Process -Wait -PassThru`: an exit code that got lost
            # would read as "applied". Asked with the same command and a plan argument the
            # helper refuses before it touches anything.
            $refusedArguments = @($argv | Select-Object -Skip 1 | ForEach-Object { ([string]$_).Replace($plan.Argument, $RefusedPlan) })
            $swapped = @($refusedArguments | Where-Object { $_.Contains($RefusedPlan) }).Count -eq 1 -and @($refusedArguments | Where-Object { $_.Contains($plan.Argument) }).Count -eq 0
            Assert-That $swapped 'the printed elevation command carries the plan argument in one place'
            if ($swapped) {
                $refused = Start-Tool -FilePath $tool -Arguments $refusedArguments -Environment $AppChildEnvironment -OnlyEnvironment
                Assert-That ($refused.ExitCode -eq $ExitInvalidPlan) "the elevation command passes the helper's exit code on (a plan the helper refuses: exit $ExitInvalidPlan)" $refused.ExitCode
                $untouched = (Test-SameBytes -Left ([System.IO.File]::ReadAllBytes($HostsPath)) -Right $SavedHosts) -and -not (Test-Path -LiteralPath $StateDir)
                Assert-That $untouched 'a refused plan changes nothing: the hosts file is the saved one and there is no state directory'
            }
        }

        $script:Restored = $false
        $script:Changed = $true
        $run = Start-Tool -FilePath $tool -Arguments @($argv | Select-Object -Skip 1 | ForEach-Object { [string]$_ }) -Environment $AppChildEnvironment -OnlyEnvironment
        $afterApply = [System.Diagnostics.Stopwatch]::StartNew()
        $intact = $false
        $facts = $null
        if ($run.ExitCode -eq 0) {
            $before = $script:Failures
            $facts = Test-Applied -Hosts $true -Resolver $false -Extra $plan.Extra -AfterApply $afterApply
            $intact = ($script:Failures -eq $before)
        }
        elseif ($run.ExitCode -eq $ExitGated) {
            # Decoded and validated, then stopped at the gate: the argument arrived whole.
            $intact = $true
            Write-Note 'the helper read the plan and stopped at the gate (exit 5): the argument arrived, nothing was applied'
        }
        else {
            Write-Note "the elevated run exited $($run.ExitCode) (1223: it did not start; $ExitInvalidPlan`: the helper could not read what arrived)"
        }
        if ($target -eq 0) {
            $control = $intact
            $hint = if ($HelperForm -eq 'bare') { ' The printed command re-runs the binary as `<exe> --shield-helper apply ...`; a helper binary that takes verbs only without that flag cannot be elevated this way.' } else { '' }
            Assert-That $intact "the elevation command works at all with the shortest plan.$hint" $run.ExitCode
        }
        elseif ($intact) {
            $longest = [Math]::Max($longest, $plan.Argument.Length)
            Write-Host "  ok    $label reached the helper intact"
        }
        else {
            Write-Note "$label did not reach the helper intact"
        }
        Invoke-Removal -Facts $facts
        if (-not $control) { break }
    }
    if ($control) {
        Write-Step "ARG_BUDGET evidence: the longest --plan-b64 value that reached the helper intact has $longest characters"
        $script:Evidence['longest_plan'] = $longest
        Assert-That ($longest -ge ($ArgumentBudget - 3)) "a plan of the budget the app uses today ($ArgumentBudget characters) reaches the helper" $longest
    }
}

function Restore-Saved {
    # Puts the saved state back without the helper. Only reached when the helper's own
    # removal was not verified. Every step is tried, whatever the one before it did.
    Write-Step 'restoring the saved state without the helper'
    try {
        if (-not (Test-SameBytes -Left ([System.IO.File]::ReadAllBytes($HostsPath)) -Right $SavedHosts)) {
            [System.IO.File]::WriteAllBytes($HostsPath, $SavedHosts)
            Write-Step "  $HostsPath`: the saved bytes are back"
        }
    }
    catch {
        [Console]::Error.WriteLine("system-smoke: COULD NOT RESTORE $HostsPath`: $($_.Exception.Message)")
    }
    try {
        if ((Get-Acl -LiteralPath $HostsPath).Sddl -ne $script:SavedHostsSddl) {
            # Not put back from here: setting an owner needs rights this script does not take.
            [Console]::Error.WriteLine("system-smoke: the owner or the access list of $HostsPath is not what it was. It was: $($script:SavedHostsSddl)")
        }
    }
    catch {
        [Console]::Error.WriteLine("system-smoke: the access list of $HostsPath could not be read: $($_.Exception.Message)")
    }
    foreach ($row in @($script:SavedDns)) {
        try {
            # Automatic first, for both address families: setting the saved IPv4 servers alone
            # would leave the resolver's IPv6 pair on the adapter. Then what the adapter had.
            Set-DnsClientServerAddress -InterfaceIndex $row.Index -ResetServerAddresses
            $static = @(Split-Servers $row.StaticV4) + @(Split-Servers $row.StaticV6)
            if ($static.Count -gt 0) {
                Set-DnsClientServerAddress -InterfaceIndex $row.Index -ServerAddresses $static
            }
        }
        catch {
            [Console]::Error.WriteLine("system-smoke: COULD NOT RESTORE the DNS of adapter $($row.Index): $($_.Exception.Message)")
        }
    }
    try {
        $now = ConvertTo-DnsText (Get-DnsSnapshot)
        if ($now -ceq $script:SavedDnsText) {
            Write-Step '  DNS: every adapter has the servers it had before the run'
        }
        else {
            [Console]::Error.WriteLine("system-smoke: COULD NOT RESTORE DNS. It was: $($script:SavedDnsText). It is: $now")
        }
    }
    catch {
        [Console]::Error.WriteLine("system-smoke: the DNS configuration could not be read back: $($_.Exception.Message)")
    }
    try {
        # The state directory did not exist before the run (checked above).
        if (Test-Path -LiteralPath $StateDir) {
            Remove-Item -LiteralPath $StateDir -Recurse -Force
            Write-Step "  $StateDir`: removed"
        }
    }
    catch {
        [Console]::Error.WriteLine("system-smoke: COULD NOT REMOVE $StateDir`: $($_.Exception.Message)")
    }
    try {
        $null = Start-Tool -FilePath (Join-Path $System32 'ipconfig.exe') -Arguments @('/flushdns')
    }
    catch {
        [Console]::Error.WriteLine("system-smoke: the name cache was not flushed: $($_.Exception.Message)")
    }
}

# ----------------------------------------------------------------------------------------------
# 1. Save.

Write-Step "os=windows tiers=$TierArgument helper=$Helper ($HelperForm form)"
$script:SavedHostsSddl = (Get-Acl -LiteralPath $HostsPath).Sddl
$script:SavedDns = Get-DnsSnapshot
$script:SavedDnsText = ConvertTo-DnsText $script:SavedDns
$script:BaselineResolves = Test-Reachable -Addresses @(Resolve-Through -Name $Reachable)
$script:Plan = ConvertTo-PlanArgument -Hosts $WithHosts -Resolver $WithResolver
Write-Step "saved: $HostsPath ($($SavedHosts.Length) bytes), its access list and the DNS of $(@($script:SavedDns).Count) adapter(s): $($script:SavedDnsText)"
foreach ($adapter in @(Get-AdaptersWithoutDns)) {
    Write-Note "adapter $adapter is up and the DNS client has no configuration for it: there is nothing on it to set, and it is left out"
}

$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("ubiqx-shield-smoke-" + [Guid]::NewGuid().ToString('N'))
$exitCode = 1
try {
    # 2 to 4.
    Invoke-Cycle -Label 'the environment of this shell'

    # 5.
    if ($SkipElevation) {
        Write-Note 'step 5 (the elevation command) was left out: -SkipElevation'
    }
    elseif (-not $WithHosts) {
        Write-Note 'step 5 (the elevation command) needs the hosts tier and was left out'
    }
    else {
        Invoke-ElevationMeasurement
    }

    # 6.
    if ($SkipHostileEnvironment) {
        Write-Note 'step 6 (hostile environment) was left out: -SkipHostileEnvironment'
    }
    else {
        $null = New-Item -ItemType Directory -Path $scratch
        $hostile = @{ windir = $scratch; SystemRoot = $scratch; ProgramData = $scratch; PSModulePath = $scratch }
        # Not a check, a fact for the record: whether a program that is started with these
        # variables can still resolve a name (the helper looks one up before and after it
        # changes DNS). `ping.exe` asks the operating system the same way the helper does.
        # `localhost` too: it is the name the system layer's own test of this looks up.
        $asked = @('localhost')
        if ($script:BaselineResolves) { $asked += $Reachable }
        foreach ($name in $asked) {
            $ping = Start-Tool -FilePath (Join-Path $System32 'ping.exe') -Arguments @('-n', '1', '-w', '1000', $name) -Environment $hostile
            $said = @($ping.StdOut -split "`r?`n" | Where-Object { $_.Trim() -ne '' } | Select-Object -First 1)
            Write-Note "name lookups in a program started with these variables: ping.exe $name exited $($ping.ExitCode) and said: $($said -join '')"
        }
        Invoke-Cycle -Label 'windir, SystemRoot, ProgramData and PSModulePath pointing at a scratch directory' -Environment $hostile
        $left = @(Get-ChildItem -LiteralPath $scratch -Force -Recurse)
        Assert-That ($left.Count -eq 0) 'nothing was created in the scratch directory the variables pointed at' (@($left | ForEach-Object { $_.FullName }))
    }

    if ($script:Failures -eq 0 -and $script:Restored) {
        $exitCode = 0
    }
}
catch {
    $script:Failures += 1
    [Console]::Error.WriteLine("system-smoke: the run stopped: $($_.Exception.Message)")
    [Console]::Error.WriteLine($_.ScriptStackTrace)
}
finally {
    if ($script:Changed -and -not $script:Restored) {
        $exitCode = 1
        Restore-Saved
    }
    if (Test-Path -LiteralPath $scratch) {
        Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($exitCode -ne 0) {
    [Console]::Error.WriteLine("system-smoke: RED ($($script:Failures) check(s) failed)")
    exit 1
}
$helperSha = (Get-FileHash -LiteralPath $Helper -Algorithm SHA256).Hash.ToLowerInvariant()
$commit = 'unknown'
try {
    $git = Start-Tool -FilePath 'git' -Arguments @('-C', $RepoRoot, 'rev-parse', '--short=7', 'HEAD')
    if ($git.ExitCode -eq 0) { $commit = $git.StdOut.Trim() }
}
catch {
    $commit = 'unknown'
}
$evidence = (@($script:Evidence.Keys) | ForEach-Object { "$_=$($script:Evidence[$_])" }) -join ' '
Write-Step "GREEN os=windows tiers=$TierArgument commit=$commit helper_sha256=$helperSha $evidence"
exit 0
