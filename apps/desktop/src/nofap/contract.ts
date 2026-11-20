// Compile-time check of `types.ts` against the fixtures the Rust golden test generates. Nothing here runs:
// `pnpm typecheck` is the test. Three directions are covered:
//   1. every value Rust serialises fits the TypeScript type (`satisfies`);
//   2. Rust sends no field the TypeScript type lacks (`ExtraKeys` is `never`);
//   3. every member of a closed union is something Rust can send (`Exclude<Union, fixture values>` is `never`).

import { commands, views, type DeepReadonly } from './fixtures/golden';
import type {
  AiProvider,
  AiReadiness,
  ApplyOutcome,
  CardAction,
  CardKind,
  ChangeKind,
  GoldenViews,
  GuardLimit,
  GuardState,
  Issue,
  Mood,
  NofapChange,
  NofapCommand,
  NofapEvent,
  Os,
  Overall,
  PlainTierState,
  PreviewKey,
  PreviewPlaceholder,
  ProductId,
  ReasonCode,
  ResolverId,
  Tier,
  Trigger,
  UserEvent,
  VisionOff,
  VisionState,
} from './types';

/* 1. Every fixture value fits its type. */
commands satisfies DeepReadonly<NofapCommand[]>;
views satisfies DeepReadonly<GoldenViews>;

/* 2. No field in a fixture that the type lacks. `V` is the literal fixture type, `T` the declared one. */
type Primitive = string | number | boolean | null | undefined;
type ExtraKeys<V, T> = V extends Primitive
  ? never
  : V extends readonly (infer VE)[]
    ? T extends readonly (infer TE)[]
      ? ExtraKeys<VE, TE>
      : never
    : T extends unknown
      ? T extends object
        ? V extends DeepReadonly<T>
          ? Exclude<keyof V, keyof T> | { [K in keyof V & keyof T]: ExtraKeys<V[K], T[K]> }[keyof V & keyof T]
          : never
        : never
      : never;

/** Compiles only when `T` is `never`; otherwise the error names what is left over. */
type None<T> = [T] extends [never] ? true : { leftover: T };
const ok = <T extends true>(): T => true as T;

ok<None<ExtraKeys<typeof commands, NofapCommand[]>>>();
ok<None<ExtraKeys<typeof views, GoldenViews>>>();

/* 3. No union member that Rust never sends. */
type E = typeof views.enums;
ok<None<Exclude<Overall, E['overall'][number]>>>();
ok<None<Exclude<GuardLimit, E['guard_limit'][number]>>>();
ok<None<Exclude<VisionOff, E['vision_off'][number]>>>();
ok<None<Exclude<Issue, E['issue'][number]>>>();
ok<None<Exclude<ChangeKind, E['change_kind'][number]>>>();
ok<None<Exclude<CardKind, E['card_kind'][number]>>>();
ok<None<Exclude<CardAction, E['card_action'][number]>>>();
ok<None<Exclude<Mood, E['mood'][number]>>>();
ok<None<Exclude<Trigger, E['trigger'][number]>>>();
ok<None<Exclude<AiReadiness, E['ai_readiness'][number]>>>();
ok<None<Exclude<AiProvider, E['ai_provider'][number]>>>();
ok<None<Exclude<ProductId, E['product'][number]>>>();
ok<None<Exclude<Os, E['os'][number]>>>();
ok<None<Exclude<Tier, E['tier'][number]>>>();
ok<None<Exclude<ResolverId, E['resolver'][number]>>>();
ok<None<Exclude<ReasonCode, E['reason'][number]>>>();

// Tagged unions: every tag of the TypeScript union appears in a fixture.
type Cmd = (typeof commands)[number];
ok<None<Exclude<NofapCommand['cmd'], Cmd['cmd']>>>();
ok<None<Exclude<NofapChange['kind'], Extract<Cmd, { cmd: 'request' }>['args']['kind']>>>();
ok<None<Exclude<NofapChange['kind'], ChangeKind> | Exclude<ChangeKind, NofapChange['kind']>>>();
ok<None<Exclude<UserEvent['event'], Extract<Cmd, { cmd: 'record' }>['args']['event']>>>();
ok<None<Exclude<NofapEvent['type'], (typeof views.events)[number]['type']>>>();
ok<None<Exclude<ApplyOutcome['outcome'], (typeof views.apply_outcomes)[number]['outcome']>>>();
type Status = (typeof views.statuses)[number];
ok<None<Exclude<PlainTierState | 'unsupported', (Status['hosts'] | Status['resolver'] | Status['policies'])['state']>>>();
ok<None<Exclude<GuardState['state'], Status['guard']['state']>>>();
ok<None<Exclude<VisionState['state'], Status['vision']['state']>>>();
ok<None<Exclude<'available' | 'gated' | 'unsupported', (typeof views.view.capabilities)['hosts' | 'resolver' | 'policies']['state']>>>();
type Line = (typeof views.preview_lines)[number];
ok<None<Exclude<PreviewKey, Line['key']>>>();
ok<None<Exclude<PreviewPlaceholder, NonNullable<Line['placeholder']>>>>();
