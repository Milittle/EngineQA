$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RuntimeEnvFile = Join-Path $RootDir "runtime.env"

function Import-KeyValueFile {
    param (
        [Parameter(Mandatory=$true)]
        [string]$Path
    )

    if (-not (Test-Path $Path)) {
        return
    }

    Get-Content $Path | ForEach-Object {
        $line = $_.Trim()
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith("#")) {
            return
        }

        $parts = $line.Split("=", 2)
        if ($parts.Count -ne 2) {
            return
        }

        $key = $parts[0].Trim()
        $value = $parts[1].Trim()
        [System.Environment]::SetEnvironmentVariable($key, $value, "Process")
    }
}

function Convert-ToNginxPath {
    param (
        [Parameter(Mandatory=$true)]
        [string]$Path
    )

    return ($Path -replace "\\", "/")
}

Import-KeyValueFile -Path $RuntimeEnvFile

$EnvFile = if ($env:APP_ENV_FILE) { $env:APP_ENV_FILE } else { Join-Path $RootDir ".env" }
if (-not (Test-Path $EnvFile)) {
    Write-Host "missing environment file: $EnvFile"
    Write-Host "initialize with: Copy-Item config/.env.example .env"
    exit 1
}

Import-KeyValueFile -Path $EnvFile

$BackendBinRel = if ($env:ENGINEQA_BACKEND_BIN) { $env:ENGINEQA_BACKEND_BIN } else { "bin\engineqa-backend.exe" }
$BackendBin = Join-Path $RootDir $BackendBinRel
if (-not (Test-Path $BackendBin)) {
    Write-Host "backend executable not found: $BackendBin"
    exit 1
}

$FrontendDirRel = if ($env:ENGINEQA_FRONTEND_DIR) { $env:ENGINEQA_FRONTEND_DIR } else { "frontend" }
$FrontendDir = Join-Path $RootDir $FrontendDirRel
if (-not (Test-Path $FrontendDir)) {
    Write-Host "frontend directory not found: $FrontendDir"
    exit 1
}

$PidFileRel = if ($env:ENGINEQA_PID_FILE) { $env:ENGINEQA_PID_FILE } else { "run\backend.pid" }
$LogFileRel = if ($env:ENGINEQA_LOG_FILE) { $env:ENGINEQA_LOG_FILE } else { "logs\backend.log" }
$PidFile = Join-Path $RootDir $PidFileRel
$LogFile = Join-Path $RootDir $LogFileRel
$NginxPidRel = if ($env:ENGINEQA_NGINX_PID_FILE) { $env:ENGINEQA_NGINX_PID_FILE } else { "run\nginx.pid" }
$NginxConfRel = if ($env:ENGINEQA_NGINX_CONF_FILE) { $env:ENGINEQA_NGINX_CONF_FILE } else { "run\nginx.conf" }
$NginxAccessLogRel = if ($env:ENGINEQA_NGINX_ACCESS_LOG) { $env:ENGINEQA_NGINX_ACCESS_LOG } else { "logs\nginx.access.log" }
$NginxErrorLogRel = if ($env:ENGINEQA_NGINX_ERROR_LOG) { $env:ENGINEQA_NGINX_ERROR_LOG } else { "logs\nginx.error.log" }
$NginxPidFile = Join-Path $RootDir $NginxPidRel
$NginxConfFile = Join-Path $RootDir $NginxConfRel
$NginxAccessLog = Join-Path $RootDir $NginxAccessLogRel
$NginxErrorLog = Join-Path $RootDir $NginxErrorLogRel

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $PidFile) | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogFile) | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $NginxPidFile) | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $NginxConfFile) | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $NginxAccessLog) | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $NginxErrorLog) | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $RootDir "data") | Out-Null

if (Test-Path $PidFile) {
    $existingPid = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($existingPid) {
        $existingProcess = Get-Process -Id $existingPid -ErrorAction SilentlyContinue
        if ($existingProcess) {
            Write-Host "backend is already running (pid=$existingPid)"
            exit 1
        }
    }
    Remove-Item -Force $PidFile
}

if (Test-Path $NginxPidFile) {
    $existingNginxPid = (Get-Content $NginxPidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($existingNginxPid) {
        $existingNginxProcess = Get-Process -Id $existingNginxPid -ErrorAction SilentlyContinue
        if ($existingNginxProcess) {
            Write-Host "frontend nginx is already running (pid=$existingNginxPid)"
            exit 1
        }
    }
    Remove-Item -Force $NginxPidFile
}

if (-not $env:APP_HOST) {
    $env:APP_HOST = "0.0.0.0"
}

$runtimeKind = if ($env:ENGINEQA_RUNTIME_KIND) { $env:ENGINEQA_RUNTIME_KIND } else { "rust" }
switch ($runtimeKind) {
    "rust" {
        if (-not $env:LANCEDB_URI) {
            $env:LANCEDB_URI = (Join-Path $RootDir "data\.lancedb")
        }
    }
    "python" {
        if (-not $env:QDRANT_LOCAL_PATH) {
            $env:QDRANT_LOCAL_PATH = (Join-Path $RootDir "data\.qdrant-local")
        }
    }
    default {
        Write-Host "unknown runtime kind: $runtimeKind"
        exit 1
    }
}

$backendProcess = Start-Process `
    -FilePath $BackendBin `
    -WorkingDirectory $RootDir `
    -RedirectStandardOutput $LogFile `
    -RedirectStandardError $LogFile `
    -PassThru

$nginxCmd = Get-Command nginx -ErrorAction SilentlyContinue
if (-not $nginxCmd) {
    Stop-Process -Id $backendProcess.Id -Force -ErrorAction SilentlyContinue
    Remove-Item -Force $PidFile -ErrorAction SilentlyContinue
    Write-Host "nginx command not found in PATH"
    exit 1
}

$backendPort = if ($env:APP_PORT) { $env:APP_PORT } else { "8080" }
$frontendPort = if ($env:FRONTEND_PORT) { $env:FRONTEND_PORT } else { "5173" }
$nginxPidN = Convert-ToNginxPath -Path $NginxPidFile
$nginxAccessLogN = Convert-ToNginxPath -Path $NginxAccessLog
$nginxErrorLogN = Convert-ToNginxPath -Path $NginxErrorLog
$frontendDirN = Convert-ToNginxPath -Path $FrontendDir

$nginxTemplate = @'
pid "__PID__";

events {
  worker_connections 1024;
}

http {
  types {
    text/html html htm shtml;
    text/css css;
    text/xml xml;
    application/javascript js mjs;
    application/json json;
    image/svg+xml svg svgz;
    image/png png;
    image/jpeg jpeg jpg;
    image/x-icon ico;
    font/woff woff;
    font/woff2 woff2;
  }
  default_type application/octet-stream;

  access_log "__ACCESS_LOG__";
  error_log "__ERROR_LOG__" warn;

  server {
    listen __FRONTEND_PORT__;
    server_name 127.0.0.1 localhost;

    root "__FRONTEND_DIR__";
    index index.html;

    location /api/ {
      proxy_pass http://127.0.0.1:__BACKEND_PORT__;
      proxy_http_version 1.1;
      proxy_set_header Host $host;
      proxy_set_header X-Real-IP $remote_addr;
      proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
      proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /health {
      proxy_pass http://127.0.0.1:__BACKEND_PORT__/health;
      proxy_http_version 1.1;
      proxy_set_header Host $host;
    }

    location /assets/ {
      if_modified_since off;
      etag off;
      add_header Cache-Control "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0" always;
      try_files $uri =404;
    }

    location / {
      try_files $uri $uri/ /index.html;
    }
  }
}
'@

$nginxConfig = $nginxTemplate.
    Replace("__PID__", $nginxPidN).
    Replace("__ACCESS_LOG__", $nginxAccessLogN).
    Replace("__ERROR_LOG__", $nginxErrorLogN).
    Replace("__FRONTEND_DIR__", $frontendDirN).
    Replace("__FRONTEND_PORT__", "$frontendPort").
    Replace("__BACKEND_PORT__", "$backendPort")

Set-Content -Path $NginxConfFile -Value $nginxConfig -Encoding UTF8
Set-Content -Path $PidFile -Value $backendProcess.Id

try {
    Start-Process -FilePath $nginxCmd.Source -ArgumentList "-g", "error_log $NginxErrorLog;", "-p", $RootDir, "-c", $NginxConfFile -WindowStyle Hidden | Out-Null
} catch {
    Stop-Process -Id $backendProcess.Id -Force -ErrorAction SilentlyContinue
    Remove-Item -Force $PidFile -ErrorAction SilentlyContinue
    Write-Host "failed to start nginx with config: $NginxConfFile"
    exit 1
}

Write-Host "services started"
Write-Host "pid: $($backendProcess.Id)"
Write-Host "backend health: http://127.0.0.1:$backendPort/health"
Write-Host "frontend url: http://127.0.0.1:$frontendPort"
Write-Host "backend logs: $LogFile"
Write-Host "nginx logs: $NginxAccessLog, $NginxErrorLog"
