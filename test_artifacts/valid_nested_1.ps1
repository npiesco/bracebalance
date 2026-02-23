# PowerShell valid nested brackets
# Tests: here-strings, double/single quoted strings, line comments

# Here-string with decoy braces (must be stripped by sanitizer)
$hereDouble = @"
    config = {
        items: [1, 2, 3],
        nested: { key: (value) }
    }
"@

$hereSingle = @'
    another { fake [ bracket ] } decoy (here)
'@

# Regular strings with decoys
$trap1 = "ignore {these} [brackets] (inside) a string"
$trap2 = 'also {ignore} [these] (too)'

function Invoke-Pipeline {
    param(
        [hashtable]$Config,
        [scriptblock[]]$Stages
    )

    $result = @{}

    foreach ($key in $Config.Keys) {
        $value = $Config[$key]
        foreach ($stage in $Stages) {
            $value = & $stage $value
        }
        $result[$key] = $value
    }

    return $result
}

function Build-Nested {
    param([string[]]$Keys)

    $outer = @{}
    foreach ($k in $Keys) {
        $inner = @{
            count = $k.Length
            upper = $k.ToUpper()
            chars = @($k.ToCharArray())
        }
        $outer[$k] = $inner
    }
    return $outer
}

$pipeline = Invoke-Pipeline -Config @{
    alpha = @(1, 2, 3)
    beta  = @(4, 5, 6)
} -Stages @(
    { param($x) $x | ForEach-Object { $_ * 2 } },
    { param($x) $x | Where-Object   { $_ -gt 4 } }
)

$nested = Build-Nested -Keys @('foo', 'bar', 'baz')

foreach ($k in $nested.Keys) {
    Write-Host "$k -> $($nested[$k].upper) (count=$($nested[$k].count))"
}
