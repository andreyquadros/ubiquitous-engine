import { Check } from 'lucide-react';
import { Button, ButtonLink } from '../components/Button';
import { cx } from '../components/cx';
import { SectionHeader } from '../components/SectionHeader';
import { CHECKOUT_ANNUAL_URL, CHECKOUT_MONTHLY_URL } from '../config';
import { useCopy, type Plan } from '../i18n';

function PlanCard({ plan, url, featured }: { plan: Plan; url: string; featured: boolean }) {
  const { copy } = useCopy();
  const hintId = `hint-${featured ? 'monthly' : 'annual'}`;
  return (
    <article className={cx('panel relative flex flex-col p-6 sm:p-7', featured && 'border-volt/50 bg-panel-2')} aria-labelledby={`plan-${featured ? 'monthly' : 'annual'}`}>
      {plan.badge && (
        <span className="absolute -top-3 left-6 rounded-pill bg-volt px-2.5 py-1 text-xs font-semibold text-on-volt">{plan.badge}</span>
      )}
      <h3 id={`plan-${featured ? 'monthly' : 'annual'}`} className="text-base font-semibold">
        {plan.name}
      </h3>
      <p className="mt-4 flex items-baseline gap-1">
        <span className="display num text-[40px] leading-none">{plan.price}</span>
        <span className="text-sm text-ink-2">{plan.period}</span>
      </p>
      {plan.alt && <p className="num mt-1 text-sm text-ink-3">{plan.alt}</p>}
      <p className="mt-4 text-sm leading-6 text-ink-2">{plan.lead}</p>
      <ul className="mt-5 flex flex-col gap-2.5">
        {plan.bullets.map((b) => (
          <li key={b} className={cx('flex items-start gap-2.5 text-sm leading-5', b === plan.highlight ? 'font-medium text-ink' : 'text-ink-2')}>
            <Check size={16} strokeWidth={2} className={cx('mt-0.5 shrink-0', b === plan.highlight ? 'text-signal' : 'text-volt')} aria-hidden="true" />
            {b}
          </li>
        ))}
      </ul>
      <div className="mt-7 flex flex-col gap-2">
        {url ? (
          <ButtonLink href={url} size="lg" variant={featured ? 'primary' : 'secondary'} rel="noopener">
            {plan.cta}
          </ButtonLink>
        ) : (
          <>
            <Button size="lg" variant={featured ? 'primary' : 'secondary'} disabled aria-describedby={hintId}>
              {copy.pricing.soon}
            </Button>
            <p id={hintId} className="text-center text-xs text-ink-3">
              {copy.pricing.soonHint}
            </p>
          </>
        )}
      </div>
    </article>
  );
}

export function Pricing() {
  const { copy } = useCopy();
  const s = copy.pricing;
  const [annual, monthly] = s.plans;
  return (
    <section id="planos" className="border-y border-line bg-panel/40 py-16 sm:py-24" aria-labelledby="pricing-title">
      <div className="container-site">
        <SectionHeader id="pricing-title" eyebrow={s.eyebrow} title={s.title} intro={s.intro} align="center" />
        <div className="mx-auto mt-12 grid max-w-4xl gap-6 md:grid-cols-2">
          <PlanCard plan={annual} url={CHECKOUT_ANNUAL_URL} featured={false} />
          <PlanCard plan={monthly} url={CHECKOUT_MONTHLY_URL} featured />
        </div>
        <p className="mx-auto mt-6 max-w-2xl text-center text-sm text-ink-2">{s.note}</p>
        <p className="mt-1 text-center text-xs text-ink-3">{s.footnote}</p>
      </div>
    </section>
  );
}
