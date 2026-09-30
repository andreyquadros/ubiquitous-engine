/** The app's SVG mark (apps/desktop/public/ubi/favicon.svg), inlined so it stays crisp at any size. */
export function LogoMark({ size = 28, className }: { size?: number; className?: string }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width={size} height={size} className={className} aria-hidden="true" focusable="false">
      <rect x="4" y="4" width="56" height="56" rx="16" fill="#2563eb" />
      <polygon points="22,20 27,10 32,20" fill="#fff" />
      <polygon points="29,19 34,7 39,19" fill="#fff" />
      <polygon points="36,20 41,10 46,20" fill="#fff" />
      <rect x="12" y="18" width="40" height="34" rx="13" fill="#fff" />
      <rect x="17" y="24" width="30" height="22" rx="10" fill="#0f172a" />
      <path d="M22 38 Q26 31 30 38" stroke="#60a5fa" strokeWidth="2.5" fill="none" strokeLinecap="round" />
      <path d="M34 38 Q38 31 42 38" stroke="#60a5fa" strokeWidth="2.5" fill="none" strokeLinecap="round" />
    </svg>
  );
}
