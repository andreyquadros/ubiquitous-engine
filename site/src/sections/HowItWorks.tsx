import { SectionHeader } from '../components/SectionHeader';
import { useCopy } from '../i18n';

export function HowItWorks() {
  const { copy } = useCopy();
  const s = copy.how;
  return (
    <section id="como-funciona" className="container-site py-16 sm:py-24" aria-labelledby="how-title">
      <SectionHeader id="how-title" eyebrow={s.eyebrow} title={s.title} intro={s.intro} />
      <ol className="mt-10 grid gap-5 md:grid-cols-3">
        {s.steps.map((step, i) => (
          <li key={step.title} className="panel relative p-5 sm:p-6">
            <span className="display num inline-flex h-10 w-10 items-center justify-center rounded-control bg-volt-soft text-lg text-volt" aria-hidden="true">
              {i + 1}
            </span>
            <h3 className="mt-4 text-base font-semibold">{step.title}</h3>
            <p className="mt-2 text-sm leading-6 text-ink-2">{step.text}</p>
          </li>
        ))}
      </ol>
    </section>
  );
}
