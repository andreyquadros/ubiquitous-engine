import clsx from 'clsx';
import { AlertTriangle, Archive, ArchiveRestore, ChevronDown, ChevronUp, Lock, Plus, Tags, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button, IconButton } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { CategoryChip } from '../components/ui/CategoryChip';
import { Dialog } from '../components/ui/Dialog';
import { Field, Input, Select, Textarea } from '../components/ui/Field';
import { IconPicker } from '../components/ui/IconPicker';
import { EmptyState } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { TagInput } from '../components/ui/TagInput';
import { Toggle } from '../components/ui/Toggle';
import { useT } from '../i18n';
import { blankCategory, categoryDescription, categoryLabel, COLOR_CHOICES, iconFor } from '../lib/categories';
import { fmtDateTime, hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { Category, Rule, RuleMatcher } from '../lib/types';
import { useAsync } from '../lib/useAsync';

/** Rule matchers in menu order; label and example hint come from t('categories.matcher.<m>') / t('categories.matcher_hint.<m>'). */
const MATCHERS: readonly RuleMatcher[] = ['domain', 'app', 'title_contains', 'regex'];

/** Tinted square with the category icon: the category's own colour, never a token. */
function IconTile({ color, icon, size = 'md' }: { color: string; icon: string; size?: 'sm' | 'md' | 'lg' }) {
  const Icon = iconFor(icon);
  const box = size === 'lg' ? 'size-12 rounded-card' : size === 'sm' ? 'size-7 rounded-[8px]' : 'size-8 rounded-[8px]';
  const glyph = size === 'lg' ? 'size-[22px]' : size === 'sm' ? 'size-3.5' : 'size-4';
  return (
    <span
      className={clsx('flex shrink-0 items-center justify-center', box)}
      style={{ background: `color-mix(in oklab, ${color} 16%, transparent)`, color, boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${color} 28%, transparent)` }}
      aria-hidden
    >
      <Icon className={glyph} strokeWidth={1.75} />
    </span>
  );
}

export function Categories() {
  const t = useT();
  const { data: cats, reload: reloadCats, setData: setCats } = useAsync(() => ipc.listCategories(true), []);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<Category | null>(null);
  const [isNew, setIsNew] = useState(false);
  const toast = useToast();

  const sorted = useMemo(() => {
    const list = cats ?? [];
    return {
      user: list.filter((c) => !c.is_system && !c.archived).sort((a, b) => a.sort_order - b.sort_order),
      archived: list.filter((c) => !c.is_system && c.archived),
      system: list.filter((c) => c.is_system).sort((a, b) => a.sort_order - b.sort_order),
    };
  }, [cats]);

  useEffect(() => {
    if (!cats?.length) return;
    if (!selectedId) {
      const first = sorted.user[0] ?? sorted.system[0];
      if (first) setSelectedId(first.id);
    }
  }, [cats, selectedId, sorted]);

  useEffect(() => {
    if (isNew) return;
    const c = cats?.find((x) => x.id === selectedId);
    setDraft(c ? structuredClone(c) : null);
  }, [selectedId, cats, isNew]);

  const select = (id: string) => {
    setIsNew(false);
    setSelectedId(id);
  };

  const startNew = () => {
    setIsNew(true);
    setSelectedId(null);
    setDraft(blankCategory(sorted.user.length));
  };

  const save = async () => {
    if (!draft) return;
    if (!draft.name.trim()) {
      toast.error(t('categories.toast.name_required'));
      return;
    }
    if (!draft.is_system && !draft.description.trim()) {
      toast.error(t('categories.toast.description_required'), t('categories.toast.description_required_hint'));
      return;
    }
    try {
      const saved = await ipc.saveCategory(draft);
      await reloadCats();
      await loadCategories();
      setIsNew(false);
      setSelectedId(saved.id);
      toast.success(t('categories.toast.saved'));
    } catch (e) {
      toast.error(t('categories.toast.save_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const archive = async (c: Category, archived: boolean) => {
    try {
      await ipc.saveCategory({ ...c, archived });
      await reloadCats();
      await loadCategories();
      toast.success(archived ? t('categories.toast.archived') : t('categories.toast.restored'));
    } catch (e) {
      toast.error(t('categories.toast.archive_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const remove = async (c: Category) => {
    if (!window.confirm(t('categories.confirm_remove', { name: categoryLabel(c) }))) return;
    try {
      await ipc.deleteCategory(c.id);
      setCats((cats ?? []).filter((x) => x.id !== c.id));
      await loadCategories();
      setSelectedId(null);
      toast.success(t('categories.toast.removed'));
    } catch (e) {
      toast.error(t('categories.toast.remove_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  /** Drag-free ordering: swap the sort order with the neighbour and persist both. */
  const move = async (c: Category, dir: -1 | 1) => {
    const list = sorted.user;
    const i = list.findIndex((x) => x.id === c.id);
    const other = list[i + dir];
    if (i < 0 || !other) return;
    const a = { ...c, sort_order: i + dir };
    const b = { ...other, sort_order: i };
    try {
      await ipc.saveCategory(a);
      await ipc.saveCategory(b);
      await reloadCats();
      await loadCategories();
    } catch (e) {
      toast.error(t('categories.toast.reorder_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const hasUser = sorted.user.length > 0 || sorted.archived.length > 0;

  return (
    <div data-testid="page-categories">
      <PageHeader
        title={t('categories.title')}
        subtitle={t('categories.subtitle')}
        actions={
          <Button variant="primary" icon={<Plus className="size-4" strokeWidth={1.75} />} onClick={startNew}>
            {t('categories.new_category')}
          </Button>
        }
      />

      <div className="grid grid-cols-12 gap-5">
        <Card padded={false} className="col-span-12 self-start min-[1100px]:col-span-4">
          {cats && !hasUser && !isNew ? (
            <EmptyState
              icon={<Tags className="size-5" strokeWidth={1.75} />}
              title={t('categories.empty.title')}
              description={t('categories.empty.body')}
              action={
                <Button variant="primary" size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={startNew}>
                  {t('categories.empty.action')}
                </Button>
              }
            />
          ) : (
            <CategoryList title={t('categories.list.yours')} items={sorted.user} selectedId={selectedId} onSelect={select} onMove={(c, d) => void move(c, d)} />
          )}
          {sorted.archived.length > 0 && <CategoryList title={t('categories.list.archived')} items={sorted.archived} selectedId={selectedId} onSelect={select} muted />}
          <CategoryList title={t('categories.list.system')} items={sorted.system} selectedId={selectedId} onSelect={select} locked />
        </Card>

        <Card className="col-span-12 min-[1100px]:col-span-8">
          {draft ? (
            <CategoryForm
              key={draft.id}
              draft={draft}
              onChange={setDraft}
              onSave={() => void save()}
              onArchive={() => void archive(draft, !draft.archived)}
              onDelete={() => void remove(draft)}
              isNew={isNew}
            />
          ) : (
            <EmptyState
              icon={<Tags className="size-5" strokeWidth={1.75} />}
              title={t('categories.pick.title')}
              description={t('categories.pick.body')}
              action={
                <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={startNew}>
                  {t('categories.new_category')}
                </Button>
              }
            />
          )}
        </Card>
      </div>

      <RulesSection categories={cats ?? []} />
    </div>
  );
}

function CategoryList({
  title,
  items,
  selectedId,
  onSelect,
  onMove,
  locked,
  muted,
}: {
  title: string;
  items: Category[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onMove?: (c: Category, dir: -1 | 1) => void;
  locked?: boolean;
  muted?: boolean;
}) {
  const t = useT();
  return (
    <section className="border-b border-line last:border-b-0" aria-label={title}>
      <div className="flex items-baseline justify-between px-4 pt-3.5 pb-1">
        <p className="eyebrow">{title}</p>
        <span className="num text-[11px] text-ink-4">{items.length}</span>
      </div>
      <ul className="px-2 pb-2">
        {items.map((c, i) => {
          const active = c.id === selectedId;
          const meta = c.is_productive
            ? c.report_time
              ? t('categories.meta.productive_at', { time: hhmmToInput(c.report_time) })
              : t('categories.meta.productive')
            : t('categories.meta.not_work');
          return (
            <li key={c.id} className="group relative">
              {active && <span className="absolute top-2.5 bottom-2.5 -left-2 w-[3px] rounded-r-full bg-volt" aria-hidden />}
              <button
                type="button"
                onClick={() => onSelect(c.id)}
                aria-current={active ? 'true' : undefined}
                className={clsx(
                  'flex min-h-11 w-full items-center gap-3 rounded-control py-1.5 pr-2 pl-2.5 text-left text-sm transition-colors duration-120',
                  active ? 'bg-volt-soft text-ink' : 'text-ink hover:bg-panel-2',
                  muted && 'opacity-60',
                )}
              >
                <IconTile color={c.color} icon={c.icon} />
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{categoryLabel(c)}</span>
                  <span className="block truncate text-[11px] text-ink-3">{c.is_system ? t('categories.meta.system') : meta}</span>
                </span>
                {locked && <Lock className="size-3.5 shrink-0 text-ink-4" strokeWidth={1.75} aria-hidden />}
              </button>
              {onMove && items.length > 1 && (
                <span className={clsx('absolute top-1/2 right-1.5 flex -translate-y-1/2 flex-col opacity-0 transition-opacity duration-120 group-hover:opacity-100 focus-within:opacity-100', active && 'opacity-100')}>
                  <IconButton label={t('categories.move_up', { name: c.name })} size="sm" className="size-5 rounded-[6px]" disabled={i === 0} onClick={() => onMove(c, -1)}>
                    <ChevronUp className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                  <IconButton label={t('categories.move_down', { name: c.name })} size="sm" className="size-5 rounded-[6px]" disabled={i === items.length - 1} onClick={() => onMove(c, 1)}>
                    <ChevronDown className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                </span>
              )}
            </li>
          );
        })}
        {!items.length && <li className="px-2.5 py-2 text-xs text-ink-3">{t('categories.list.empty')}</li>}
      </ul>
    </section>
  );
}

function CategoryForm({ draft, onChange, onSave, onArchive, onDelete, isNew }: { draft: Category; onChange: (c: Category) => void; onSave: () => void; onArchive: () => void; onDelete: () => void; isNew: boolean }) {
  const t = useT();
  const ro = draft.is_system;
  const set = (patch: Partial<Category>) => onChange({ ...draft, ...patch });
  const selectedColor = draft.color.toUpperCase();

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(e) => {
        e.preventDefault();
        onSave();
      }}
    >
      <div className="flex items-center gap-4">
        <IconTile color={draft.color} icon={draft.icon} size="lg" />
        <div className="min-w-0 flex-1">
          <h2 className="display truncate text-[20px] leading-7">{categoryLabel(draft) || t('categories.new_category')}</h2>
          <p className="mt-0.5 text-xs text-ink-3">
            {ro ? t('categories.form.system_note') : isNew ? t('categories.form.unsaved') : t('categories.form.created_at', { date: fmtDateTime(draft.created_at) })}
          </p>
        </div>
        {ro && (
          <Badge tone="neutral">
            <Lock className="size-3" strokeWidth={1.75} aria-hidden /> {t('categories.badge.system')}
          </Badge>
        )}
        {draft.archived && <Badge tone="amber">{t('categories.badge.archived')}</Badge>}
      </div>

      <div className="grid grid-cols-1 gap-4 min-[720px]:grid-cols-2">
        <Field label={t('categories.field.name')}>
          {(id) => <Input id={id} value={ro ? categoryLabel(draft) : draft.name} disabled={ro} onChange={(e) => set({ name: e.target.value })} placeholder={t('categories.field.name_placeholder')} required />}
        </Field>
        <Field label={t('categories.field.report_time')} hint={t('categories.field.report_time_hint')}>
          {(id) => <Input id={id} type="time" disabled={ro} value={hhmmToInput(draft.report_time)} onChange={(e) => set({ report_time: e.target.value ? inputToHhmm(e.target.value) : null })} className="num" />}
        </Field>
      </div>

      <Field label={t('categories.field.description')} hint={t('categories.field.description_hint')}>
        {(id) => <Textarea id={id} value={ro ? categoryDescription(draft) : draft.description} disabled={ro} onChange={(e) => set({ description: e.target.value })} placeholder={t('categories.field.description_placeholder')} className="min-h-24" />}
      </Field>

      <Field label={t('categories.field.keywords')} hint={t('categories.field.keywords_hint')}>
        {(id) => <TagInput id={id} value={draft.keywords} disabled={ro} onChange={(keywords) => set({ keywords })} placeholder={t('categories.field.keywords_placeholder')} />}
      </Field>

      <div className="grid grid-cols-1 gap-5 min-[720px]:grid-cols-[auto_1fr] min-[720px]:gap-8">
        <div className="flex flex-col gap-2">
          <span className="text-[13px] font-medium">{t('categories.field.color')}</span>
          <div className="grid grid-cols-7 gap-2" role="radiogroup" aria-label={t('categories.field.color')}>
            {COLOR_CHOICES.map((c) => {
              const on = selectedColor === c;
              return (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  aria-label={c}
                  onClick={() => set({ color: c })}
                  className="size-7 rounded-full transition-[box-shadow,transform] duration-120 hover:scale-105"
                  style={{ background: c, boxShadow: on ? `0 0 0 2px var(--panel), 0 0 0 4px ${c}` : undefined }}
                />
              );
            })}
            <label
              className="relative size-7 cursor-pointer overflow-hidden rounded-full transition-[box-shadow] duration-120"
              title={t('categories.field.custom_color')}
              style={{
                background: COLOR_CHOICES.includes(selectedColor) ? 'conic-gradient(from 0deg, #ff7a1f, #ffc24d, #2ee6a6, #4d8dff, #9b8cff, #ff5c7a, #ff7a1f)' : draft.color,
                boxShadow: COLOR_CHOICES.includes(selectedColor) ? undefined : `0 0 0 2px var(--panel), 0 0 0 4px ${draft.color}`,
              }}
            >
              <input type="color" value={draft.color} onChange={(e) => set({ color: e.target.value.toUpperCase() })} className="absolute inset-0 size-full cursor-pointer opacity-0" aria-label={t('categories.field.custom_color')} />
              <Plus className="absolute top-1/2 left-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 text-white drop-shadow-[0_1px_1px_rgb(0_0_0/.5)]" strokeWidth={2.5} aria-hidden />
            </label>
          </div>
        </div>
        <div className="flex flex-col gap-2">
          <span className="text-[13px] font-medium">{t('categories.field.icon')}</span>
          <IconPicker value={draft.icon} onChange={(icon) => set({ icon })} color={draft.color} label={t('categories.field.icon')} className="max-w-md" />
        </div>
      </div>

      <Field label={t('categories.field.template')} hint={t('categories.field.template_hint')}>
        {(id) => <Textarea id={id} value={draft.report_template ?? ''} disabled={ro} onChange={(e) => set({ report_template: e.target.value || null })} placeholder={t('categories.field.template_placeholder')} className="font-mono text-xs" />}
      </Field>

      <Field label={t('categories.field.productive')} hint={t('categories.field.productive_hint')} inline>
        {(id) => <Toggle id={id} checked={draft.is_productive} disabled={ro} onChange={(v) => set({ is_productive: v })} />}
      </Field>

      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line pt-4">
        <div className="flex gap-1">
          {!ro && !isNew && (
            <>
              <Button variant="ghost" icon={draft.archived ? <ArchiveRestore className="size-4" strokeWidth={1.75} /> : <Archive className="size-4" strokeWidth={1.75} />} onClick={onArchive}>
                {draft.archived ? t('categories.action.restore') : t('categories.action.archive')}
              </Button>
              <Button variant="ghost" className="text-rose hover:bg-rose/10 hover:text-rose" icon={<Trash2 className="size-4" strokeWidth={1.75} />} onClick={onDelete}>
                {t('common.remove')}
              </Button>
            </>
          )}
        </div>
        <Button type="submit" variant="primary" size="lg">
          {isNew ? t('categories.create_category') : t('common.save_changes')}
        </Button>
      </div>
    </form>
  );
}

function RulesSection({ categories }: { categories: Category[] }) {
  const t = useT();
  const { data: rules, reload, setData } = useAsync(() => ipc.listRules(), []);
  const [open, setOpen] = useState(false);
  const [matcher, setMatcher] = useState<RuleMatcher>('domain');
  const [pattern, setPattern] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const options = categories.filter((c) => !c.archived && c.id !== 'sys-private' && c.id !== 'sys-uncategorized');

  useEffect(() => {
    if (!categoryId && options[0]) setCategoryId(options[0].id);
  }, [options, categoryId]);

  const toggle = async (r: Rule, enabled: boolean) => {
    try {
      const saved = await ipc.saveRule({ ...r, enabled });
      setData((rules ?? []).map((x) => (x.id === saved.id ? saved : x)));
    } catch (e) {
      toast.error(t('categories.toast.rule_update_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const remove = async (r: Rule) => {
    try {
      await ipc.deleteRule(r.id);
      setData((rules ?? []).filter((x) => x.id !== r.id));
      toast.success(t('categories.toast.rule_removed'));
    } catch (e) {
      toast.error(t('categories.toast.remove_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const create = async () => {
    if (!pattern.trim() || !categoryId) return;
    setBusy(true);
    try {
      await ipc.saveRule({
        id: crypto.randomUUID(),
        category_id: categoryId,
        matcher,
        pattern: pattern.trim(),
        priority: 10,
        origin: 'user',
        enabled: true,
        created_at: new Date().toISOString(),
        hit_count: 0,
        miss_count: 0,
        last_contradicted_at: null,
      });
      await reload();
      setPattern('');
      setOpen(false);
      toast.success(t('categories.toast.rule_created'));
    } catch (e) {
      toast.error(t('categories.toast.rule_create_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const learned = (rules ?? []).filter((r) => r.origin === 'learned').length;

  return (
    <Card className="mt-5" padded={false}>
      <div className="px-5 pt-5">
        <CardHeader
          title={t('categories.rules.title')}
          subtitle={
            rules?.length
              ? t('categories.rules.subtitle', {
                  rules: t('categories.rules.count', { count: rules.length }),
                  learned: t('categories.rules.learned', { count: learned }),
                })
              : t('categories.rules.subtitle_empty')
          }
          action={
            <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={() => setOpen(true)}>
              {t('categories.rules.new')}
            </Button>
          }
        />
      </div>
      {rules?.length ? (
        <table className="w-full text-sm">
          <thead>
            <tr className="border-y border-line text-left text-xs text-ink-3">
              <th className="px-5 py-2 font-medium">{t('categories.rules.col.rule')}</th>
              <th className="px-3 py-2 font-medium">{t('common.category')}</th>
              <th className="px-3 py-2 font-medium">{t('categories.rules.col.origin')}</th>
              <th className="px-3 py-2 text-right font-medium">{t('categories.rules.col.hits')}</th>
              <th className="px-3 py-2 font-medium">{t('categories.rules.col.enabled')}</th>
              <th className="px-3 py-2" />
            </tr>
          </thead>
          <tbody className="divide-y divide-line">
            {rules.map((r) => (
              <tr key={r.id} className={clsx('transition-opacity duration-150', !r.enabled && 'opacity-50')}>
                <td className="px-5 py-2.5">
                  <span className="mr-2 inline-flex h-5 items-center rounded-md border border-line bg-panel-2 px-1.5 text-[11px] text-ink-3">{t(`categories.matcher.${r.matcher}`)}</span>
                  <span className="font-mono text-[13px]">{r.pattern}</span>
                </td>
                <td className="px-3 py-2.5">
                  <CategoryChip categories={categories} categoryId={r.category_id} />
                </td>
                <td className="px-3 py-2.5">
                  <Badge tone={r.origin === 'learned' ? 'violet' : 'volt'}>{r.origin === 'learned' ? t('categories.rules.origin.learned') : t('categories.rules.origin.user')}</Badge>
                </td>
                <td className="num px-3 py-2.5 text-right text-xs text-ink-2">
                  <span className="text-signal">{r.hit_count}</span>
                  <span className="mx-1 text-ink-4">/</span>
                  <span className={r.miss_count > 0 ? 'text-rose' : 'text-ink-3'}>{r.miss_count}</span>
                  {r.last_contradicted_at && (
                    <AlertTriangle className="ml-1.5 inline size-3.5 align-[-2px] text-amber" strokeWidth={1.75} aria-label={t('categories.rules.last_contradicted', { date: fmtDateTime(r.last_contradicted_at) })} />
                  )}
                </td>
                <td className="px-3 py-2.5">
                  <Toggle size="sm" checked={r.enabled} onChange={(v) => void toggle(r, v)} label={t('categories.rules.enable', { pattern: r.pattern })} />
                </td>
                <td className="px-3 py-2.5 text-right">
                  <IconButton label={t('categories.rules.remove')} size="sm" onClick={() => void remove(r)}>
                    <Trash2 className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="border-t border-line">
          <EmptyState
            className="py-8"
            title={t('categories.rules.empty.title')}
            description={t('categories.rules.empty.body')}
            action={
              <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={() => setOpen(true)}>
                {t('categories.rules.create')}
              </Button>
            }
          />
        </div>
      )}

      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title={t('categories.rules.new')}
        description={t('categories.rules.dialog_body')}
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button variant="primary" onClick={() => void create()} loading={busy} disabled={!pattern.trim()}>
              {t('categories.rules.create')}
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <Field label={t('categories.rules.field.type')}>
            {(id) => (
              <Select id={id} value={matcher} onChange={(e) => setMatcher(e.target.value as RuleMatcher)}>
                {MATCHERS.map((m) => (
                  <option key={m} value={m}>
                    {t(`categories.matcher.${m}`)}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t('categories.rules.field.pattern')} hint={t(`categories.matcher_hint.${matcher}`)}>
            {(id) => <Input id={id} value={pattern} onChange={(e) => setPattern(e.target.value)} className="font-mono" />}
          </Field>
          <Field label={t('common.category')}>
            {(id) => (
              <Select id={id} value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
                {options.map((c) => (
                  <option key={c.id} value={c.id}>
                    {categoryLabel(c)}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>
      </Dialog>
    </Card>
  );
}
