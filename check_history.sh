#!/bin/bash
# Check invoice registration status for all T-numbers in history.csv

CSV="history.csv"
OUT="history_checked.csv"
UA="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
BASE="https://www.invoice-kohyo.nta.go.jp/regno-search/detail"
CACHE_FILE="/tmp/tnum_cache.txt"

> "$CACHE_FILE"

# Step 1: Extract unique T-numbers
echo "=== Step 1: Extracting unique T-numbers ==="
declare -A SEEN
TNUMS=()

while IFS= read -r line; do
    tnum=$(echo "$line" | awk -v FPAT='([^,]*)|("[^"]*")' '{print $5}' | tr -d ' "')
    if [[ -z "$tnum" || "$tnum" == "なし" ]]; then
        continue
    fi
    # Normalize
    if [[ "$tnum" =~ ^T[0-9]{13}$ ]]; then
        normalized="$tnum"
    elif [[ "$tnum" =~ ^[0-9]{13}$ ]]; then
        normalized="T$tnum"
    else
        continue
    fi
    if [[ -z "${SEEN[$normalized]+x}" ]]; then
        SEEN[$normalized]=1
        TNUMS+=("$normalized")
    fi
done < <(tail -n +2 "$CSV")

echo "Found ${#TNUMS[@]} unique T-numbers"

# Step 2: Check each unique T-number
echo "=== Step 2: Checking registration status ==="
for i in "${!TNUMS[@]}"; do
    tnum="${TNUMS[$i]}"
    digits="${tnum#T}"
    url="${BASE}?selRegNo=${digits}"

    html=$(curl -s -A "$UA" --connect-timeout 10 --max-time 15 "$url" 2>/dev/null)

    if echo "$html" | grep -q "検索対象の登録番号は存在しません"; then
        status="未登録"
    else
        status="登録済"
    fi

    echo "$tnum=$status" >> "$CACHE_FILE"
    echo "  [$((i+1))/${#TNUMS[@]}] $tnum => $status"
done

# Step 3: Build lookup map and update CSV
echo "=== Step 3: Updating CSV ==="
declare -A STATUS_MAP
while IFS='=' read -r k v; do
    STATUS_MAP[$k]="$v"
    # Also map without T prefix
    digits="${k#T}"
    STATUS_MAP[$digits]="$v"
done < "$CACHE_FILE"

# Write header
IFS= read -r header < "$CSV"
echo "${header},登録状況" > "$OUT"

# Write data rows
count=0
while IFS= read -r line; do
    count=$((count + 1))
    tnum=$(echo "$line" | awk -v FPAT='([^,]*)|("[^"]*")' '{print $5}' | tr -d ' "')

    if [[ -z "$tnum" || "$tnum" == "なし" ]]; then
        status="番号なし"
    else
        # Normalize
        if [[ "$tnum" =~ ^T[0-9]{13}$ ]]; then
            normalized="$tnum"
        elif [[ "$tnum" =~ ^[0-9]{13}$ ]]; then
            normalized="T$tnum"
        else
            normalized="$tnum"
        fi
        status="${STATUS_MAP[$normalized]:-確認不可}"
    fi

    echo "${line},${status}" >> "$OUT"
done < <(tail -n +2 "$CSV")

echo "=== Done: $count rows processed ==="
echo "Output: $OUT"

# Replace original
cp "$OUT" "$CSV"
echo "Updated $CSV with registration status column"
