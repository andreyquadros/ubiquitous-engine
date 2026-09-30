#!/usr/bin/env bash
# Executa um script bash na VPS, como root, via SSH.
#
# Usado pelo workflow .github/workflows/deploy-site.yml e utilizável de qualquer
# máquina com ssh:
#
#   VPS_HOST=203.0.113.10 VPS_SSH_KEY_FILE=~/.ssh/ubiqx-deploy infra/ops/ssh-run.sh 'uptime'
#   VPS_HOST=203.0.113.10 VPS_PASSWORD='...' infra/ops/ssh-run.sh "$(cat meu-script.sh)"
#
# Credencial: VPS_SSH_KEY_FILE (chave privada) ou VPS_PASSWORD (precisa de sshpass).
# O script é enviado em base64 e executado a partir de um arquivo temporário, então
# comandos que leem stdin (apt, docker exec -i, etc.) não "comem" as linhas seguintes.
set -euo pipefail

: "${VPS_HOST:?defina VPS_HOST (IP ou hostname da VPS)}"
VPS_USER="${VPS_USER:-root}"
VPS_PORT="${VPS_PORT:-22}"
SCRIPT="${1:-${OPS_COMMAND:-}}"
if [ -z "$SCRIPT" ]; then
  echo "informe o script como argumento ou em OPS_COMMAND" >&2
  exit 2
fi

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
# Com o known_hosts do repositório preenchido, a chave do servidor é verificada de
# verdade; vazio, aceita na primeira conexão (TOFU) e mostra a chave para registrar.
if [ -s "$KNOWN_HOSTS" ] && grep -q "$VPS_HOST" "$KNOWN_HOSTS"; then
  SSH_OPTS+=(-o StrictHostKeyChecking=yes)
else
  SSH_OPTS+=(-o StrictHostKeyChecking=accept-new)
  echo "[ssh-run] chave do host ainda não registrada em $KNOWN_HOSTS; aceitando na primeira conexão." >&2
  echo "[ssh-run] para fixar, adicione a linha abaixo em infra/ops/known_hosts:" >&2
  ssh-keyscan -p "$VPS_PORT" -t ed25519 "$VPS_HOST" 2>/dev/null >&2 || true
fi

ENCODED="$(printf '%s' "$SCRIPT" | base64 | tr -d '\n')"
REMOTE="set -o pipefail; f=\$(mktemp /tmp/ops.XXXXXX.sh); printf '%s' '$ENCODED' | base64 -d > \"\$f\"; bash \"\$f\"; rc=\$?; rm -f \"\$f\"; exit \$rc"

if [ -n "${VPS_SSH_KEY_FILE:-}" ]; then
  SSH_OPTS+=(-i "$VPS_SSH_KEY_FILE" -o IdentitiesOnly=yes -o BatchMode=yes)
  exec ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" "$REMOTE"
elif [ -n "${VPS_PASSWORD:-}" ]; then
  command -v sshpass >/dev/null || { echo "instale sshpass para autenticar por senha" >&2; exit 2; }
  SSH_OPTS+=(-o PreferredAuthentications=password -o PubkeyAuthentication=no)
  SSHPASS="$VPS_PASSWORD" exec sshpass -e ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" "$REMOTE"
else
  echo "sem credencial: defina VPS_SSH_KEY_FILE (chave privada) ou VPS_PASSWORD" >&2
  exit 2
fi
