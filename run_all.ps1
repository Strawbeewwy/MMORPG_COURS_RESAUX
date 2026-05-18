param(
    [string]$RedisContainerName = "redis-mmorpg",
    [int]$RedisPort = 6379,
    [int]$GateKeeperPort = 3000,
    [int]$OrchestratorPort = 9000,
    [int]$FirstDedicatedServerPort = 7001,
    [int]$HotServersMin = 1,
    [string]$Zone = "zone_A"
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

function Start-CargoProcess {
    param(
        [string]$Title,
        [string]$Package,
        [hashtable]$Environment = @{}
    )

    Write-Step "Starting $Title"

    $envCommands = ""

    foreach ($entry in $Environment.GetEnumerator()) {
        $key = $entry.Key
        $value = $entry.Value
        $escapedValue = "$value".Replace("'", "''")
        $envCommands += "`$env:$key = '$escapedValue'; "
    }

    $command = "$envCommands cargo run -p $Package"

    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $command `
        -WindowStyle Normal

    Write-Host "$Title started in a new PowerShell window."
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

    cargo build

    Write-Host "Workspace build completed."
}

Write-Step "MMORPG Lab startup"

if (-not (Test-CommandExists "cargo")) {
    throw "Cargo was not found in PATH. Please install Rust."
}

Start-Redis

Build-Workspace

Wait-Seconds -Seconds 2 -Reason "allow Redis to accept connections"

Start-CargoProcess `
    -Title "Orchestrator" `
    -Package "orchestrator" `
    -Environment @{
        "REDIS_URL" = "redis://127.0.0.1:$RedisPort"
        "ORCH_ADDR" = "127.0.0.1:$OrchestratorPort"
        "HOT_SERVERS_MIN" = "$HotServersMin"
        "FIRST_DS_PORT" = "$FirstDedicatedServerPort"
        "ZONE" = "$Zone"
        "DS_BINARY" = "gameserver"
        "SCALER_INTERVAL_SECONDS" = "10"
    }

Wait-Seconds -Seconds 8 -Reason "allow Orchestrator to spawn Dedicated Server and publish heartbeat"

Start-CargoProcess `
    -Title "GateKeeper" `
    -Package "gatekeeper" `
    -Environment @{
        "REDIS_URL" = "redis://127.0.0.1:$RedisPort"
        "GATEKEEPER_ADDR" = "127.0.0.1:$GateKeeperPort"
    }

Wait-Seconds -Seconds 3 -Reason "allow GateKeeper to start"

Start-CargoProcess `
    -Title "Launcher" `
    -Package "launcher" `
    -Environment @{
        "GATEKEEPER_URL" = "http://127.0.0.1:$GateKeeperPort"
    }

Write-Step "All services launched"

Write-Host "Redis:        localhost:$RedisPort"
Write-Host "Orchestrator: 127.0.0.1:$OrchestratorPort"
Write-Host "GateKeeper:   http://127.0.0.1:$GateKeeperPort"
Write-Host "Launcher:     started"
Write-Host ""
Write-Host "The Launcher will start the GameClient after successful login." -ForegroundColor Green