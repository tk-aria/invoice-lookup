import { readFileSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';

const CSV_PATH = 'history.csv';
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36';
const BASE_URL = 'https://www.invoice-kohyo.nta.go.jp/regno-search/detail';
const TNUM_COL = 4;

function parseCSVLine(line) {
    const fields = [];
    let current = '';
    let inQuotes = false;
    for (const ch of line) {
        if (ch === '"') { inQuotes = !inQuotes; }
        else if (ch === ',' && !inQuotes) { fields.push(current); current = ''; }
        else { current += ch; }
    }
    fields.push(current);
    return fields;
}

function normalizeTnum(raw) {
    raw = raw.trim();
    if (!raw || raw === 'なし') return null;
    let digits = raw.startsWith('T') ? raw.slice(1) : raw;
    if (digits.length === 13 && /^\d{13}$/.test(digits)) return digits;
    return null;
}

function checkTnum(digits) {
    const url = `${BASE_URL}?selRegNo=${digits}`;
    try {
        const html = execSync(
            `curl -s -A "${UA}" --connect-timeout 10 --max-time 15 "${url}"`,
            { encoding: 'utf-8', timeout: 20000 }
        );
        if (html.includes('検索対象の登録番号は存在しません')) return false;
        if (html.includes('登録番号')) return true;
        return null;
    } catch (e) {
        console.error(`  ERROR checking T${digits}: ${e.message}`);
        return null;
    }
}

// Read CSV
const content = readFileSync(CSV_PATH, 'utf-8');
const lines = content.split('\n').filter(l => l.trim());
const header = parseCSVLine(lines[0]);
const dataRows = lines.slice(1).map(l => parseCSVLine(l));

console.log(`Loaded ${dataRows.length} data rows`);

// Collect unique T-numbers
const uniqueDigits = new Map();
for (const row of dataRows) {
    if (row.length > TNUM_COL) {
        const digits = normalizeTnum(row[TNUM_COL]);
        if (digits && !uniqueDigits.has(digits)) {
            uniqueDigits.set(digits, null);
        }
    }
}

console.log(`Found ${uniqueDigits.size} unique T-numbers to check`);

// Check each unique T-number
let idx = 0;
for (const [digits] of uniqueDigits) {
    idx++;
    const registered = checkTnum(digits);
    uniqueDigits.set(digits, registered);
    const status = registered === true ? '登録済' : (registered === false ? '未登録' : '確認不可');
    console.log(`  [${idx}/${uniqueDigits.size}] T${digits} => ${status}`);
}

// Update rows
header.push('登録状況');
for (const row of dataRows) {
    if (row.length > TNUM_COL) {
        const digits = normalizeTnum(row[TNUM_COL]);
        if (digits) {
            const reg = uniqueDigits.get(digits);
            row.push(reg === true ? '登録済' : (reg === false ? '未登録' : '確認不可'));
        } else {
            row.push('番号なし');
        }
    } else {
        row.push('番号なし');
    }
}

// Write CSV back
const outputLines = [header.join(',')];
for (const row of dataRows) {
    // Re-quote fields containing commas
    const quoted = row.map(f => f.includes(',') ? `"${f}"` : f);
    outputLines.push(quoted.join(','));
}
writeFileSync(CSV_PATH, outputLines.join('\n') + '\n', 'utf-8');

// Summary
const counts = { '登録済': 0, '未登録': 0, '番号なし': 0, '確認不可': 0 };
for (const row of dataRows) {
    const s = row[row.length - 1];
    if (counts[s] !== undefined) counts[s]++;
}
console.log(`\n=== Summary ===`);
console.log(`  登録済: ${counts['登録済']}`);
console.log(`  未登録: ${counts['未登録']}`);
console.log(`  番号なし: ${counts['番号なし']}`);
console.log(`  確認不可: ${counts['確認不可']}`);
console.log(`  合計: ${dataRows.length} rows`);
console.log(`Updated ${CSV_PATH}`);
