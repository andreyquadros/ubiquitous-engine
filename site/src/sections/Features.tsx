import { Bot, FileText, Keyboard, Languages, Radar, ShieldCheck, Tags, Target } from 'lucide-react';
import { SectionHeader } from '../components/SectionHeader';
import { useCopy } from '../i18n';

const ICONS = [Radar, Tags, FileText, Keyboard, Target, ShieldCheck, Bot, Languages];
const TONES = ['text-volt', 'text-violet', 'text-signal', 'text-amber', 'text-ember', 'text-signal', 'text-volt', 'text-violet'];

export function Features() {
  const { copy } = useCopy();
  const s = copy.features;
  return (
    <section id="recursos" className="border-y border-line bg-panel/40 py-16 sm:py-24" aria-labelledby="features-title">
      <div className="container-site">
        <SectionHeader id="features-title" eyebrow={s.eyebrow} title={s.title} intro={s.intro} />
        <ul className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {s.items.map((f, i) => {
            const Icon = ICONS[i % ICONS.length];
            return (
              <li key={f.title} className="panel p-5">
                <Icon size={20} strokeWidth={1.75} className={TONES[i % TONES.length]} aria-hidden="true" />
                <h3 className="mt-4 text-[15px] font-semibold">{f.title}</h3>
                <p className="mt-2 text-sm leading-6 text-ink-2">{f.text}</p>
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
