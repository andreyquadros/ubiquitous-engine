# Fixtures of the system layer's parsers

Samples of what the system tools print, in the shapes the tools document. They were
written by hand for these tests: this build sandbox runs neither macOS nor Windows, so none
was recorded from a live machine. The smoke job (`nofap-system-smoke`) is what runs the
real tools.

| File | Tool |
|---|---|
| `networksetup-services*.txt` | `networksetup -listallnetworkservices` (English and PT-BR header) |
| `networksetup-dns-*.txt` | `networksetup -getdnsservers <service>` |
| `defaults-firefox.txt` | `defaults read /Library/Preferences/org.mozilla.firefox` |
| `powershell-adapters-*.json` | the full-read script of `windows.rs` (`ConvertTo-Json`) |
| `reg-policy-chrome.txt` | `reg.exe query HKLM\SOFTWARE\Policies\Google\Chrome` |
| `reg-nameserver.txt` | `reg.exe query …\Tcpip\Parameters\Interfaces /s /v NameServer` (PT-BR summary line) |
| `resolv-conf-*.txt` | `/etc/resolv.conf` with and without the systemd-resolved stub |
| `chrome-foreign-policy.json` | a policy file somebody else put in a `managed` directory |
