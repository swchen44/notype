param([Parameter(Mandatory = $true)][string]$OutputPath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    $speaker.SelectVoice('Microsoft Zira Desktop')
    $format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
    $speaker.SetOutputToWaveFile([System.IO.Path]::GetFullPath($OutputPath), $format)
    $speaker.Speak('Please keep the number forty two and do not delete the API.')
} finally { $speaker.Dispose() }
Get-FileHash -LiteralPath $OutputPath
