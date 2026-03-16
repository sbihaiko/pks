# M14 — Zero-Config Onboarding

> **Status:** PENDENTE
> **Depende de:** M10 (IPC — Unix Socket / Named Pipe), M11 (RepoId — git_common_dir), M12 (Shadow Journal — PKS_SHADOW_JOURNAL config), M13 (Ollama opcional — PKS_EMBEDDING_PROVIDER config)
> **Estimativa:** 5-8 dias de desenvolvimento

---

## Objetivo

Reduzir o tempo de adoção do PKS em um novo projeto de **~30 minutos de configuração manual** para **menos de 30 segundos** com um único comando.

Atualmente o desenvolvedor precisa: identificar o diretório `.git`, criar manualmente a branch `pks-knowledge`, registrar o repositório no daemon via IPC, configurar paths absolutos, e aguardar a indexação inicial. O M14 elimina todas essas etapas com o subcomando `pks init`.

---

## Experiência do Usuário

Fluxo de sucesso esperado no terminal:

```
$ pks init
✓ Git root detectado: /Users/dev/payment-api
✓ Config gerada: .pks/config.toml
✓ Branch criada: pks-knowledge
✓ Daemon registrado: payment-api (RepoId: /Users/dev/payment-api/.git)
✓ Indexação inicial: 142 arquivos, 1.847 chunks (BM25 pronto)
PKS ativo. Buscar: pks search "retry logic"
```

Caso o projeto já esteja inicializado:

```
$ pks init
⚠ PKS já inicializado em /Users/dev/payment-api
  Use --force para sobrescrever a configuração existente.
```

---

## Fluxo Interno do `pks init`

O subcomando executa as seguintes etapas em ordem:

1. **Detectar git root** — executa `git rev-parse --show-toplevel` no diretório corrente. Aborta com mensagem clara se não estiver dentro de um repositório Git.
2. **Criar diretório `.pks/`** — cria `<git_root>/.pks/` se não existir. Não sobrescreve arquivos existentes sem `--force`.
3. **Gerar `.pks/config.toml`** — escreve o template com valores inferidos automaticamente: `name` vem do basename do diretório, `git_common_dir` vem do caminho canônico retornado pelo `git rev-parse --git-common-dir`.
4. **Criar branch `pks-knowledge`** — usa `git2-rs` para criar a branch órfã via API de baixo nível (sem checkout). Se a branch já existir, pula sem erro.
5. **Registrar no daemon via IPC** — envia `PksCommand::RegisterRepo { repo_id, config_path }` pelo socket Unix/Named Pipe (implementado em M10). Se o daemon não estiver rodando, faz auto-spawn com lockfile (implementado em M10).
6. **Disparar indexação inicial** — envia `PksCommand::TriggerIndex { repo_id }`. A indexação ocorre em background no daemon; o cliente aguarda confirmação de enfileiramento (não aguarda conclusão).
7. **Imprimir resumo** — exibe o output formatado com os checkmarks e a contagem de arquivos/chunks retornada pelo daemon.
8. **Executar `pks refresh`** — chama `pks refresh` para registrar o novo repositório no daemon e atualizar o registro de vaults em `{PKS_VAULTS_DIR}/`. Garante que o repo recém-inicializado apareça imediatamente nas listagens de `list_knowledge_vaults`.

---

## O Arquivo `.pks/config.toml`

Template gerado pelo `pks init` com todos os campos documentados:

```toml
# Gerado automaticamente por `pks init` em 2026-03-15
# Edite conforme necessário. Execute `pks init --force` para regenerar.

[project]
# Nome do projeto (inferido do basename do diretório)
name = "payment-api"

# Caminho absoluto para o diretório .git comum do repositório.
# Suporta múltiplas worktrees: todas apontam para o mesmo git_common_dir.
# Usado como RepoId canônico pelo daemon (ver M11).
git_common_dir = "/Users/dev/payment-api/.git"

[indexing]
# Habilita a indexação automática (acionada via git hooks e pks refresh)
enabled = true

# Caminhos monitorados, relativos à raiz do projeto
watch_paths = ["."]

# Padrões ignorados na indexação
ignore_patterns = [
  "target/",
  "node_modules/",
  ".git/",
  "dist/",
  "build/",
  "*.lock",
]

[journal]
# Shadow journaling automático via Git hooks (post-commit, post-checkout).
# false por padrão: opt-in explícito por razões de privacidade.
shadow_journaling = false

# Número mínimo de palavras para persistir uma entrada de journal.
# Entradas abaixo deste limiar são descartadas silenciosamente.
min_words_per_entry = 50

[embedding]
# Provider de embeddings vetoriais.
# "none" = apenas BM25 (padrão, zero dependências externas).
# "ollama" = habilita busca semântica via Ollama local (requer --with-vector-db).
provider = "none"

# Modelo Ollama usado quando provider = "ollama"
# ollama_model = "nomic-embed-text"
```

---

## O Slash Command `/pks-init`

O arquivo `.agent/workflows/pks-init.md` define o comportamento do LLM quando o usuário solicita a inicialização do PKS via chat (sem usar o terminal diretamente).

Quando o usuário diz "inicialize o PKS neste projeto" ou usa `/pks-init`, o LLM executa automaticamente:

1. **Executa `pks init` silenciosamente** via ferramenta de shell, capturando o output.
2. **Lê o `.pks/config.toml` gerado** para confirmar os valores inferidos (`name`, `git_common_dir`).
3. **Reporta o resultado** ao usuário: N arquivos indexados, N chunks, status da branch `pks-knowledge`, e o `RepoId` registrado.
4. **Confirma que o PKS está ativo** e sugere um `pks search` de exemplo baseado no nome do projeto.

Se o comando falhar (ex: diretório não é um repositório Git), o LLM captura o erro, explica a causa ao usuário em PT-BR, e sugere a ação corretiva sem entrar em loop de retry.

O arquivo do workflow seguirá o padrão dos demais em `.agent/workflows/` (ex: `pks-install.md`): cabeçalho com nome, objetivo, pré-condições, e passos numerados com o comportamento esperado do LLM.

---

## Subtarefas

| ID | Descrição | Arquivos | Dependência |
|----|-----------|----------|-------------|
| T14.1 | Implementar `InitCommand` struct com campo `project_path: PathBuf` e `force: bool`, e método `run() -> Result<()>` | `[CREATE] pks/src/cli/init.rs` | — |
| T14.2 | Detecção do git root via `git rev-parse --show-toplevel` e `--git-common-dir`, usando `std::process::Command` com validação de output | `pks/src/cli/init.rs` | — |
| T14.3 | Geração do template `config.toml` com valores inferidos automaticamente (`name` do basename, `git_common_dir` do Git) | `pks/src/cli/init.rs` | T14.2 |
| T14.4 | Criação da branch `pks-knowledge` via `git2-rs` (branch órfã, sem checkout); idempotente se a branch já existir | `pks/src/cli/init.rs` | — |
| T14.5 | Registro via IPC: envio de `PksCommand::RegisterRepo { repo_id, config_path }` ao daemon | `pks/src/cli/init.rs` | M10 (IPC), M11 (RepoId) |
| T14.6 | Disparo da indexação inicial via `PksCommand::TriggerIndex { repo_id }` e exibição do resumo retornado pelo daemon | `pks/src/cli/init.rs` | T14.5 |
| T14.7 | Criar o slash command `/pks-init` para uso por LLMs | `[CREATE] .agent/workflows/pks-init.md` | T14.1–T14.6 |
| T14.8 | Teste de integração end-to-end: cria repositório Git temporário, executa `pks init`, verifica as 7 etapas (`.pks/config.toml` existe, branch criada, daemon registrado, chunks > 0) | `[CREATE] pks/tests/test_init_e2e.rs` | T14.1–T14.6 |
| T14.9 | Chamar `pks refresh` ao final de `InitCommand::run()` para registrar o repo no daemon de forma explícita | `pks/src/cli/init.rs` | T14.5 |

---

## Critérios de Aceite do M14

- [ ] `pks init` completa em < 30 segundos em um repositório com até 500 arquivos `.md`
- [ ] `.pks/config.toml` é gerado com `name` e `git_common_dir` corretos, sem intervenção manual
- [ ] Branch `pks-knowledge` é criada automaticamente via `git2-rs`; segunda execução é idempotente
- [ ] `pks search "qualquer termo"` funciona imediatamente após `pks init` (indexação inicial concluída)
- [ ] `is_initialized(path: &Path) -> bool` retorna `true` após init e `false` antes
- [ ] `pks init --force` sobrescreve `.pks/config.toml` sem abortar
- [ ] Teste de integração `test_init_e2e` passa em `cargo test` sem mocks
- [ ] Slash command `/pks-init` documentado em `.agent/workflows/pks-init.md` e funcional
- [ ] Nenhuma alteração em `.gitignore` do repositório do usuário (usa `.git/info/exclude` para excluir `.pks/` de commits acidentais)
- [ ] `pks init` executa `pks refresh` ao final do bootstrap; o repo aparece em `pks list` imediatamente após

---

## Tratamento de Edge Cases

### Projeto já inicializado

`is_initialized(path)` verifica a existência de `.pks/config.toml`. Se encontrado, `run()` imprime aviso e retorna `Ok(())` sem sobrescrever. Com `--force`, sobrescreve apenas o `config.toml`; branch e daemon registration são idempotentes.

### Diretório não é um repositório Git

`git rev-parse --show-toplevel` retorna código de saída não-zero. O `InitCommand` captura o erro, imprime:

```
✗ Erro: diretório atual não é um repositório Git.
  Execute `git init` antes de usar `pks init`.
```

E retorna `Err(PksError::NotAGitRepo)`.

### Daemon não está em execução

O módulo IPC (M10) tenta conexão ao socket. Em caso de falha, aciona auto-spawn com lockfile antes de enviar `RegisterRepo`. Se o auto-spawn também falhar (ex: binário não encontrado no PATH), `pks init` completa as etapas 1-4 localmente e avisa:

```
⚠ Daemon offline — config salva localmente.
  O registro e a indexação ocorrerão na próxima inicialização do daemon.
```

A inicialização não é bloqueada pela ausência do daemon.

### Múltiplas worktrees do mesmo repositório

O `git_common_dir` aponta para o mesmo `.git` em todas as worktrees (comportamento nativo do Git). O `RepoId` gerado em M11 é canônico: `pks init` executado em qualquer worktree registra o mesmo `RepoId` no daemon, sem duplicação de índices.
