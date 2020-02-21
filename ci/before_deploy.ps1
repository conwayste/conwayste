# This script takes care of packaging the build artifacts that will go in the
# release zipfile

$SRC_DIR = $pwd.Path
$STAGE = [System.Guid]::NewGuid().ToString()

Set-Location $env:TEMP
New-Item -Type Directory -Name $STAGE
Set-Location $STAGE
New-Item -Type Directory -Name resources

$ZIP = "$SRC_DIR\conwayste-$($env:APPVEYOR_REPO_TAG_NAME)-$($env:TARGET).zip"

Copy-Item "$SRC_DIR\target\$($env:TARGET)\release\client.exe" ".\"
Copy-Item -Path "$SRC_DIR\resources" -Destination ".\" -Recurse
Copy-Item -Path "$SRC_DIR\resources\*" -Destination ".\resources\" -Recurse

7z a "$ZIP" *

Push-AppveyorArtifact "$ZIP"

Remove-Item *.* -Force
Set-Location ..
Remove-Item $STAGE
Set-Location $SRC_DIR
