param([Parameter(Mandatory = $true)][string]$RequestPath)
$ErrorActionPreference = 'Stop'
$request = Get-Content -LiteralPath $RequestPath -Raw | ConvertFrom-Json
$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = $request.binary
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
if ($request.cwd) { $start.WorkingDirectory = [string]$request.cwd }
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
foreach ($argument in $request.arguments) { $start.ArgumentList.Add([string]$argument) }
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $start
# Warm the managed memory/reader APIs before starting a potentially short query.
$self = [System.Diagnostics.Process]::GetCurrentProcess()
$self.Refresh()
[void]$self.PeakWorkingSet64
$self.Dispose()
$emptyReader = [IO.StreamReader]::new([IO.MemoryStream]::new([byte[]]::new(0)))
[void]$emptyReader.ReadToEndAsync().GetAwaiter().GetResult()
$emptyReader.Dispose()
$observedPeak = 0L
$watch = [System.Diagnostics.Stopwatch]::StartNew()
[void]$process.Start()
$process.Refresh()
$observedPeak = [Math]::Max($observedPeak, [long]$process.PeakWorkingSet64)
$stdout = $process.StandardOutput.ReadToEndAsync()
$stderr = $process.StandardError.ReadToEndAsync()
while (-not $process.WaitForExit(1)) {
    $process.Refresh()
    $observedPeak = [Math]::Max($observedPeak, [long]$process.PeakWorkingSet64)
}
$watch.Stop()
$process.Refresh()
$capturedOutput = $stdout.GetAwaiter().GetResult()
if ($request.stdout_path) {
    [IO.File]::WriteAllText([string]$request.stdout_path, $capturedOutput, [Text.UTF8Encoding]::new($false))
}
$record = @{
    exit_code = $process.ExitCode
    elapsed_ms = $watch.Elapsed.TotalMilliseconds
    observed_peak_working_set_bytes = $observedPeak
    memory_sample_interval_ms = 1
    cpu_ms = $process.TotalProcessorTime.TotalMilliseconds
    stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($capturedOutput)
    stderr = $stderr.GetAwaiter().GetResult()
}
$process.Dispose()
$record | ConvertTo-Json -Compress
