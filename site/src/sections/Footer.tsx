import { LangToggle } from '../components/LangToggle';
import { LogoMark } from '../components/LogoMark';
import { CONTACT_EMAIL, DOCS_URL, RELEASES_URL, REPO_URL } from '../config';
import { useCopy } from '../i18n';

export function Footer() {
  const { copy } = useCopy();
  const f = copy.footer;
  const links: Array<[string, string]> = [
    [f.repo, REPO_URL],
    [f.releases, RELEASES_URL],
    [f.docs, DOCS_URL],
  ];
  if (CONTACT_EMAIL) links.push([f.contact, `mailto:${CONTACT_EMAIL}`]);
  return (
    <footer className="border-t border-line">
      <div className="container-site flex flex-col gap-8 py-12 md:flex-row md:items-start md:justify-between">
        <div className="max-w-sm">
          <span className="flex items-center gap-2.5">
            <LogoMark size={26} />
            <span className="display text-[16px]">
              ubiq<span className="text-volt">X</span> <span className="font-medium text-ink-2">AI</span>
            </span>
          </span>
          <p className="mt-3 text-sm leading-6 text-ink-2">{f.tagline}</p>
        </div>
        <nav aria-label={copy.nav.footerLinks} className="flex flex-col gap-2 text-sm">
          {links.map(([label, href]) => (
            <a key={href} href={href} className="text-ink-2 underline-offset-4 hover:text-ink hover:underline" rel="noopener">
              {label}
            </a>
          ))}
        </nav>
        <LangToggle />
      </div>
      <div className="container-site flex flex-col gap-2 border-t border-line py-5 text-xs text-ink-3 sm:flex-row sm:justify-between">
        <span>{f.rights}</span>
        <a href={`${REPO_URL}/tree/main/site`} className="underline-offset-4 hover:text-ink-2 hover:underline" rel="noopener">
          {f.source}
        </a>
      </div>
    </footer>
  );
}
