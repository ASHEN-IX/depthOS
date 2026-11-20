.pragma library

// Qt plain-text rendering handles markup. Strip directional overrides and
// terminal controls, and flatten line separators in compact metadata labels.
function safeText(value) {
    return String(value === null || value === undefined ? "" : value)
        .replace(/[\u0000-\u001F\u007F-\u009F\u061C\u200E\u200F\u2028-\u202E\u2066-\u206F]/g, " ")
        .replace(/ +/g, " ")
        .trim();
}

function countLabel(count, singular, plural) {
    return count + " " + (count === 1 ? singular : plural);
}
