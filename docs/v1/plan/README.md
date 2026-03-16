# PKS - Milestone 6: Maturação e Operação 🚀

Este documento explica como configurar o **Milestone 6 (M6)** do Prometheus Knowledge System (PKS), focado em estabilidade operacional, diagnósticos CLI e Disaster Recovery (DR).

## 📋 Pré-requisitos

### 1. Ollama (Motor de Embeddings)
O PKS utiliza o Ollama localmente para gerar os vetores.

- **Instalação (macOS)**: `brew install --cask ollama`
- **Modelo**: `ollama pull nomic-embed-text`

### 2. GitHub CLI (Para Disaster Recovery do Índice)
O PKS M6 utiliza um repositório Git separado para fazer o backup dos seus vetores e snapshots de busca.

- **Instalação**: 
  ```bash
  brew install gh
  ```
- **Autenticação (Importante)**: 
  Você precisa autenticar o CLI para que o PKS possa gerenciar o repositório de snapshots:
  ```bash
  gh auth login
  ```
- **Criar o repositório de backup**:
  ```bash
  gh repo create pks-snapshots --private
  ```

---

## ⚙️ Configuração (.env)

Configure as variáveis abaixo para habilitar o backup remoto e observabilidade:

```bash
# URL do repositório de backup criado via gh
PKS_VECTOR_REMOTE_URL="https://github.com/SEU_USUARIO/pks-snapshots.git"

# Diretório onde os backups serão armazenados antes do push
PKS_SNAPSHOTS_DIR="$HOME/.pks/snapshots"

# Porta do Daemon (padrão: 3030)
PKS_PORT=3030
```

---

## 🚀 Comandos de Diagnóstico (CLI)

O M6 introduziu ferramentas para você monitorar a saúde do sistema sem precisar olhar os logs.

### 1. Verificar Estado do Daemon
Mostra uptime, uso de memória e saúde das filas de processamento:
```bash
./pks status
```

### 2. Validar Drift de Dados
Compara seus arquivos `.md` atuais com o que está indexado no PKS para garantir que nada ficou sem processar:
```bash
./pks validate ./seu_vault
```

---

## 🛡️ Graceful Shutdown & DR

O PKS agora suporta encerramento gracioso via `SIGTERM` ou `SIGINT` (Ctrl+C).

- **O que acontece ao fechar**: O Daemon captura o sinal, interrompe novas tarefas e **salva um snapshot automático** de todo o índice no diretório de snapshots (e faz o push para o GitHub se configurado).
- **Recuperação**: No próximo boot, o PKS lerá esses snapshots, acelerando drasticamente o tempo de inicialização em vaults grandes.

---

## 🔍 Monitoramento
Logs JSON estruturados ficam em: `~/.pks/logs/pks.log`.
