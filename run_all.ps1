param(
    [string]$RedisContainerName = "redis-mmorpg",
    [int]$RedisPort = 6379,
    [int]$GateKeeperPort = 3000,
    [int]$OrchestratorPort = 9000,
    [int]$FirstDedicatedServerPort = 7001,
    [int]$HotServersMin = 1,
    [int]$Launchers = 3,
    [int]$BrokerPort = 5000
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)

    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Test-CommandExists {
    param([string]$CommandName)

    $command = Get-Command $CommandName -ErrorAction SilentlyContinue
    return $null -ne $command
}

function Start-Redis {
    Write-Step "Starting Redis"

    if (-not (Test-CommandExists "docker")) {
        throw "Docker was not found in PATH. Please install Docker or start Redis manually."
    }

    $existingContainer = docker ps -a --filter "name=^/$RedisContainerName$" --format "{{.Names}}"

    if ($existingContainer -eq $RedisContainerName) {
        $runningContainer = docker ps --filter "name=^/$RedisContainerName$" --format "{{.Names}}"

        if ($runningContainer -eq $RedisContainerName) {
            Write-Host "Redis container '$RedisContainerName' is already running."
        }
        else {
            Write-Host "Starting existing Redis container '$RedisContainerName'."
            docker start $RedisContainerName | Out-Null
        }
    }
    else {
        Write-Host "Creating Redis container '$RedisContainerName'."
        docker run -d `
            --name $RedisContainerName `
            -p "${RedisPort}:6379" `
            redis:7-alpine | Out-Null
    }

    Write-Host "Redis ready on localhost:$RedisPort"
}

function Start-BinaryProcess {
    param(
        [string]$Title,
        [string]$BinaryName,
        [hashtable]$Environment = @{}
    )

    Write-Step "Starting $Title"

    $binaryPath = Join-Path $PSScriptRoot "target\debug\$BinaryName.exe"
    if (-not (Test-Path $binaryPath)) {
        throw "Binary not found: $binaryPath (build may have failed)."
    }

    $envCommands = ""
    foreach ($entry in $Environment.GetEnumerator()) {
        $key = $entry.Key
        $value = $entry.Value
        $escapedValue = "$value".Replace("'", "''")
        $envCommands += "`$env:$key = '$escapedValue'; "
    }

    $escapedBinaryPath = $binaryPath.Replace("'", "''")
    $command = "$envCommands & '$escapedBinaryPath'"

    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $command `
        -WindowStyle Normal

    Write-Host "$Title started in a new PowerShell window."
}

function Start-Launchers {
    param(
        [int]$Count
    )

    if ($Count -le 0) {
        return
    }

    for ($i = 1; $i -le $Count; $i++) {
        Start-BinaryProcess `
            -Title "Launcher" `
            -BinaryName "launcher" `
            -Environment @{
            "GATEKEEPER_URL" = "http://127.0.0.1:$GateKeeperPort"
        }
    }
}

function Wait-Seconds {
    param(
        [int]$Seconds,
        [string]$Reason
    )

    Write-Host "Waiting $Seconds seconds: $Reason"
    Start-Sleep -Seconds $Seconds
}

function Build-Workspace {
    Write-Step "Building workspace"

    cargo build --workspace
    if ($LASTEXITCODE -ne 0) {
        throw "Workspace build failed with exit code $LASTEXITCODE."
    }

    Write-Host "Workspace build completed."
}

Write-Step "MMORPG Lab startup"

if (-not (Test-CommandExists "cargo")) {
    throw "Cargo was not found in PATH. Please install Rust."
}

Build-Workspace

Start-BinaryProcess `
    -Title "Broker" `
    -BinaryName "broker" `
    -Environment @{
    "BROKER_PORT" = $BrokerPort
}


Wait-Seconds -Seconds 5 -Reason "allow broker to start"


Start-Redis

Wait-Seconds -Seconds 2 -Reason "allow Redis to accept connections"

Start-BinaryProcess `
    -Title "Orchestrator" `
    -BinaryName "orchestrator" `
    -Environment @{
    "REDIS_URL" = "redis://127.0.0.1:$RedisPort"
    "ORCH_ADDR" = "127.0.0.1:$OrchestratorPort"
    "HOT_SERVERS_MIN" = "$HotServersMin"
    "FIRST_DS_PORT" = "$FirstDedicatedServerPort"
    "ZONE" = "$Zone"
    "DS_BINARY" = "gameserver"
    "SCALER_INTERVAL_SECONDS" = "10"
}

Wait-Seconds -Seconds 8 -Reason "allow Orchestrator to load up"

Start-BinaryProcess `
    -Title "Spatial Service" `
    -BinaryName "spatial_service" `
    -Environment @{
    "QUAD_TREE_MAX_DEPTH" = 4
    "BROKER_HOST" = "127.0.0.1"
    "BROKER_PORT" = $BrokerPort
    "ORCH_HOST" = "127.0.0.1"
    "ORCH_PORT" = $OrchestratorPort + 1
}
Wait-Seconds -Seconds 3 -Reason "allow spatial service to start"


Start-BinaryProcess `
    -Title "GateKeeper" `
    -BinaryName "gatekeeper" `
    -Environment @{
    "REDIS_URL" = "redis://127.0.0.1:$RedisPort"
    "GATEKEEPER_ADDR" = "127.0.0.1:$GateKeeperPort"
    "GATEKEEPER_HTTP_ADDRESS" = "127.0.0.1:$GateKeeperPort"
    "RUST_LOG" = "info"
    "BROKER_HOST" = "127.0.0.1"
    "BROKER_PORT" = $BrokerPort
}

Wait-Seconds -Seconds 3 -Reason "allow GateKeeper to start"

Start-Launchers -Count $Launchers


Wait-Seconds -Seconds 2 -Reason "allow Launcher window initialization"


Write-Step "All services launched"

Write-Host "Redis:              localhost:$RedisPort"
Write-Host "Orchestrator:       127.0.0.1:$OrchestratorPort"
Write-Host "GateKeeper:         http://127.0.0.1:$GateKeeperPort"
Write-Host "Launchers started: $Launchers"
Write-Host ""
Write-Host "Launcher flow for replication testing." -ForegroundColor Green