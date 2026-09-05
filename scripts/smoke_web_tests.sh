#!/usr/bin/env bash
set -euo pipefail

# Smoke test for Alloy Web Tests suite
# Verifies that Caddy serves files properly and that Alloy can render test pages.

PORT="${WEB_TESTS_PORT:-8080}"
BASE_URL="http://localhost:${PORT}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "==> Verificando se o servidor de testes Caddy está ativo em ${BASE_URL}..."

if ! curl -s -f -o /dev/null "${BASE_URL}/"; then
    echo "Servidor não está respondendo em ${BASE_URL}."
    echo "Tentando iniciar via docker compose..."
    docker compose -f web-tests/docker-compose.yml up -d
    
    # Aguardar até 10 segundos
    for i in {1..10}; do
        if curl -s -f -o /dev/null "${BASE_URL}/"; then
            echo "Servidor Caddy iniciado com sucesso!"
            break
        fi
        sleep 1
    done
fi

# 1. Testar se os arquivos essenciais retornam HTTP 200
ENDPOINTS=(
    "/"
    "/index.html"
    "/assets/base.css"
    "/assets/test-pattern.png"
    "/html/semantic-tags.html"
    "/html/text-formatting.html"
    "/html/lists-omission.html"
    "/html/entities-syntax.html"
    "/html/graceful-degradation.html"
    "/css/box-model/margin-collapse.html"
    "/css/box-model/padding-border.html"
    "/css/box-model/box-sizing.html"
    "/css/flexbox/direction-justify.html"
    "/css/flexbox/align-items-self.html"
    "/css/flexbox/grow-shrink-basis.html"
    "/css/flexbox/wrap.html"
    "/css/inline-text/white-space.html"
    "/css/inline-text/text-align.html"
    "/css/selectors/combinators.html"
    "/css/selectors/pseudo-classes.html"
    "/css/selectors/cascade.html"
    "/css/media-queries/min-max-width.html"
    "/css/graceful-degradation.html"
    "/showcase/card-component.html"
    "/showcase/article-page.html"
    "/navigation/page-a.html"
    "/navigation/page-b.html"
    "/navigation/link-resolution.html"
    "/navigation/error-404.html"
)

echo "==> Verificando integridade dos endpoints HTTP..."
for path in "${ENDPOINTS[@]}"; do
    url="${BASE_URL}${path}"
    status=$(curl -s -o /dev/null -w "%{http_code}" "${url}")
    if [ "$status" -ne 200 ]; then
        echo "FALHA: ${url} retornou status ${status} (esperado 200)"
        exit 1
    fi
    echo "  [OK 200] ${path}"
done

echo "==> Testando renderização headless com o binário alloy..."
mkdir -p target/web-tests-smoke
cargo run -p alloy -- render web-tests/showcase/article-page.html -o target/web-tests-smoke/article.png
if [ -f target/web-tests-smoke/article.png ] && [ -s target/web-tests-smoke/article.png ]; then
    echo "  [OK RENDER] web-tests/showcase/article-page.html -> target/web-tests-smoke/article.png"
else
    echo "FALHA: Renderização não produziu imagem PNG válida."
    exit 1
fi

echo "==> Todos os testes da suíte web passaram com sucesso!"
