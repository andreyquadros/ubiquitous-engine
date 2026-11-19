import type { NamespaceMessages } from '../types';

// The `nofap` module, shared by ubiqX and the standalone app: the page, the wizard, the card window and the
// counselling copy bank. t('nofap.<key>'). `{name}` is the display name the engine sends (`display_name` of the
// view), never a literal in a string. Seeded empty by the foundation unit; the strings belong to the module's
// frontend.
const messages = {
  'pt-BR': {},
  en: {},
} satisfies NamespaceMessages;

export default messages;
