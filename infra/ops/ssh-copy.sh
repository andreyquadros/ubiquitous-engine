#!/usr/bin/env bash
# Copia um diretório local para a VPS, por SSH, com a mesma credencial do ssh-run.sh.
#
#   VPS_HOST=203.0.113.10 VPS_SSH_KEY_FILE=~/.ssh/ubiqx-deploy \
#     infra/ops/ssh-copy.sh infra/site/.stage /opt/ubiqx/site
#
# O conteúdo do diretório de origem substitui o de destino (o destino é criado e limpo).
# Usa tar por cima do ssh, então não precisa de rsync nem de scp na VPS.
set -euo pipefail

: "${VPS_HOST:?defina VPS_HOST (IP ou hostname da VPS)}"
SRC="${1:?informe o diretório local}"
DEST="${2:?informe o diretório remoto}"
VPS_USER="${VPS_USER:-root}"
VPS_PORT="${VPS_PORT:-22}"

[ -d "$SRC" ] || { echo "$SRC não é um diretório" >&2; exit 2; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KNOWN_HOSTS="${VPS_KNOWN_HOSTS:-$HERE/known_hosts}"

SSH_OPTS=(
  -p "$VPS_PORT"
  -o UserKnownHostsFile="$KNOWN_HOSTS"
  -o ServerAliveInterval=30
  -o ServerAliveCountMax=10
  -o ConnectTimeout=30
  -o LogLevel=ERROR
)
if [ -s "$KNOWN_HOSTS" ] && grep -q "$VPS_HOST" "$KNOWN_HOSTS"; then
  SSH_OPTS+=(-o StrictHostKeyChecking=yes)
else
  SSH_OPTS+=(-o StrictHostKeyChecking=accept-new)
fi

# O destino é recriado do zero, para não deixar arquivos de uma versão anterior para trás.
REMOTE="set -e; rm -rf '$DEST'; mkdir -p '$DEST'; tar -xzf - -C '$DEST'"

if [ -n "${VPS_SSH_KEY_FILE:-}" ]; then
  SSH_OPTS+=(-i "$VPS_SSH_KEY_FILE" -o IdentitiesOnly=yes -o BatchMode=yes)
  tar -czf - -C "$SRC" . | ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" "$REMOTE"
elif [ -n "${VPS_PASSWORD:-}" ]; then
  command -v sshpass >/dev/null || { echo "instale sshpass para autenticar por senha" >&2; exit 2; }
  SSH_OPTS+=(-o PreferredAuthentications=password -o PubkeyAuthentication=no)
  tar -czf - -C "$SRC" . | SSHPASS="$VPS_PASSWORD" sshpass -e ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" "$REMOTE"
else
  echo "sem credencial: defina VPS_SSH_KEY_FILE (chave privada) ou VPS_PASSWORD" >&2
  exit 2
fi
echo "[ssh-copy] $SRC -> $VPS_USER@$VPS_HOST:$DEST"
