import { ChevronDown } from 'lucide-react';
import { SectionHeader } from '../components/SectionHeader';
import { useCopy } from '../i18n';

export function Faq() {
  const { copy } = useCopy();
  const s = copy.faq;
  return (
    <section id="perguntas" className="border-t border-line bg-panel/40 py-16 sm:py-24" aria-labelledby="faq-title">
      <div className="container-site grid gap-10 lg:grid-cols-12">
        <div className="lg:col-span-4">
          <SectionHeader id="faq-title" eyebrow={s.eyebrow} title={s.title} />
        </div>
        <div className="lg:col-span-8">
          <div className="panel divide-y divide-line">
            {s.items.map((item, i) => (
              <details key={item.q} className="group" open={i === 0}>
                <summary className="flex cursor-pointer list-none items-center justify-between gap-4 px-5 py-4 text-[15px] font-semibold marker:content-none [&::-webkit-details-marker]:hidden">
                  {item.q}
                  <ChevronDown size={18} strokeWidth={1.75} className="shrink-0 text-ink-3 transition-transform group-open:rotate-180" aria-hidden="true" />
                </summary>
                <p className="px-5 pb-5 text-sm leading-6 text-ink-2">{item.a}</p>
              </details>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
