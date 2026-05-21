param(
    [string]$RedisContainerName = "redis-mmorpg",
    [int]$RedisPort = 6379,
    [int]$GateKeeperPort = 3000,
    [int]$OrchestratorPort = 9000,
    [int]$FirstDedicatedServerPort = 7001,
    [int]$HotServersMin = 1,
    [string]$Zone = "zone_A",
    [int]$DirectGameClients = 3
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

function Start-DirectGameClients {
    param(
        [int]$Count,
        [int]$ServerPort,
        [string]$ServerIp = "127.0.0.1",
        [string]$ServerZone
    )

    if ($Count -le 0) {
        return
    }

    for ($i = 1; $i -le $Count; $i++) {
        $playerId = "direct-player-$i"
        $username = "DirectClient$i"

        Start-BinaryProcess `
            -Title "GameClient (Direct #$i)" `
            -BinaryName "gameclient" `
            -Environment @{
            "PLAYER_ID" = $playerId
            "USERNAME" = $username
            "GAME_SERVER_IP" = $ServerIp
            "GAME_SERVER_PORT" = "$ServerPort"
            "GAME_SERVER_ZONE" = $ServerZone
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

Wait-Seconds -Seconds 8 -Reason "allow Orchestrator to spawn Dedicated Server and publish heartbeat"

Start-BinaryProcess `
    -Title "GateKeeper" `
    -BinaryName "gatekeeper" `
    -Environment @{
    "REDIS_URL" = "redis://127.0.0.1:$RedisPort"
    "GATEKEEPER_ADDR" = "127.0.0.1:$GateKeeperPort"
    "GATEKEEPER_HTTP_ADDRESS" = "127.0.0.1:$GateKeeperPort"
    "RUST_LOG" = "info"
}

Wait-Seconds -Seconds 3 -Reason "allow GateKeeper to start"

Start-BinaryProcess `
    -Title "Launcher" `
    -BinaryName "launcher" `
    -Environment @{
    "GATEKEEPER_URL" = "http://127.0.0.1:$GateKeeperPort"
}

Wait-Seconds -Seconds 2 -Reason "allow Launcher window initialization"

Start-DirectGameClients `
    -Count $DirectGameClients `
    -ServerPort $FirstDedicatedServerPort `
    -ServerZone $Zone

Write-Step "All services launched"

Write-Host "Redis:              localhost:$RedisPort"
Write-Host "Orchestrator:       127.0.0.1:$OrchestratorPort"
Write-Host "GateKeeper:         http://127.0.0.1:$GateKeeperPort"
Write-Host "Launcher:           started"
Write-Host "Direct GameClients: $DirectGameClients started"
Write-Host ""
Write-Host "Launcher flow + direct clients are running for replication testing." -ForegroundColor Green