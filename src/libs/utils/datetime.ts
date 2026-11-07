/** 时间转本地展示：年月日时分秒显式齐备，保证行内精度到秒；locale 缺省走默认语言。 */
export function formatDateTime(iso: string, locale?: string): string {
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    hourCycle: "h23",
  }).format(new Date(iso));
}
