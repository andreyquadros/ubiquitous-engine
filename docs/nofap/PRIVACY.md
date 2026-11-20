# Privacidade do NoFap

O que o NoFap olha, o que sai do seu computador e o que fica guardado, camada por camada.
Vale para os dois produtos feitos do mesmo código: o módulo dentro do ubiqX e o app separado
(no sistema ele aparece com o nome neutro `ubiqx-shield`).

O NoFap é uma ferramenta de autocontrole que uma pessoa adulta instala no próprio computador.
Ele é visível, só liga depois que você conclui o assistente e não serve para vigiar outra
pessoa. Tudo o que está aqui pode ser conferido no código; onde este texto e o código
divergirem, vale o código, e o texto está errado.

## Em uma página

| Camada | O que olha | O que sai do computador | O que fica guardado |
|---|---|---|---|
| Guarda do app | o endereço da aba que está na frente, a cada 2 segundos | nada | contagens por dia e hora, sem endereço |
| Lista no sistema | nada | nada | uma lista de sites adultos conhecidos no arquivo `hosts`, mais os sites que você digitou |
| DNS filtrado (opcional) | nada | todos os nomes de sites que o computador consulta vão para a Cloudflare, no lugar do seu provedor | a configuração de DNS do sistema; o seu DNS anterior, para mostrar a você |
| Políticas dos navegadores (opcional) | nada | nada | quatro ou cinco ajustes nos navegadores |
| Verificação por IA (opcional) | uma foto reduzida da janela que está na frente, quando o site ou o app é desconhecido | essa foto e um texto fixo que diz para que ela serve, para o provedor de IA que você escolheu | contagens por dia e hora, o custo por hora, e o site aprendido como um código embaralhado |
| Apoio (cartões, respiração, deslize, check-in) | o que você toca | nada | contagens por dia e hora; a sua frase e os seus planos, se você escrever |

O que não existe, em nenhuma camada:

- relatório, aviso ou cópia para outra pessoa (não há "parceiro de responsabilidade");
- conta, cadastro, sincronização na nuvem ou telemetria sobre o uso do NoFap;
- registro dos sites que você visitou, das telas verificadas ou das respostas da IA;
- texto de aconselhamento escrito por um modelo: as mensagens vêm de um banco revisado;
- modo escondido: o ícone fica na bandeja enquanto a proteção está ligada.

## Camada por camada

### 1. Guarda do app

**O que olha.** A cada 2 segundos, o app pergunta ao sistema qual janela está na frente. Se
for um navegador que ele sabe ler, lê o endereço da aba ativa. Em alguns casos só o título da
janela está disponível (Linux com X11), e em outros nada (Firefox no macOS, Linux com
Wayland); a página do NoFap diz quando a guarda está limitada e por quê.

**O que faz com isso.** Compara o nome do site, dentro do seu computador, com uma lista
embutida de cerca de 240 mil sites adultos, com regras de palavras e de terminações, e com a
sua lista. A lista grande vem dentro do app como códigos embaralhados, não como nomes; só os
20 mil sites mais visitados dela também vêm por extenso, porque são os que vão para o arquivo
`hosts` (camada 2). Se o site está na lista, a aba é fechada e um cartão discreto aparece.

**O que sai do computador.** Nada.

**O que fica guardado.** Uma contagem: quantos bloqueios houve em cada hora de cada dia, e de
que tipo (lista, sua lista, palavra-chave). Não fica o nome do site, o título, o endereço nem
o horário exato. O cartão que aparece na tela também não carrega o nome do site.

**Na memória, até fechar o app.** O que um botão do cartão precisa para agir (qual app, qual
site, qual janela) fica na memória por até 10 minutos e nunca vai para o disco.

**Só no ubiqX: o que o rastreador de produtividade esquece.** Quando há um bloqueio, o bloco
de atividade que estava aberto é reescrito como "privado": título, endereço, domínio e
descrição são apagados, e as capturas de tela daquele bloco são excluídas do disco. Enquanto o
site é um site bloqueado ou misto, o rastreador não tira captura dele. Limite conhecido: o
banco de dados não sobrescreve os bytes de uma linha alterada, então o título antigo pode
continuar no arquivo do banco até aquele espaço ser reutilizado.

### 2. Lista no sistema (arquivo `hosts`)

**O que é escrito.** Um bloco marcado no arquivo `hosts` do sistema, entre as linhas
`# >>> ubiqx-shield v1 BEGIN …` e `# <<< ubiqx-shield END`, com:

- um nome de teste fixo, igual em todos os computadores (`canary.ubiqx-shield.invalid`);
- os sites que você digitou em "Adicionar site", a menos que escolha "guardar só neste app";
- os sites adultos mais visitados da lista embutida: até 20.000 no macOS e no Linux, até
  8.000 no Windows.

O resto do arquivo não é tocado, e a segunda linha do bloco diz, em inglês, como desfazer.

**Quem consegue ler.** O arquivo `hosts` pode ser lido por qualquer conta deste computador.
Por isso só entram nele nomes amplamente conhecidos e os que você mesmo digitou. Os sites que
a verificação por IA aprendeu nunca entram nesse arquivo.

**O arquivo de estado.** Ao lado, numa pasta do sistema que só o administrador altera, fica
um arquivo pequeno que qualquer conta pode ler: quais camadas foram aplicadas e para qual dos
dois produtos, os sites que você digitou, o tamanho e a conferência do bloco, e quais ajustes
de navegador foram criados. Os caminhos estão em [REMOVAL.md](REMOVAL.md).

**O que sai do computador.** Nada. Um detalhe: enquanto o bloco está aplicado, o app consulta
o nome de teste a cada 30 minutos para saber se o sistema está respeitando o arquivo. Se o
sistema não estiver respeitando, essa consulta sai para o seu servidor de DNS como um nome
que não existe, e quem opera esse servidor pode reconhecer nele o nome deste programa.

### 3. DNS filtrado (opcional)

**O que muda.** O sistema passa a perguntar os nomes de sites à Cloudflare for Families
(`1.1.1.3`, `1.0.0.3` e os dois endereços IPv6 equivalentes), no lugar do servidor do seu
provedor de internet ou do que você tinha configurado.

**O que sai do computador.** Todos os nomes de sites que este computador consultar, de
qualquer programa, passam pela Cloudflare. É a Cloudflare que decide o que barrar: sites
adultos e sites que ela considera perigosos. Ela trata esses dados pelos termos dela. O ubiqX
e o NoFap não recebem nada disso: não existe servidor nosso nesse caminho.

**Uma consulta do próprio app.** Ao aplicar esta camada, e de tempos em tempos enquanto ela
está ligada, o app consulta o nome `one.one.one.one` (um nome público da Cloudflare) para
saber se a rede em que você está alcança o DNS filtrado. É uma consulta de DNS como
qualquer outra, sem identificador seu; é ela que permite avisar "esta rede não alcança o DNS
filtrado" em vez de deixar você sem internet.

**O que não muda.** As consultas continuam sem criptografia, como é o padrão do sistema: a
rede em que você está continua vendo os nomes consultados, como já via.

**O que fica guardado.** A configuração de DNS do próprio sistema. Se você tinha um DNS
configurado à mão, o app anota qual era (no banco de dados dele) para mostrar a você antes de
aplicar e de novo ao remover. Ao desligar, o DNS volta ao automático; o valor anotado não é
regravado no sistema.

**Saída sem espera.** Se a rede parar de resolver nomes, "Restaurar minha rede" devolve o DNS
ao automático na hora, sem o tempo de espera.

### 4. Políticas dos navegadores (opcional)

**O que é escrito.** Ajustes que os próprios navegadores oferecem para empresas:

| Navegador | Ajuste |
|---|---|
| Chrome, Chromium | DNS próprio do navegador desligado; SafeSearch do Google forçado |
| Edge | os mesmos, mais SafeSearch do Bing no nível estrito |
| Brave | os do Chrome, mais as janelas com Tor desligadas |
| Firefox | DNS sobre HTTPS desligado e travado |

No macOS, só o Firefox recebe ajuste. Navegação anônima não é desligada.

**O que você vai ver.** Os navegadores passam a mostrar "Gerenciado pela sua organização".
Não há organização nenhuma: é o efeito de usar esse mecanismo.

**O que sai do computador.** Nada. Com o SafeSearch forçado, a busca recebe o mesmo pedido
de sempre com o filtro ligado.

### 5. Verificação por IA (opcional)

É a camada mais sensível, porque envolve uma imagem da sua tela. Ela só liga se você marcar
"Entendi o que é enviado" no assistente, e para de funcionar na hora se você remover a chave
de IA, ativar o modo só local do ubiqX ou trocar de provedor (aí ela espera você confirmar o
novo).

**Quando uma foto é tirada.** Só da janela que está na frente, e só quando as regras locais
não conseguiram decidir:

- um site que as listas não conhecem;
- uma plataforma mista (rede social, fórum, hospedagem de imagens);
- um navegador cujo endereço o app não consegue ler;
- um player de vídeo ou visualizador de imagens (VLC, mpv, QuickTime e outros);
- uma janela anônima, do mesmo jeito que uma janela comum (o texto de consentimento avisa);
- apps de mensagens e de e-mail **só se você ligar essa opção**, que vem desligada.

**O que nunca é fotografado.**

- sites de bancos, pagamentos, governo, saúde, e-mail e documentos da lista "nunca
  fotografar", e as versões web de mensageiros (WhatsApp, Telegram, Messenger, Discord);
- endereços da sua rede local (`localhost`, roteador, servidores internos);
- sites que já foram decididos (bloqueados ou na sua lista "nunca bloquear");
- gerenciadores de senha e as janelas do próprio ubiqX e do NoFap;
- qualquer outro app que não seja navegador, player ou visualizador;
- no ubiqX, os apps e domínios das suas listas de privacidade. Uma exclusão que já existia
  quando você ligou o NoFap vale na hora; uma exclusão adicionada depois passa a valer para
  esta verificação após o tempo de espera que você escolheu.

**O que é a foto.** Uma imagem só daquela janela, nunca da tela inteira. Em navegadores, a
faixa de cima (abas, barra de endereço, favoritos) é cortada, para não levar o título de
outras abas. A imagem é reduzida para no máximo 640 pixels de lado e comprimida. Uma tela
quase igual à última que deu "nada" não é enviada de novo.

**O que é enviado.** Essa imagem e dois textos fixos em inglês, iguais para todo mundo: a
instrução de classificação e o pedido "Classify this screenshot.". A instrução diz ao
provedor de onde o pedido vem: de um app de autocontrole que uma pessoa adulta instalou no
próprio computador para ficar longe de pornografia. Ou seja, o provedor fica sabendo que a
chave ou a conta usada pertence a alguém que usa uma ferramenta assim. Não vai o endereço, o
título, o nome do app que estava na frente, o seu idioma, o seu perfil nem um identificador
seu. A resposta é uma palavra (explícito, sugestivo, nada, não sei) e um grau de certeza
(alto, médio ou baixo). A resposta tem um formato fechado, sem campo de texto livre, e a
instrução manda o modelo não descrever a imagem, não transcrever texto e não identificar
ninguém.

**Para quem.** Para o provedor de IA que está nas suas configurações:

- **Anthropic, OpenAI ou xAI, com a sua chave.** A imagem vai direto do seu computador para
  o provedor.
- **"IA do Ubi" (só no ubiqX, com um segundo consentimento).** A imagem passa pelo servidor
  do ubiqX a caminho da Anthropic. O servidor não guarda a imagem nem o conteúdo do pedido, e
  não os escreve em registro. Ele anota, como para qualquer pedido, a hora, o modelo, a
  quantidade de tokens e o custo. Uma sequência de pedidos pequenos com imagem é reconhecível:
  quem opera o servidor consegue saber que um assinante usa a verificação, mas não o que
  havia na tela.

**O que o provedor faz com a imagem.** Isso está fora do app. O provedor aplica as regras e
os sistemas de segurança dele ao que recebe e guarda pelo prazo que os termos dele definem.
Uma imagem que ele julgue ilegal pode levá-lo a suspender a chave ou a comunicar o caso,
como os termos dele e a lei de onde ele opera exigirem. O NoFap não acrescenta nenhuma
comunicação própria. Antes de ligar esta camada, leia os termos do provedor que você usa.

**O que fica guardado no seu computador.**

- A imagem: nunca. Ela fica na memória durante a chamada e é descartada; não vira arquivo,
  miniatura nem linha no banco.
- Uma contagem de verificações e de confirmações por dia e hora.
- Uma linha de custo por chamada (modelo, tokens, valor), com a hora cheia no lugar do
  horário exato, para que o livro de custos não vire um histórico minuto a minuto.
- O site aprendido: quando a IA confirma um site desconhecido, ele passa a ser bloqueado
  direto nas próximas visitas, e fica guardado só como um código embaralhado com um sal, sem
  o nome. Não é mostrado, não é exportado e não entra no arquivo `hosts`. Limite: o sal fica
  guardado ao lado, então quem tem acesso à sua conta consegue testar se um site específico
  está no conjunto, mas não consegue listar o conjunto.
- Nenhuma linha de registro por verificação. O arquivo de log do app não recebe o resultado,
  o site, o título nem o app. Uma chamada que falha deixa uma linha de aviso com o provedor e
  o erro, sem dizer de que tipo de chamada se tratava.

**Limites que você define.** Por padrão: até 20 verificações por hora, 120 por dia e
US$ 2,00 por mês. Ao atingir um limite, a verificação para e a página diz por quê. Uma tela
que não foi verificada na hora não é verificada depois.

**"Ver um exemplo do que é enviado".** Mostra, na página, a imagem reduzida da última janela
que seria candidata. Essa imagem não é enviada nem guardada.

**Pausa e modo privado do ubiqX.** Pausar o ubiqX ou usar o modo privado dele **não** pausa
esta verificação. O texto de consentimento diz isso.

### 6. Apoio: cartões, respiração, deslize, check-in

- As mensagens vêm de um banco de frases revisado. Nenhuma é escrita por um modelo, e nada do
  que você faz nesses fluxos é enviado a alguém.
- O que você toca vira contagem por dia e hora: respirações iniciadas e concluídas, pedidos
  de respiro, deslizes que você mesmo registrou, check-ins e os gatilhos que você marcou,
  pedidos de desligar e cancelamentos, saídas do app com a proteção ligada. A nota de 0 a 10
  no fim da respiração aparece na tela e não é guardada.
- A sua frase de valores e os planos "se… então…" ficam guardados em texto legível no banco
  de dados do app.
- O histórico de deslizes não é um diário: é só o número por dia e hora, e só você o inicia.
  Um bloqueio feito pela IA nunca é contado como deslize.
- Notificações do sistema: quando o cartão não pode ser mostrado, o app usa uma notificação
  com o título "UBI" e uma frase neutra. O sistema operacional guarda o próprio histórico de
  notificações, com horário, e o app não consegue apagá-lo. Para limpar: no macOS, abra a
  Central de Notificações (clique no relógio) e use o "x" do grupo; no Windows, abra a
  central de notificações (Win + N no Windows 11, Win + A no Windows 10) e escolha "Limpar
  tudo"; no GNOME, abra a lista pelo relógio e escolha "Limpar".

## O que fica guardado, e onde

| Onde | O quê |
|---|---|
| Banco de dados do app (SQLite, sem criptografia) | suas escolhas e consentimentos; a frase e os planos; pedidos de mudança pendentes; os sites da sua lista, em texto; sites aprendidos e adiamentos, só como código embaralhado; contagens por dia e hora; o custo das verificações; o seu DNS anterior |
| Cofre do sistema (Keychain, Gerenciador de Credenciais, Secret Service) | a chave do provedor de IA |
| Arquivo `hosts` do sistema | o bloco descrito acima |
| Pasta de estado do sistema | o arquivo de estado descrito acima |
| Configuração de DNS e políticas dos navegadores | o que as camadas 3 e 4 escreveram |
| Pasta compartilhada entre os dois produtos | um arquivo que diz qual dos dois está fazendo a guarda agora (produto, número do processo, horário do último sinal); existe só enquanto a proteção roda |
| Pasta de exportações | o arquivo que você pediu em "Exportar dados", se pediu |
| Arquivo de log do app | mudanças de estado (ligou, camada aplicada, verificação em espera); nunca um bloqueio, site, título ou resultado |

O banco de dados fica na pasta de dados de cada produto:

| | macOS | Windows | Linux |
|---|---|---|---|
| ubiqX (`ubiqx.db`) | `~/Library/Application Support/ai.ubiqx.app/` | `%AppData%\ai.ubiqx.app\` | `~/.local/share/ai.ubiqx.app/` |
| app separado (`shield.db`) | `~/Library/Application Support/ai.ubiqx.shield/` | `%AppData%\ai.ubiqx.shield\` | `~/.local/share/ai.ubiqx.shield/` |

Os dois produtos não compartilham banco, chave nem configurações. O app separado não tem
rastreador de produtividade: as tabelas de atividade, capturas e relatórios existem no banco
dele e ficam vazias.

## Por quanto tempo, e como apagar

| Dado | Fica até | "Apagar meu histórico do NoFap" | "Apagar todos os dados" (ubiqX) | Exportação |
|---|---|---|---|---|
| Imagens da verificação | nunca são gravadas | | | |
| Memória do app (chaves de janela, limites, cartões) | fechar o app | limpa | limpa | não |
| Contagens por dia e hora | 90 dias | apaga | apaga | sim |
| Escolhas, consentimentos, pedidos pendentes | você mudar | mantém | **mantém** | sim |
| Frase de valores e planos | você mudar | apaga | mantém | sim |
| Sites da sua lista | você remover | mantém | mantém | sim |
| Sites aprendidos (códigos) | "Esquecer sites aprendidos", que passa pelo tempo de espera | mantém | mantém | só a quantidade |
| Adiamentos de "Não era isso" | vencerem (até 24 horas) | apaga | mantém | não |
| Custo das verificações | como os demais custos de IA | mantém | apaga | não |
| Bloco no `hosts`, DNS, políticas, arquivo de estado | você remover do sistema | não toca | não toca | não |

- **"Apagar todos os dados" do ubiqX não desliga o NoFap de propósito.** Se apagasse as
  escolhas, seria um botão de desligar sem o tempo de espera que você mesmo escolheu.
- **"Apagar meu histórico do NoFap" sobrescreve o que apaga.** As contagens, os adiamentos e
  as versões anteriores das suas escolhas são sobrescritos no arquivo do banco, e não só
  marcados como livres. Se outro programa estiver com o banco aberto ao mesmo tempo (a linha
  de comando do ubiqX, por exemplo), parte disso só é consolidada quando ele fechar.
- **"Apagar todos os dados" não sobrescreve.** Ele apaga as linhas; os bytes podem continuar
  no arquivo até serem reutilizados.
- **"Apagar tudo do NoFap"** (escolhas, listas e histórico) existe só com a proteção
  desligada.
- **Exportação.** "Exportar dados do NoFap" grava um arquivo na pasta de exportações do app.
  Ele não inclui o sal nem os códigos dos sites aprendidos. A exportação geral do ubiqX não
  inclui nada do NoFap, porque é a que uma pessoa costuma mandar para outra.
- **Remover do computador.** O que o produto pode deixar no sistema, e os comandos para tirar
  cada coisa mesmo sem o app, estão em [REMOVAL.md](REMOVAL.md).

## Quem consegue ver o quê

| Quem | O que consegue ver |
|---|---|
| Outra conta neste computador | que o programa está instalado e rodando (nome do processo, ícone); o bloco no arquivo `hosts`, com os sites que você digitou; o arquivo de estado; o DNS e as políticas do sistema |
| Quem entra na sua conta | tudo acima, mais o banco de dados: suas escolhas, a frase e os planos, a sua lista, as contagens por dia e hora. Não vê sites visitados, telas nem resultados, porque não são guardados. Consegue testar se um site específico foi aprendido |
| Um administrador do computador | tudo acima, e consegue remover a proteção |
| A rede e o provedor de internet | os nomes de sites consultados, como antes. Com o DNS filtrado, as consultas vão para a Cloudflare |
| A Cloudflare (só com o DNS filtrado) | os nomes de sites que este computador consulta e o endereço de onde vêm |
| O provedor de IA (só com a verificação por IA) | as imagens enviadas, pela chave ou conta usada, e que elas vêm de um app de autocontrole contra pornografia (o texto do pedido diz isso) |
| O servidor do ubiqX (só com a "IA do Ubi") | que uma verificação foi feita: hora, modelo, tokens e custo. Nunca a imagem |
| Qualquer outra pessoa | nada. O NoFap não envia nada a ninguém |

## Senha de administrador

As camadas 2, 3 e 4 mudam o sistema, então pedem a senha de administrador uma vez por
mudança, pelo próprio sistema operacional, depois de mostrar a lista exata do que vai mudar.
O que convém saber sobre esse pedido:

- No macOS, a janela de senha fala em `osascript`, que é a ferramenta do sistema usada para
  pedir a permissão.
- No Windows, enquanto o instalador não for assinado, a janela diz "fornecedor desconhecido".
- No Linux, o pedido vem do `pkexec`. O AppImage não consegue pedir essa permissão; nesse
  formato as camadas do sistema ficam indisponíveis e a página diz isso.
- O programa que recebe a permissão é o próprio app, que executa só quatro ações fixas (ver,
  mostrar o comando, aplicar, remover) e só chama ferramentas do sistema pelo caminho
  completo. Ele não tem como ser usado para rodar outra coisa.
- Onde a sua conta consegue alterar o arquivo do app (instalação por usuário no Windows, app
  em `~/Applications` ou arrastado para `/Applications` por uma conta administradora, versão
  de desenvolvimento), ele é tão confiável quanto essa conta, como qualquer programa
  instalado por usuário: outro programa rodando na sua conta poderia trocá-lo e esperar o
  próximo pedido de senha.
- Não há processo privilegiado em segundo plano. Nada é reaplicado sozinho: se a proteção do
  sistema mudar, o app avisa e só refaz com um novo pedido de senha.

## Atualizações

O app consulta, de tempos em tempos, um arquivo público no GitHub para saber se há versão
nova. Essa consulta não leva identificador seu; o GitHub vê o endereço de internet de onde
ela veio, como em qualquer acesso. Cada produto lê só o próprio arquivo e ignora o do outro.

## Limites que é melhor dizer

- O NoFap dificulta, não tranca. Quem tem a senha de administrador desfaz tudo, e ele não
  alcança o celular, outra rede, uma VPN ou o Tor.
- O tempo de espera para desligar fica no banco de dados do próprio app. Quem apaga ou edita
  esse banco acaba com a espera.
- O banco de dados não é criptografado. Quem tem acesso à sua conta neste computador pode
  ver que o NoFap está instalado e ler a sua frase e os seus planos.
- A guarda e a verificação por IA olham só a janela que está na frente. Uma página aberta
  numa janela de trás só é barrada pelas camadas do sistema.
- Uma VPN traz o próprio DNS, e um navegador em que você escolheu um DNS próprio resolve por
  fora do sistema (a camada 4 fecha isso onde ela existe). No Safari com a Retransmissão
  Privada do iCloud, a lista do sistema e o DNS filtrado podem ser ignorados.

## O que ainda falta confirmar

Este texto descreve o projeto (`docs/nofap/DESIGN.md`) e o código que está no repositório.
Três pontos dependem de verificação fora do código, e estão na lista de
[REAL-MACHINE-CHECKLIST.md](REAL-MACHINE-CHECKLIST.md):

- **Termos dos provedores de IA** (item 12). Ninguém leu ainda o que a Anthropic, a OpenAI e
  a xAI dizem hoje sobre receber capturas que podem ser sexualmente explícitas, por quanto
  tempo as guardam e o que os sistemas de segurança delas fazem. A verificação por IA não
  deve ser oferecida a usuários antes disso, e o texto de consentimento deve ser ajustado ao
  que os termos disserem.
- **Termos da Cloudflare for Families** (item 4), a reler antes de publicar.
- **O arquivo de log depois de um dia de uso** (item 19): a promessa de "nenhuma linha por
  verificação" é testada no código; falta conferir o arquivo real de cada app.
