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

# O certresolver vem do próprio Traefik que já roda aqui; "letsencrypt" é o padrão do Dokploy.
RESOLVER="$(docker inspect "${TRAEFIK:-traefik}" --format '{{json .Args}}' 2>/dev/null \
  | grep -o 'certificatesresolvers\.[A-Za-z0-9_-]*\.acme' | head -1 | cut -d. -f2 || true)"
RESOLVER="${RESOLVER:-letsencrypt}"

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

PROVIDERS="$(docker inspect "${TRAEFIK:-traefik}" --format '{{json .Args}}' 2>/dev/null \
  | grep -oE '\-\-providers\.[a-z]+' | sort -u | tr '\n' ' ' || true)"
echo "[ubiqx] host=$HOST certresolver=$RESOLVER"
echo "[ubiqx] traefik=${TRAEFIK:-não encontrado} modo-de-rede=${MODE:-?} providers=${PROVIDERS:-?}"
echo "[ubiqx] redes do traefik: $(printf '%s' "$NETS" | tr '\n' ' ')| escolhida: ${NET:-bridge padrão}"

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

docker rm -f ubiqx-web >/dev/null 2>&1 || true
docker run -d --name ubiqx-web --restart unless-stopped \
  ${NET_ARGS[@]+"${NET_ARGS[@]}"} \
  -v "$DIR/Caddyfile:/etc/caddy/Caddyfile:ro" \
  -v "$DIR/www:/srv/www:ro" \
  -l traefik.enable=true \
  -l "traefik.http.routers.ubiqx.rule=Host(\`$HOST\`)" \
  -l traefik.http.routers.ubiqx.entrypoints=websecure \
  -l "traefik.http.routers.ubiqx.tls.certresolver=$RESOLVER" \
  -l traefik.http.services.ubiqx.loadbalancer.server.port=80 \
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
echo "[ubiqx] pronto. Fora, depende do DNS: $HOST deve apontar para o IP desta VPS (registro A)."
