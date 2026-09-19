#!/usr/bin/env bash
# Publica a página do ubiqX (site/) na VPS, num container Caddy próprio.
#
# Roda NA VPS, como root (pelo workflow deploy-site ou por infra/ops/ssh-run.sh).
# Espera que os arquivos já tenham sido enviados para /opt/ubiqx/site (www/ e Caddyfile).
#
# É um container avulso na rede do Dokploy; o Traefik que já roda ali o descobre pelas
# labels e cuida do TLS. Não toca em nenhuma outra stack da máquina.
# Para remover: docker rm -f ubiqx-web
set -euo pipefail

HOST="${UBIQX_HOST:-ubi.mvk1.cloud}"
DIR=/opt/ubiqx/site

[ -f "$DIR/Caddyfile" ] || { echo "faltam os arquivos em $DIR (rode o upload antes)" >&2; exit 2; }
[ -f "$DIR/www/index.html" ] || { echo "faltam os arquivos em $DIR/www" >&2; exit 2; }

TRAEFIK="$(docker ps --filter ancestor=traefik --format '{{.Names}}' | head -1)"
[ -n "$TRAEFIK" ] || TRAEFIK="$(docker ps --filter name=traefik --format '{{.Names}}' | head -1)"

# Entrypoint e certresolver saem dos argumentos do próprio Traefik. Os nomes mudam de
# instalação para instalação, e um nome que não existe não dá erro: o roteador simplesmente
# não é criado, o pedido cai em outro roteador qualquer e o navegador recebe o certificado
# padrão do Traefik — que foi exatamente o que aconteceu na primeira tentativa.
ARGS="$(docker inspect "${TRAEFIK:-traefik}" --format '{{range .Args}}{{println .}}{{end}}' 2>/dev/null || true)"
ep_named() { printf '%s\n' "$ARGS" | grep -iE "^--entry[Pp]oints\.[A-Za-z0-9_-]+\.address=[^=]*:$1([^0-9]|$)" | head -1 | cut -d. -f2; }
EPS="$(printf '%s\n' "$ARGS" | grep -iE '^--entry[Pp]oints\.[A-Za-z0-9_-]+\.address=' | cut -d. -f2 | sort -u | tr '\n' ' ')"
RESOLVERS="$(printf '%s\n' "$ARGS" | grep -oiE 'certificatesresolvers\.[A-Za-z0-9_-]+\.acme' | cut -d. -f2 | sort -u | tr '\n' ' ')"

ENTRY_HTTPS="${UBIQX_ENTRYPOINT:-$(ep_named 443)}"
ENTRY_HTTP="${UBIQX_ENTRYPOINT_HTTP:-$(ep_named 80)}"
RESOLVER="${UBIQX_CERTRESOLVER:-$(printf '%s' "$RESOLVERS" | awk '{print $1}')}"

# Nem todo Traefik é configurado por linha de comando: muitos leem um traefik.yml estático.
# Nesse caso os argumentos não dizem nada e é preciso olhar o arquivo dentro do container.
YML=""
if [ -z "$EPS" ] && [ -n "$TRAEFIK" ]; then
  YML="$(docker exec "$TRAEFIK" sh -c 'cat /etc/traefik/traefik.yml /etc/traefik/traefik.yaml /traefik.yml /traefik.yaml 2>/dev/null' 2>/dev/null || true)"
  YML_EPS="$(printf '%s\n' "$YML" | awk '
    /^[[:space:]]*[eE]ntry[pP]oints:/ { inep=1; next }
    inep && /^[^[:space:]#]/          { inep=0 }
    inep && /^[[:space:]]{1,4}[A-Za-z0-9_-]+:[[:space:]]*$/ { name=$1; sub(/:.*/, "", name) }
    inep && /address:/ && name != ""  { port=($0 ~ /:443/) ? "443" : (($0 ~ /:80([^0-9]|$)/) ? "80" : ""); if (port != "") print port "=" name }
  ')"
  EPS="$(printf '%s\n' "$YML_EPS" | cut -d= -f2 | sort -u | tr '\n' ' ')"
  [ -n "$ENTRY_HTTPS" ] || ENTRY_HTTPS="$(printf '%s\n' "$YML_EPS" | grep '^443=' | head -1 | cut -d= -f2)"
  [ -n "$ENTRY_HTTP" ]  || ENTRY_HTTP="$(printf '%s\n' "$YML_EPS" | grep '^80='  | head -1 | cut -d= -f2)"
  [ -n "$RESOLVER" ] || RESOLVER="$(printf '%s\n' "$YML" | awk '
    /^[[:space:]]*certificates[rR]esolvers:/ { inr=1; next }
    inr && /^[^[:space:]#]/ { inr=0 }
    inr && /^[[:space:]]{1,4}[A-Za-z0-9_-]+:[[:space:]]*$/ { name=$1; sub(/:.*/, "", name); print name; exit }
  ')"
fi

# Último recurso: os nomes que a maioria das instalações usa. Se estiverem errados, o
# diagnóstico impresso abaixo diz quais são os certos, e UBIQX_ENTRYPOINT decide.
ENTRY_HTTPS="${ENTRY_HTTPS:-websecure}"
ENTRY_HTTP="${ENTRY_HTTP:-web}"

# A rede: para o Traefik alcançar a página, os dois precisam se enxergar. Há dois arranjos,
# e o nome da rede muda de instalação para instalação, então perguntamos ao Traefik:
#
#   * Traefik numa rede Docker (dokploy-network, <projeto>_default…) — entramos nela também;
#   * Traefik em modo host (o caso desta VPS) — ele enxerga qualquer container pelo IP, então
#     a bridge padrão basta e a label traefik.docker.network não deve ser usada.
MODE="$(docker inspect "${TRAEFIK:-traefik}" --format '{{.HostConfig.NetworkMode}}' 2>/dev/null || true)"
NETS=""
if [ -n "$TRAEFIK" ]; then
  NETS="$(docker inspect "$TRAEFIK" \
    --format '{{range $k, $v := .NetworkSettings.Networks}}{{$k}}{{"\n"}}{{end}}' 2>/dev/null \
    | grep -vE '^(bridge|host|none|)$' || true)"
fi
if [ -n "${UBIQX_NETWORK:-}" ]; then
  NET="$UBIQX_NETWORK"
else
  NET="$(printf '%s\n' "$NETS" | grep -m1 dokploy || true)"
  [ -n "$NET" ] || NET="$(printf '%s\n' "$NETS" | grep -m1 . || true)"
fi

PROVIDERS="$(printf '%s\n' "$ARGS" | grep -oiE '\-\-providers\.[a-z]+' | sort -u | tr '\n' ' ' || true)"
echo "[ubiqx] host=$HOST"
echo "[ubiqx] traefik=${TRAEFIK:-não encontrado} modo-de-rede=${MODE:-?} providers=${PROVIDERS:-?}"
echo "[ubiqx] entrypoints=${EPS:-nenhum nos argumentos} | 443=${ENTRY_HTTPS:-?} 80=${ENTRY_HTTP:-?}"
echo "[ubiqx] certresolvers=${RESOLVERS:-nenhum nos argumentos} | escolhido=${RESOLVER:-nenhum}"
echo "[ubiqx] redes do traefik: $(printf '%s' "$NETS" | tr '\n' ' ')| escolhida: ${NET:-bridge padrão}"

# Quem mais disputa este hostname: sem isto, um roteador alheio que casa com tudo rouba o
# domínio em silêncio e o navegador vai parar noutro site.
echo "[ubiqx] roteadores já declarados por outros containers:"
for c in $(docker ps --filter label=traefik.enable=true --format '{{.Names}}' | grep -v '^ubiqx-web$' || true); do
  docker inspect "$c" --format '{{json .Config.Labels}}' 2>/dev/null \
    | tr ',' '\n' | grep -iE 'traefik\.http\.routers\.[^"]*\.(rule|entrypoints|tls)' \
    | sed "s/^/    $c  /" | head -8 || true
done

NET_ARGS=()
if [ -n "$NET" ]; then
  docker network inspect "$NET" >/dev/null 2>&1 || {
    echo "a rede '$NET' não existe nesta VPS." >&2
    echo "redes disponíveis:" >&2
    docker network ls --format '  {{.Name}}' >&2
    echo "escolha uma com UBIQX_NETWORK=<nome> (precisa ser uma em que o Traefik esteja)." >&2
    exit 2
  }
  # Com várias redes em jogo, a label diz ao Traefik de qual IP do container ele deve falar.
  NET_ARGS=(--network "$NET" -l traefik.docker.network="$NET")
fi

# Dois roteadores: o de TLS e um de HTTP puro. O segundo custa nada e garante que a página
# fique acessível por http:// mesmo que o certificado ainda não tenha saído.
LABELS=(
  -l traefik.enable=true
  -l traefik.http.services.ubiqx.loadbalancer.server.port=80
  -l "traefik.http.routers.ubiqx.rule=Host(\`$HOST\`)"
  -l "traefik.http.routers.ubiqx.entrypoints=$ENTRY_HTTPS"
  -l traefik.http.routers.ubiqx.service=ubiqx
  -l traefik.http.routers.ubiqx.tls=true
  -l "traefik.http.routers.ubiqx-http.rule=Host(\`$HOST\`)"
  -l "traefik.http.routers.ubiqx-http.entrypoints=$ENTRY_HTTP"
  -l traefik.http.routers.ubiqx-http.service=ubiqx
)
# Sem um certresolver de verdade a label só faria o Traefik servir o certificado padrão dele.
if [ -n "$RESOLVER" ]; then
  LABELS+=(-l "traefik.http.routers.ubiqx.tls.certresolver=$RESOLVER")
else
  echo "[ubiqx] atenção: nenhum certresolver ACME encontrado neste Traefik; o TLS vai sair com o certificado padrão dele." >&2
fi

docker rm -f ubiqx-web >/dev/null 2>&1 || true
docker run -d --name ubiqx-web --restart unless-stopped \
  ${NET_ARGS[@]+"${NET_ARGS[@]}"} \
  -v "$DIR/Caddyfile:/etc/caddy/Caddyfile:ro" \
  -v "$DIR/www:/srv/www:ro" \
  "${LABELS[@]}" \
  caddy:2-alpine >/dev/null

sleep 3
docker ps --filter name=ubiqx-web --format '[ubiqx] {{.Names}} {{.Status}}'
# O teste é feito de onde o Traefik está: na rede dele, ou no próprio host quando ele é host.
IP="$(docker inspect ubiqx-web --format '{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}' | awk '{print $1}')"
if [ -n "$NET" ]; then
  code="$(docker run --rm --network "$NET" curlimages/curl:latest -s -o /dev/null -w '%{http_code}' http://ubiqx-web/ || true)"
  echo "[ubiqx] a página responde $code para quem está em $NET"
else
  code="$(docker run --rm --network host curlimages/curl:latest -s -o /dev/null -w '%{http_code}' "http://$IP/" || true)"
  echo "[ubiqx] a página responde $code em http://$IP/, que é como o Traefik em modo host a alcança"
fi
[ "$code" = "200" ] || echo "[ubiqx] atenção: esperava 200 aqui; o Traefik provavelmente também não vai alcançar." >&2

# O DNS aponta mesmo para esta máquina? Um registro A para outro servidor explica de uma vez
# o certificado de outro domínio, o site alheio na tela e o silêncio do Traefik daqui: o
# pedido do navegador simplesmente nunca chega nesta VPS.
RESOLVED="$(getent ahostsv4 "$HOST" 2>/dev/null | awk '{print $1; exit}' || true)"
MYIP="$(ip route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src"){print $(i+1); exit}}' || true)"
[ -n "$MYIP" ] || MYIP="$(hostname -I 2>/dev/null | awk '{print $1}' || true)"
if [ -z "$RESOLVED" ]; then
  echo "[ubiqx] atenção: $HOST ainda não resolve para IP nenhum (registro A ausente ou não propagado)." >&2
elif [ -z "$MYIP" ]; then
  echo "[ubiqx] dns: $HOST resolve para $RESOLVED; não consegui descobrir o IP desta VPS para comparar."
elif [ "$RESOLVED" != "$MYIP" ]; then
  echo "[ubiqx] ATENÇÃO: $HOST resolve para $RESOLVED, mas esta VPS é $MYIP." >&2
  echo "[ubiqx] enquanto o registro A não apontar para cá, o navegador continua caindo noutro servidor." >&2
else
  echo "[ubiqx] dns: $HOST resolve para esta VPS."
fi

# Quem responde por este host nesta máquina? `--resolve` pula o DNS e entrega o pedido ao
# Traefik local, com o cabeçalho Host certo. Se vier a página do ubiqX aqui e outra coisa no
# navegador, o problema não é roteamento: é o pedido chegando noutro servidor.
probe() {
  docker run --rm --network host curlimages/curl:latest -sk --max-time 10 \
    --resolve "$HOST:$1:127.0.0.1" "$2://$HOST/" 2>/dev/null || true
}
title_of() { printf '%s' "$1" | tr -d '\r\n' | sed -n 's/.*<title>\([^<]*\)<\/title>.*/\1/p' | cut -c1-70; }
echo "[ubiqx] perguntando ao traefik desta máquina, sem passar pelo dns:"
echo "[ubiqx]   https -> título: $(title_of "$(probe 443 https)")"
echo "[ubiqx]   http  -> título: $(title_of "$(probe 80 http)")"

echo "[ubiqx] flags do provider docker deste traefik:"
printf '%s\n' "$ARGS" | grep -iE '^--providers\.' | sed 's/^/    /' || echo "    (nenhuma na linha de comando)"

# O veredito de verdade é do Traefik: ele conta se pegou as labels e o que o ACME respondeu.
if [ -n "$TRAEFIK" ]; then
  sleep 8
  TLOG="$(docker logs --tail 400 "$TRAEFIK" 2>&1 | grep -iE "ubiqx|${HOST//./\\.}|acme|certificate" | tail -15 || true)"
  if [ -n "$TLOG" ]; then
    echo "[ubiqx] o que o traefik diz sobre este host e sobre certificados:"
    printf '%s\n' "$TLOG" | sed 's/^/    /'
  else
    echo "[ubiqx] o traefik não registrou nada sobre este host — sinal de que nenhum pedido para $HOST chegou aqui."
  fi
fi
echo "[ubiqx] pronto. Fora, depende do DNS: $HOST deve apontar para o IP desta VPS (registro A)."
