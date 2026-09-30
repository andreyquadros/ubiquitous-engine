export type Locale = 'pt-BR' | 'en';

/** Flat key → text map for one language. Keys are written without the namespace prefix. */
export type Messages = Record<string, string>;

/** One namespace file: the same keys in every supported language. */
export type NamespaceMessages = Record<Locale, Messages>;

export type Vars = Record<string, string | number>;
