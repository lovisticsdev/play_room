# Scripts

Convenience wrappers for running Play Room from PowerShell or Bash. Each script resolves the repository root from its own location, so it can be launched from any working directory.

## Server

```powershell
.\scripts\run-server.ps1
.\scripts\run-server.ps1 -Port 9000
```

```bash
bash scripts/run-server.sh
bash scripts/run-server.sh --port 9000
```

## Terminal Client

```powershell
.\scripts\run-client.ps1 -Name alice
.\scripts\run-client.ps1 -Name alice -Port 9000
```

```bash
bash scripts/run-client.sh alice
bash scripts/run-client.sh alice --port 9000
```

## Web Client

```powershell
.\scripts\run-web.ps1
.\scripts\run-web.ps1 -Install -Port 5174
```

```bash
bash scripts/run-web.sh
bash scripts/run-web.sh --install --port 5174
```
