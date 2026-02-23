# PowerShell broken nested brackets
# Here-strings with decoy braces must be stripped by sanitizer

$hereDouble = @"
    fake { [unclosed brackets] (inside) here-string }
"@

$hereSingle = @'
    more {fake} [decoys] (here)
'@

# Regular string decoys
$trap = "ignore {this} [bracket] (content)"

function Invoke-BrokenPipeline {
    param(
        [hashtable]$Config,
        [scriptblock[]]$Stages
    )

    $result = @{}

    foreach ($key in $Config.Keys) {
        $value = $Config[$key]

        foreach ($stage in $Stages) {
            $value = & $stage $value

            if ($value -gt 0) {
                $result[$key] += $value

                # <-- missing } for if block

            # <-- missing } for inner foreach

        # <-- missing } for outer foreach

    # <-- missing } for function Invoke-BrokenPipeline
