param(
    [Parameter(Mandatory=$false)]
    [string]$Name = "player"
)

cargo run -p play-room-client -- --name $Name
