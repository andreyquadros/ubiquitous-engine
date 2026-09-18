import type { NamespaceMessages } from '../types';

// 'updates' namespace: the update banner (every page) and the strings the Settings updates section shares with it.
// Keys are used as t('updates.<key>').
const messages = {
  'pt-BR': {
    'banner.title': 'Nova versão do ubiqX disponível: build {date} ({sha})',
    'banner.badge': 'Atualização disponível',
    download: 'Baixar (.dmg)',
    later: 'Depois',
    how_to_install: 'Como instalar',
    release_page: 'Página do release',
    build: 'build {date} ({sha})',
    dev_build: 'build de desenvolvimento',
    'toast.open_failed': 'Não foi possível abrir o download',
  },
  en: {
    'banner.title': 'New ubiqX version available: build {date} ({sha})',
    'banner.badge': 'Update available',
    download: 'Download (.dmg)',
    later: 'Later',
    how_to_install: 'How to install',
    release_page: 'Release page',
    build: 'build {date} ({sha})',
    dev_build: 'development build',
    'toast.open_failed': 'Could not open the download',
  },
} satisfies NamespaceMessages;

export default messages;
