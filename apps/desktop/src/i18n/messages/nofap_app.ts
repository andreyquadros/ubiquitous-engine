import type { NamespaceMessages } from '../types';

// Chrome of the standalone app (navigation, welcome, its own settings). t('nofap_app.<key>'). The module's own
// strings are in the `nofap` namespace. Seeded empty by the foundation unit; the strings belong to the standalone
// frontend.
const messages = {
  'pt-BR': {},
  en: {},
} satisfies NamespaceMessages;

export default messages;
