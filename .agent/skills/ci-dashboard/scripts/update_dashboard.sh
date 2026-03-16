#!/bin/bash
set -e

# Resolve Project Root dynamically
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../../" && pwd)"
cd "$PROJECT_ROOT"

# Function to update status
update_status() {
    local status=$1
    local progress=$2
    local message=$3
    local timestamp=$(date +%s)
    echo "window.CI_DASHBOARD_STATUS = {\"status\": \"$status\", \"progress\": $progress, \"message\": \"$message\", \"test_label\": \"\", \"eta_label\": \"\", \"timestamp\": $timestamp};" > ci-dashboard/tmp/status.js
}

# Trap errors to set error status
trap 'update_status "error" 100 "Deu ruim! ❌ Erro inesperado durante a atualização do dashboard."' ERR

echo "🚀 Atualizando dados do Test Dashboard..."

# 0. Garantir diretório temporário
mkdir -p ci-dashboard/tmp
update_status "running" 0 "Iniciando a atualização em background... Preparando motores 🚀"

# 1. Secret/Vault Handling (Local vs CI)
# Prefer Environment Variables (CI), fallback to .vault_local (Local)
if [ -f ".vault_local/google_api_key.txt" ] && [ -z "$GOOGLE_API_KEY" ]; then
    export GOOGLE_API_KEY=$(cat .vault_local/google_api_key.txt)
    echo "🔑 Usando API Key de .vault_local"
fi

update_status "running" 10 "Executando bateria completa de testes em paralelo... ⏳"

# 1. Gerar Testes e Cobertura (Coordenado pelo test_runner.py)
echo "🧪 Running Tests & Coverage (Fullstack)..."
python3 ".agent/skills/ci-dashboard/scripts/test_runner.py" --layer fullstack || echo "⚠️ Alguns testes falharam."

update_status "running" 60 "Testes concluídos! 🔍 Analisando Status do GitHub, Riscos e Ambientes..."

# 2. Collect Statuses (CI, Env, Risks) -- paralelo por serem independentes
echo "🔍 Checking System Status (paralelo)..."
python3 ".agent/skills/ci-dashboard/scripts/github_status.py" --output ci-dashboard/tmp/ci.json &
PID_GH=$!
python3 ".agent/skills/ci-dashboard/scripts/env_status.py" --output ci-dashboard/tmp/environments.json &
PID_ENV=$!
python3 ".agent/skills/ci-dashboard/scripts/risk_analyzer.py" . --output ci-dashboard/tmp/risks.json &
PID_RISK=$!
# Aguardar todos terminarem (ignorar falhas individualmente)
wait $PID_GH  || true
wait $PID_ENV || true
wait $PID_RISK || true

update_status "running" 80 "Consolidando métricas e verificando a cobertura de código... 📊"

# 3. Collect/Merge Coverage Summaries
echo "📊 Merging Coverage Data..."
python3 ".agent/skills/ci-dashboard/scripts/coverage_collector.py" --output ci-dashboard/tmp/coverage.json

update_status "running" 90 "Quase lá! ✨ Renderizando a nova interface interativa do Dashboard..."

# 4. Generate Dashboard
echo "🚀 Generating Dashboard..."
python3 ".agent/skills/ci-dashboard/scripts/dashboard_generator.py" \
    --config "ci-dashboard/config.json" \
    --test-results "ci-dashboard/tmp/test_results.json" \
    --coverage "ci-dashboard/tmp/coverage.json" \
    --backend-coverage "ci-dashboard/tmp/backend_coverage.json" \
    --frontend-coverage "ci-dashboard/tmp/frontend_coverage.json" \
    --ci-status "ci-dashboard/tmp/ci.json" \
    --risks "ci-dashboard/tmp/risks.json" \
    --history "ci-dashboard/tmp/history.json" \
    --env-status "ci-dashboard/tmp/environments.json" \
    --frontend-results "ci-dashboard/tmp/frontend_results.json" \
    --output "ci-dashboard/tmp/dashboard.html"

update_status "idle" 100 "Tudo pronto! Dashboard gerado com sucesso."

echo "✅ Dashboard Updated!"
echo "📍 HTML: ci-dashboard/tmp/dashboard.html"
echo "📍 Data: ci-dashboard/tmp/data.js (Ignorado no Git)"

