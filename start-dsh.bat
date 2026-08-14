@echo off
setlocal
cd /d "%~dp0"
title dsh-gui launcher

echo Starting DeepSeek Harness (node, port 3080)...
start "dsh-server" cmd /c "node node_modules\@deepseek-ai\dsh\lib\bin.js web --host 127.0.0.1 --port 3080"

echo Waiting for http://127.0.0.1:3080 ...
powershell -NoProfile -Command ^
  "$ok=$false; for($i=0;$i -lt 60;$i++){ try{ $r=Invoke-WebRequest -UseBasicParsing http://127.0.0.1:3080/ -TimeoutSec 1; if($r.StatusCode -eq 200){$ok=$true;break} }catch{}; Start-Sleep -Milliseconds 500 }; exit $(if($ok){0}else{1})"

if errorlevel 1 (
  echo.
  echo Server did not come up in ~30s. Make sure "npm install" was run and Node.js is installed.
  pause
  exit /b 1
)

echo Opening desktop window (Edge app mode)...
start "" msedge --app=http://127.0.0.1:3080
echo.
echo DeepSeek Harness is running in a desktop window.
echo Close the "dsh-server" console window when you are done to stop it.
pause
