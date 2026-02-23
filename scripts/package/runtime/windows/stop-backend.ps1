$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RuntimeEnvFile = Join-Path $RootDir "runtime.env"

if (Test-Path $RuntimeEnvFile) {
    Get-Content $RuntimeEnvFile | ForEach-Object {
        $line = $_.Trim()
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith("#")) {
            return
        }

        $parts = $line.Split("=", 2)
        if ($parts.Count -ne 2) {
            return
        }

        [System.Environment]::SetEnvironmentVariable($parts[0].Trim(), $parts[1].Trim(), "Process")
    }
}

$PidFileRel = if ($env:ENGINEQA_PID_FILE) { $env:ENGINEQA_PID_FILE } else { "run\backend.pid" }
$PidFile = Join-Path $RootDir $PidFileRel
$NginxPidRel = if ($env:ENGINEQA_NGINX_PID_FILE) { $env:ENGINEQA_NGINX_PID_FILE } else { "run\nginx.pid" }
$NginxPidFile = Join-Path $RootDir $NginxPidRel

if (Test-Path $NginxPidFile) {
    $nginxPidValue = (Get-Content $NginxPidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if (-not $nginxPidValue) {
        Write-Host "invalid nginx pid file: $NginxPidFile"
        Remove-Item -Force $NginxPidFile
        exit 1
    }

    $nginxProcess = Get-Process -Id $nginxPidValue -ErrorAction SilentlyContinue
    if ($nginxProcess) {
        Stop-Process -Id $nginxPidValue -Force
        Write-Host "stopped frontend nginx pid=$nginxPidValue"
    } else {
        Write-Host "frontend nginx already stopped pid=$nginxPidValue"
    }
    Remove-Item -Force $NginxPidFile
} else {
    Write-Host "frontend nginx is not running (pid file missing: $NginxPidFile)"
}

if (-not (Test-Path $PidFile)) {
    Write-Host "backend is not running (pid file missing: $PidFile)"
    exit 0
}

$pidValue = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
if (-not $pidValue) {
    Write-Host "invalid backend pid file: $PidFile"
    Remove-Item -Force $PidFile
    exit 1
}

$process = Get-Process -Id $pidValue -ErrorAction SilentlyContinue
if ($process) {
    Stop-Process -Id $pidValue -Force
    Write-Host "stopped backend pid=$pidValue"
} else {
    Write-Host "backend already stopped pid=$pidValue"
}

Remove-Item -Force $PidFile
