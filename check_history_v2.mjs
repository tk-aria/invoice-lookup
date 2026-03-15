import { readFileSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';

const CSV_PATH = 'history.csv';
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36';
const BASE_URL = 'https://www.invoice-kohyo.nta.go.jp/regno-search/detail';
const TNUM_COL = 4;
const DATE_COL = 0;

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

/** Parse "2025/01/02" → [2025, 1, 2] */
function parseCsvDate(s) {
    const parts = s.trim().split('/');
    if (parts.length !== 3) return null;
    return parts.map(Number);
}

/** Parse "令和5年10月1日" → [2023, 10, 1] */
function parseJapaneseDate(s) {
    s = s.trim();
    let offset;
    if (s.startsWith('令和')) { offset = 2018; s = s.slice(2); }
    else if (s.startsWith('平成')) { offset = 1988; s = s.slice(2); }
    else if (s.startsWith('昭和')) { offset = 1925; s = s.slice(2); }
    else return null;

    const m = s.match(/(\d+)年(\d+)月(\d+)日/);
    if (!m) return null;
    return [offset + parseInt(m[1]), parseInt(m[2]), parseInt(m[3])];
}

/** Compare [y,m,d] arrays. Returns <0, 0, or >0 */
function compareDates(a, b) {
    if (!a || !b) return 0;
    if (a[0] !== b[0]) return a[0] - b[0];
    if (a[1] !== b[1]) return a[1] - b[1];
    return a[2] - b[2];
}

function fetchInvoiceInfo(digits) {
    const url = `${BASE_URL}?selRegNo=${digits}`;
    try {
        const html = execSync(
            `curl -s -A "${UA}" --connect-timeout 10 --max-time 15 "${url}"`,
            { encoding: 'utf-8', timeout: 20000 }
        );
        if (html.includes('検索対象の登録番号は存在しません')) {
            return { registered: false, registrationDate: null };
        }
        // Extract registration date from HTML
        let regDate = null;
        const labelMatches = [...html.matchAll(/<h3[^>]*class="[^"]*itemlabel[^"]*"[^>]*>(.*?)<\/h3>/gs)];
        const dataMatches = [...html.matchAll(/<p[^>]*class="[^"]*itemdata[^"]*"[^>]*>(.*?)<\/p>/gs)];
        for (let i = 0; i < labelMatches.length && i < dataMatches.length; i++) {
            const label = labelMatches[i][1].trim();
            const value = dataMatches[i][1].trim();
            if (label === '登録年月日') {
                regDate = value;
                break;
            }
        }
        return { registered: true, registrationDate: regDate };
    } catch (e) {
        console.error(`  ERROR checking T${digits}: ${e.message}`);
        return { registered: null, registrationDate: null };
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
    const info = fetchInvoiceInfo(digits);
    uniqueDigits.set(digits, info);
    const status = info.registered === true ? '登録済' : (info.registered === false ? '未登録' : '確認不可');
    const regDateStr = info.registrationDate || '-';
    console.log(`  [${idx}/${uniqueDigits.size}] T${digits} => ${status} (登録日: ${regDateStr})`);
}

// Update rows with 3-category classification
header.push('登録状況');
for (const row of dataRows) {
    if (row.length > TNUM_COL) {
        const digits = normalizeTnum(row[TNUM_COL]);
        if (digits) {
            const info = uniqueDigits.get(digits);
            if (info && info.registered === true) {
                // Compare CSV date vs registration date
                const csvDate = row.length > DATE_COL ? parseCsvDate(row[DATE_COL]) : null;
                const regDate = info.registrationDate ? parseJapaneseDate(info.registrationDate) : null;
                if (csvDate && regDate && compareDates(csvDate, regDate) < 0) {
                    row.push('登録前');
                } else {
                    row.push('登録済');
                }
            } else if (info && info.registered === false) {
                row.push('未登録');
            } else {
                row.push('確認不可');
            }
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
    const quoted = row.map(f => f.includes(',') ? `"${f}"` : f);
    outputLines.push(quoted.join(','));
}
writeFileSync(CSV_PATH, outputLines.join('\n') + '\n', 'utf-8');

// Summary
const counts = { '登録済': 0, '登録前': 0, '未登録': 0, '番号なし': 0, '確認不可': 0 };
const beforeEntries = [];
for (const row of dataRows) {
    const s = row[row.length - 1];
    if (counts[s] !== undefined) counts[s]++;
    if (s === '登録前') {
        const tnum = row.length > TNUM_COL ? row[TNUM_COL] : '?';
        const date = row.length > DATE_COL ? row[DATE_COL] : '?';
        const title = row.length > 3 ? row[3] : '?';
        beforeEntries.push(`${date} ${title} (${tnum})`);
    }
}
console.log(`\n=== Summary ===`);
console.log(`  登録済: ${counts['登録済']}`);
console.log(`  登録前: ${counts['登録前']}`);
console.log(`  未登録: ${counts['未登録']}`);
console.log(`  番号なし: ${counts['番号なし']}`);
console.log(`  確認不可: ${counts['確認不可']}`);
console.log(`  合計: ${dataRows.length} rows`);
if (beforeEntries.length > 0) {
    console.log(`\n  登録前エントリ:`);
    for (const e of beforeEntries) {
        console.log(`    - ${e}`);
    }
}
console.log(`Updated ${CSV_PATH}`);
