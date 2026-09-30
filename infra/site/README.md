# Publicar a página do ubiqX numa VPS

A landing page (`site/`) tem dois destinos possíveis, e eles não competem:

* **GitHub Pages** — `.github/workflows/site.yml`, automático a cada push em `main`. É o
  destino permanente enquanto `ubiqx.ai` não existe.
* **VPS própria** — `.github/workflows/deploy-site.yml`, este diretório. Serve a mesma
  página num domínio de verdade (hoje `ubi.mvk1.cloud`), num container Caddy atrás do
  Traefik que o Dokploy já roda na máquina.

## O que vai para a VPS

O workflow compila `site/` com `SITE_BASE=/`, monta um pacote com

```
www/         a página compilada (site/dist)
Caddyfile    servidor estático, escutando :80 dentro do container
deploy.sh    sobe o container e registra as labels do Traefik
```

e o copia para `/opt/ubiqx/site` na VPS (`infra/ops/ssh-copy.sh`), executando em seguida
`deploy.sh` por SSH (`infra/ops/ssh-run.sh`). O container chama-se `ubiqx-web`; nada mais
na máquina é tocado. Para remover: `docker rm -f ubiqx-web`.

## Configuração, uma vez

1. **Chave SSH dedicada**, sem passphrase — o runner não tem como digitar uma:

   ```sh
   ssh-keygen -t ed25519 -f ~/.ssh/ubiqx-deploy -N "" -C "ubiqx-deploy"
   ssh-copy-id -i ~/.ssh/ubiqx-deploy.pub root@<IP-DA-VPS>
   ```

2. **Segredos do repositório** (Settings → Secrets and variables → Actions → *repository*,
   não *environment*):

   | Segredo | Valor |
   |---|---|
   | `VPS_HOST` | IP ou hostname da VPS |
   | `VPS_SSH_KEY` | conteúdo de `~/.ssh/ubiqx-deploy`, com as linhas `BEGIN`/`END` |
   | `VPS_USER` | opcional; `root` quando não definido |

   `VPS_PASSWORD` funciona no lugar da chave, mas a chave é melhor: é revogável sozinha.

3. **DNS**: registro **A** de `ubi.mvk1.cloud` apontando para o IP da VPS. Sem ele o
   container sobe e responde na rede interna, mas o navegador não chega nele — e o
   Let's Encrypt não emite o certificado.

Sem os segredos o workflow avisa e termina verde (um push não deve ficar vermelho por
uma publicação que ninguém pediu); num disparo manual pelo botão **Run workflow**, ele
falha, porque aí o pedido foi explícito.

## Rodar à mão

De qualquer máquina com `ssh`, a partir da raiz do repositório:

```sh
SITE_BASE=/ SITE_URL=https://ubi.mvk1.cloud pnpm -C site build
rm -rf infra/site/.stage && mkdir -p infra/site/.stage
cp -R site/dist infra/site/.stage/www
cp infra/site/Caddyfile infra/site/deploy.sh infra/site/.stage/

export VPS_HOST=<IP> VPS_SSH_KEY_FILE=~/.ssh/ubiqx-deploy
infra/ops/ssh-copy.sh infra/site/.stage /opt/ubiqx/site
OPS_COMMAND='export UBIQX_HOST=ubi.mvk1.cloud; bash /opt/ubiqx/site/deploy.sh' infra/ops/ssh-run.sh
```

## Como o script acha a rede e o certificado

Para o Traefik alcançar a página, os dois precisam se enxergar — senão o hostname responde
404. Há dois arranjos, e `deploy.sh` descobre qual é perguntando ao próprio Traefik em vez
de chutar um nome:

* **Traefik numa rede Docker** (`dokploy-network`, `<projeto>_default`…) — o container entra
  na mesma rede e recebe a label `traefik.docker.network`, que diz de qual IP falar.
* **Traefik em modo host** (o caso da VPS de hoje) — ele enxerga qualquer container pelo IP,
  então a bridge padrão basta e a label não deve existir; ela faria o Traefik procurar um IP
  numa rede que ele não tem.

O certresolver sai do mesmo lugar, dos argumentos do Traefik, caindo em `letsencrypt` quando
não dá para saber.

Se a escolha sair errada, `UBIQX_NETWORK=<nome>` decide. Cada execução imprime o Traefik
encontrado, o modo de rede dele, seus *providers* e a rede escolhida; um nome inexistente
lista as redes da máquina antes de parar. No fim, o script confere a página **de onde o
Traefik está** — de dentro da rede dele, ou do próprio host quando ele é host — e avisa se
não vier `200`, que é o sinal de que o Traefik também não vai alcançar.
