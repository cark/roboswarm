@REM rem cargo build --profile dist --target wasm32-unknown-unknown

@REM rd /Q /S web_release
@REM mkdir web_release
@REM copy target\wasm32-unknown-unknown\dist\robo-swarm.wasm web_release\*.*
@REM copy build\web\styles.css web_release\*.*

@REM @echo off
@REM SET RUSTFLAGS=%RUSTFLAGS% --profile dist
trunk build --release --public-url ./
powershell -Command "(gc dist\index.html) -replace '/\./', './' | Out-File dist\index.html"

@REM pause
