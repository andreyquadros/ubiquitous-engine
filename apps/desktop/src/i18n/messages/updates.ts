import type { NamespaceMessages } from '../types';

// 'updates' namespace: the update banner (every page) and the strings the Settings updates section shares with it.
// Keys are used as t('updates.<key>').
const messages = {
  'pt-BR': {
    'banner.title': 'Nova versão do ubiqX disponível: build {date} ({sha})',
    'banner.badge': 'Atualização disponível',
    // download button, one label per installer kind of the feed (ReleaseInfo.kind)
    'download.dmg': 'Baixar (.dmg)',
    'download.exe': 'Baixar (.exe)',
    'download.msi': 'Baixar (.msi)',
    'download.appimage': 'Baixar (.AppImage)',
    'download.deb': 'Baixar (.deb)',
    later: 'Depois',
    how_to_install: 'Como instalar',
    release_page: 'Página do release',
    build: 'build {date} ({sha})',
    dev_build: 'build de desenvolvimento',
    'toast.open_failed': 'Não foi possível abrir o download',
    // in-app update: o botão, a barra de progresso e os desfechos (lib/updater.ts)
    install_now: 'Atualizar agora',
    'install.checking': 'Procurando o pacote…',
    'install.downloading': 'Baixando {done} de {total}',
    'install.downloading_unknown': 'Baixando… {done}',
    'install.installing': 'Instalando…',
    'install.relaunching': 'Reabrindo o ubiqX…',
    'install.progress_label': 'Progresso da atualização',
    'install.failed': 'A atualização automática falhou: {error}',
    'install.unavailable': 'Este build não consegue se atualizar sozinho. Baixe o instalador e instale à mão.',
    'install.retry': 'Tentar de novo',
    // aviso honesto do macOS: a CI assina o bundle ad hoc, então o sistema vê um app novo
    'macos.warning': 'No macOS a nova versão vem assinada pela CI sem identidade fixa, então o sistema vai tratá-la como outro app: Gravação de Tela e Automação terão de ser concedidas de novo, e o Keychain pode voltar a pedir a chave de IA.',
    'macos.confirm': 'Atualizar mesmo assim',
    'macos.cancel': 'Cancelar',
  },
  en: {
    'banner.title': 'New ubiqX version available: build {date} ({sha})',
    'banner.badge': 'Update available',
    'download.dmg': 'Download (.dmg)',
    'download.exe': 'Download (.exe)',
    'download.msi': 'Download (.msi)',
    'download.appimage': 'Download (.AppImage)',
    'download.deb': 'Download (.deb)',
    later: 'Later',
    how_to_install: 'How to install',
    release_page: 'Release page',
    build: 'build {date} ({sha})',
    dev_build: 'development build',
    'toast.open_failed': 'Could not open the download',
    install_now: 'Update now',
    'install.checking': 'Looking for the package…',
    'install.downloading': 'Downloading {done} of {total}',
    'install.downloading_unknown': 'Downloading… {done}',
    'install.installing': 'Installing…',
    'install.relaunching': 'Reopening ubiqX…',
    'install.progress_label': 'Update progress',
    'install.failed': 'The automatic update failed: {error}',
    'install.unavailable': 'This build cannot update itself. Download the installer and install it by hand.',
    'install.retry': 'Try again',
    'macos.warning': 'On macOS the new version is signed by CI without a stable identity, so the system will treat it as a different app: Screen Recording and Automation will have to be granted again, and the Keychain may ask for the AI key once more.',
    'macos.confirm': 'Update anyway',
    'macos.cancel': 'Cancel',
  },
} satisfies NamespaceMessages;

export default messages;
