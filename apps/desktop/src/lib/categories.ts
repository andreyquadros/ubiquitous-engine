import {
  Ban,
  BookOpen,
  Briefcase,
  Building2,
  CircleDashed,
  Code,
  Coffee,
  EyeOff,
  FlaskConical,
  Globe,
  GraduationCap,
  Heart,
  Landmark,
  Laptop,
  Lightbulb,
  Mail,
  MessageSquare,
  Microscope,
  Music,
  Palette,
  PenTool,
  Presentation,
  Rocket,
  Sparkles,
  Star,
  Target,
  Tv,
  Users,
  Wrench,
  type LucideIcon,
} from 'lucide-react';
import { SYSTEM_CATEGORIES, type Category, type Id } from './types';

/** Curated lucide icons a category may use (kebab-case names as stored in the DB). */
export const ICON_CHOICES: { name: string; Icon: LucideIcon }[] = [
  { name: 'graduation-cap', Icon: GraduationCap },
  { name: 'rocket', Icon: Rocket },
  { name: 'building-2', Icon: Building2 },
  { name: 'briefcase', Icon: Briefcase },
  { name: 'code', Icon: Code },
  { name: 'book-open', Icon: BookOpen },
  { name: 'flask-conical', Icon: FlaskConical },
  { name: 'microscope', Icon: Microscope },
  { name: 'presentation', Icon: Presentation },
  { name: 'users', Icon: Users },
  { name: 'pen-tool', Icon: PenTool },
  { name: 'palette', Icon: Palette },
  { name: 'lightbulb', Icon: Lightbulb },
  { name: 'target', Icon: Target },
  { name: 'globe', Icon: Globe },
  { name: 'landmark', Icon: Landmark },
  { name: 'laptop', Icon: Laptop },
  { name: 'mail', Icon: Mail },
  { name: 'message-square', Icon: MessageSquare },
  { name: 'wrench', Icon: Wrench },
  { name: 'heart', Icon: Heart },
  { name: 'star', Icon: Star },
  { name: 'sparkles', Icon: Sparkles },
  { name: 'music', Icon: Music },
  { name: 'tv', Icon: Tv },
  { name: 'coffee', Icon: Coffee },
  { name: 'eye-off', Icon: EyeOff },
  { name: 'ban', Icon: Ban },
  { name: 'circle-dashed', Icon: CircleDashed },
];

const ICON_MAP = new Map(ICON_CHOICES.map((c) => [c.name, c.Icon]));

export const iconFor = (name: string | undefined | null): LucideIcon => ICON_MAP.get(name ?? '') ?? CircleDashed;

export const COLOR_CHOICES = [
  '#2563EB', '#0EA5E9', '#06B6D4', '#10B981', '#84CC16', '#EAB308',
  '#F97316', '#EF4444', '#EC4899', '#A855F7', '#6366F1', '#64748B',
];

export const UNCATEGORIZED_COLOR = '#94A3B8';

export const categoryById = (cats: Category[], id: Id | null | undefined): Category | undefined =>
  id ? cats.find((c) => c.id === id) : undefined;

export const categoryName = (cats: Category[], id: Id | null | undefined): string =>
  categoryById(cats, id)?.name ?? 'Sem categoria';

export const categoryColor = (cats: Category[], id: Id | null | undefined): string =>
  categoryById(cats, id)?.color ?? UNCATEGORIZED_COLOR;

export const isUncategorized = (id: Id | null | undefined): boolean => !id || id === SYSTEM_CATEGORIES.uncategorized;

/** Categories a user can assign by hand, in display order (user categories first, then system ones except private). */
export const assignableCategories = (cats: Category[]): Category[] => {
  const user = cats.filter((c) => !c.is_system && !c.archived).sort((a, b) => a.sort_order - b.sort_order);
  const system = cats.filter((c) => c.is_system && c.id !== SYSTEM_CATEGORIES.private && c.id !== SYSTEM_CATEGORIES.uncategorized);
  return [...user, ...system];
};

/** Categories that get daily reports. */
export const reportCategories = (cats: Category[]): Category[] =>
  cats.filter((c) => !c.is_system && !c.archived && c.is_productive).sort((a, b) => a.sort_order - b.sort_order);

export const newCategoryId = (): Id =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : `cat-${Date.now().toString(36)}`;

export const blankCategory = (sortOrder: number): Category => ({
  id: newCategoryId(),
  name: '',
  color: COLOR_CHOICES[(sortOrder + 3) % COLOR_CHOICES.length] ?? '#2563EB',
  icon: 'briefcase',
  description: '',
  keywords: [],
  report_time: null,
  report_template: null,
  is_productive: true,
  is_system: false,
  archived: false,
  sort_order: sortOrder,
  created_at: new Date().toISOString(),
});
