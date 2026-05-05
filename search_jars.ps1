$libDir = \"C:\Users\smopo\Downloads\explyt-work\bootstrap\lib\"  
$searchTerm = \"dangerous\"  
Get-ChildItem \"$libDir\*.jar\" | ForEach-Object {  
$name = $_.Name 
