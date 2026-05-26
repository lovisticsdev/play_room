param(
    [string]$Config = "examples/server.toml"
)

cargo run -p play-room-server -- --config $Config
