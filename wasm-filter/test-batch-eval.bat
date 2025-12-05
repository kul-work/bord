@echo off
setlocal enabledelayedexpansion

set "samples[0]=I love this beautiful sunny day"
set "samples[1]=This is absolutely terrible and I hate it"
set "samples[2]=All people are worthless and should die"
set "samples[3]=The weather is okay today"

for /L %%i in (0,1,3) do (
    echo.
    echo Testing sample %%i: !samples[%%i]!
    curl -X POST "http://127.0.0.1:3000/posts" -H "Content-Type: application/json" -H "Authorization: Bearer test-integration-token-abc123" -d "{\"content\": \"!samples[%%i]!\"}"
)

endlocal
