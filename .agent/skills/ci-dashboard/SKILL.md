---
name: ci-dashboard
description: Unified testing interface with dashboard, coverage reporting, and risk analysis based on project Constitution.
capabilities:
  - run_tests: Execute tests categorized by Type (Unit, Integration, Sanity, Health Check) and Layer (Backend, Frontend, Fullstack).
  - generate_report: Create a premium HTML dashboard summarizing test results, CI status, and risks.
  - check_risks: Analyze codebase for violations of Constitution rules (file size, absolute paths, language).
---

# Test Dashboard Skill

Esta skill padroniza a interface de testes do projeto, permitindo uma visão clara da saúde do código através de um dashboard intuitivo inspirado no TestSprite, mas integrado com as ferramentas locais.

## Scripts de Runtime

| Script | Uso |
|--------|-----|
| `scripts/test_runner.py` | Roda os testes e gera resultados JSON |
| `scripts/run_frontend_tests.py` | Executa testes de frontend (Node.js) |
| `scripts/coverage_collector.py` | Consolida cobertura e prepara links para HTML reports |
| `scripts/github_status.py` | Busca status do CI via GitHub CLI |
| `scripts/risk_analyzer.py` | Avalia o código contra a Constituição do projeto |
| `scripts/dashboard_generator.py` | Gera o `dashboard.html` final |

## Paralelismo e Performance ⚡

A skill agora está nativamente configurada para rodar **testes backend em paralelo** utilizando o plugin `pytest-xdist`.

- Quando acionada (e o plugin estiver no `.venv`), o script `test_runner.py` injeta `-n auto` para máxima velocidade na execução da suíte toda.
- A barra de notificação/Dashboard reage aos logs processando a tag contínua de pontos e mantendo o painel elegante e em tempo-real (`⚡ Executando suite em paralelo...`).
- **Dependências Requeridas:** Para habilitar esse benefício, o ambiente virtual dos testes backend (`.venv`) deve conter: `pytest-xdist`, `pytest-cov` e `pytest-json-report`.

## Taxonomia

### Tipos
- **Unit**: Lógica isolada.
- **Integration**: Múltiplas unidades integradas.
- **Sanity**: Caminho feliz crítico (gate de deploy).
- **Health Check**: Monitoramento contínuo em ambiente.

### Camadas
- **Backend**: API Python, services.
- **Frontend**: Apps Script, JS, HTML.
- **Fullstack**: Fluxos fim-a-fim.

## Cobertura — Regras e Interpretação 📊

- **O percentual de cobertura exibido é global**, calculado sobre todas as pastas declaradas como `"type": "backend"` em `ci-dashboard/config.json` (hoje: `src/backend`, `src/ops` e `src/tools`).
- **Não altere os `types` do `config.json` de forma unilateral** para manipular o número exibido. Qualquer mudança no escopo de cobertura deve ser uma decisão explicitamente discutida com o usuário.
- **A descoberta de novos testes é automática.** O pytest varre recursivamente todas as pastas declaradas em `test_directories` procurando por `test_*.py`. Não é necessário registrar novos arquivos manualmente.
- **Para o número de cobertura subir**, é preciso escrever testes nas pastas que têm cobertura 0% (`src/ops`, `src/tools`). Não há atalho via config.

## Estrutura de Diretórios (NÃO NEGOCIÁVEL)

A skill deve seguir rigorosamente a estrutura de 3 partes. **NENHUM** arquivo gerado (outputs) deve persistir na raiz da pasta `ci-dashboard`.

1.  **Templates (Jinja2)**:
    *   Local: `.agent/skills/ci-dashboard/templates/`
    *   Arquivos: `dashboard.html`, `coverage_list.html`, `tests_list.html`

2.  **Scripts (Python/Shell)**:
    *   Local: `.agent/skills/ci-dashboard/scripts/`
    *   Scripts: `dashboard_generator.py`, `risk_analyzer.py`, `test_runner.py`

3.  **Runtime Data (Ignorado pelo Git)**:
    *   Local: `ci-dashboard/` (Raiz do projeto)
    *   Estrutura Interna:
        *   `config.json` (Configuração persistente: Links, Nomes)
        *   `tmp/` (Arquivos temporários: JSONs de cobertura, HTML gerado)

    *   Local: `ci-dashboard/tmp/`
    *   Exemplo: `coverage_summary.json`, `test_results.json`, `dashboard.html`

> 🔴 **PROIBIDO**: Salvar `dashboard.html`, `data.js` ou jsons de status na raiz `ci-dashboard/`. Salve SEMPRE em `tmp/`.