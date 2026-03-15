#!/usr/bin/env python3
"""Check invoice registration status for all T-numbers in history.csv"""

import csv
import re
import subprocess
import sys
import time

CSV_PATH = "history.csv"
UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
BASE_URL = "https://www.invoice-kohyo.nta.go.jp/regno-search/detail"
TNUM_COL = 4  # 0-indexed: column E = "インボイス番号"

def check_tnum(digits, ua=UA):
    """Check if a T-number is registered. Returns True/False/None."""
    url = f"{BASE_URL}?selRegNo={digits}"
    try:
        result = subprocess.run(
            ["curl", "-s", "-A", ua, "--connect-timeout", "10", "--max-time", "15", url],
            capture_output=True, text=True, timeout=20
        )
        html = result.stdout
        if "検索対象の登録番号は存在しません" in html:
            return False
        if "登録番号" in html:
            return True
        return None
    except Exception as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return None

def normalize_tnum(raw):
    """Normalize T-number. Returns (digits, is_valid)."""
    raw = raw.strip()
    if not raw or raw == "なし":
        return None, False
    if raw.startswith("T"):
        digits = raw[1:]
    else:
        digits = raw
    if len(digits) == 13 and digits.isdigit():
        return digits, True
    return None, False

def main():
    # Read CSV
    with open(CSV_PATH, "r", encoding="utf-8") as f:
        reader = csv.reader(f)
        rows = list(reader)

    if not rows:
        print("Empty CSV")
        return

    header = rows[0]
    data = rows[1:]
    print(f"Loaded {len(data)} data rows")

    # Collect unique T-numbers
    unique_digits = {}
    for row in data:
        if len(row) > TNUM_COL:
            digits, valid = normalize_tnum(row[TNUM_COL])
            if valid and digits not in unique_digits:
                unique_digits[digits] = None  # placeholder

    print(f"Found {len(unique_digits)} unique T-numbers to check")

    # Check each unique T-number
    for i, digits in enumerate(unique_digits.keys()):
        registered = check_tnum(digits)
        unique_digits[digits] = registered
        status = "登録済" if registered else ("未登録" if registered is False else "確認不可")
        print(f"  [{i+1}/{len(unique_digits)}] T{digits} => {status}")
        time.sleep(0.1)  # polite delay

    # Update rows
    header.append("登録状況")
    for row in data:
        if len(row) > TNUM_COL:
            digits, valid = normalize_tnum(row[TNUM_COL])
            if valid:
                reg = unique_digits.get(digits)
                if reg is True:
                    row.append("登録済")
                elif reg is False:
                    row.append("未登録")
                else:
                    row.append("確認不可")
            else:
                row.append("番号なし")
        else:
            row.append("番号なし")

    # Write back
    with open(CSV_PATH, "w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(header)
        writer.writerows(data)

    # Summary
    registered = sum(1 for r in data if r[-1] == "登録済")
    unregistered = sum(1 for r in data if r[-1] == "未登録")
    no_number = sum(1 for r in data if r[-1] == "番号なし")
    unknown = sum(1 for r in data if r[-1] == "確認不可")
    print(f"\n=== Summary ===")
    print(f"  登録済: {registered}")
    print(f"  未登録: {unregistered}")
    print(f"  番号なし: {no_number}")
    print(f"  確認不可: {unknown}")
    print(f"  合計: {len(data)} rows")
    print(f"Updated {CSV_PATH}")

if __name__ == "__main__":
    main()
