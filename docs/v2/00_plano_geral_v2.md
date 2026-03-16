# PKS v2 — Plano Geral

**Data:** 2026-03-15
**Status:** Planejamento — pré-execução
**Milestone de partida:** M10 (pós-conclusão de M9)

---

## Visão Geral

O PKS v1 (Milestones M1–M9) entregou um sistema funcional de RAG local: daemon com BM25 via Tantivy, embeddings opcionais via Ollama, Git Journaling passivo (M8) e instalador one-click para macOS e Windows (M9). O sistema está em estado de "produção local" — operacional, mas com limitações arquiteturais que impedem escalabilidade e uso confiável em ambientes multi-worktree e multi-IDE.

O PKS v2 endereça três classes de problemas identificados na operação real. Primeiro, o modo `--stdio` atual instancia o `PrevalentState` completo a cada invocação MCP, resultando em múltiplas cópias do índice BM25 em RAM e ausência de coordenação entre processos. A transformação em Singleton via IPC elimina essa ineficiência. Segundo, a identidade de repositório (`RepoId`) hoje é frágil em cenários com múltiplas worktrees Git do mesmo repositório; o `RepoId Unificado` via `git rev-parse --git-common-dir` corrige isso sem mudança de esquema de dados. Terceiro, a dependência do Ollama bloqueia onboarding em máquinas sem GPU ou sem conexão — o modo BM25-only deve ser 100% funcional e explicitamente suportado.

Além de corrigir essas limitações, o v2 introduz duas capacidades novas de alto valor: o `pks_execute` como ferramenta MCP que intercepta problemas de janela de contexto roteando execução pelo sandbox do Context-Mode; e o `pks init` / `/pks-init` que elimina a necessidade de configuração manual, tornando o onboarding verdadeiramente zero-config.

---

## Steerings Aplicados

- **STEERING_remove_fswatcher.md** — Substitui FSWatcher contínuo (`notify` crate) por comando `pks refresh` explícito. Impacta M10 (remover init do watcher do loop principal) e M14 (`pks init` chama `pks refresh` internamente).

---

## Pré-requisitos

Os seguintes itens devem estar presentes e operacionais antes de iniciar qualquer milestone v2:

| Pré-requisito | Verificação |
|---|---|
| M8 concluído | `cargo test` passa em `git_journal_harness.rs`; hook `post-commit` anexa entradas em `prometheus/90-ai-memory/` |
| M9 concluído | Workflow `/pks-install` funcional em macOS e Windows; `pks --stdio` responde via MCP |
| `pks --stdio` em produção | Pelo menos um projeto com vault ativo e indexado em `~/pks-vaults/` ou equivalente |
| Rust toolchain ≥ 1.77 | `rustc --version` confirma edição estável com suporte a `async fn` em traits |
| Repositório PKS com testes passando | `cargo test --workspace` verde no branch `main` |
| Documentação v1 arquivada | Arquivos `docs/plan/` presentes e referenciáveis (não deletados) |

---

## Mapa de Milestones v2

| Milestone | Nome | Objetivo Principal | Arquivos Principais | Complexidade |
|---|---|---|---|---|
| **M10** | Core Engine & IPC Singleton | Transformar o daemon em Singleton verdadeiro com Unix Domain Socket; eliminar múltiplas instâncias de `PrevalentState` | `pks/src/cli.rs` [MODIFY]<br>`pks/src/main.rs` [MODIFY]<br>`pks/src/ipc/mod.rs` [NEW]<br>`pks/src/commands/refresh.rs` [NEW]<br>`pks/src/cli.rs` [MODIFY — add refresh subcommand] | **G** |
| **M11** | RepoId Unificado + Bare Commits | Unificar identidade de repo via `git rev-parse --git-common-dir`; escrever commits `pks-knowledge` diretamente em worktrees secundárias sem sujar a árvore de trabalho | `pks/src/git/repo_identity.rs` [NEW]<br>`pks/src/git/bare_commit.rs` [NEW] | **M** |
| **M12** | Shadow Journaling Passivo | Journaling passivo de sessões LLM via hooks Git, sem intervenção do usuário | `pks/src/hooks/shadow_journal.rs` [NEW] | **M** |
| **M13** | Ollama Opcional + pks_execute | Modo BM25-only 100% funcional sem Ollama; nova ferramenta MCP `pks_execute` roteando execução pelo Context-Mode para proteger janela de contexto | `pks/src/mcp/tools/pks_execute.rs` [NEW]<br>`.agent/workflows/pks-install.md` [MODIFY] | **G** |
| **M14** | Zero-Config Onboarding | Comando `pks init` + slash command `/pks-init` para bootstrap automático de qualquer repositório | `pks/src/cli/init.rs` [NEW]<br>`.agent/workflows/pks-init.md` [NEW] | **P** |

**Legenda de complexidade:** P = Pequeno (≤3 dias), M = Médio (3–7 dias), G = Grande (>7 dias ou risco arquitetural alto)

---

## Dependências entre Milestones

```
M10 (IPC Singleton)
 └──► M11 (RepoId Unificado + Bare Commits)
       ├──► M12 (Shadow Journaling Passivo)
       │     └──► M14 (Zero-Config Onboarding)
       └──► M13 (Ollama Opcional + pks_execute)
             └──► M14 (Zero-Config Onboarding)
```

**Detalhamento das dependências:**

| Dependência | Razão |
|---|---|
| M11 depende de M10 | O `RepoId Unificado` pressupõe que há um único daemon coordenando o estado; sem IPC Singleton, múltiplos processos podem calcular `RepoId` de forma inconsistente |
| M12 depende de M11 | O `shadow_journal.rs` grava entradas via Bare Commits; `bare_commit.rs` (M11) deve estar estável antes |
| M13 depende de M11 | O `pks_execute` roteia queries que incluem `RepoId` como contexto de filtro; a semântica unificada de M11 é necessária para resultados corretos |
| M14 depende de M12 e M13 | O `pks init` instala hooks de Shadow Journaling (M12) e configura o modo Ollama opcional (M13); ambos devem estar implementados |

---

## Critérios de Entrada (Pronto para Iniciar v2)

- [ ] `cargo test --workspace` passa 100% no branch `main` com M8 e M9 integrados
- [ ] O MCP server `pks --stdio` responde às ferramentas `list_knowledge_vaults` e `search_knowledge_vault` em pelo menos um ambiente de produção local
- [ ] Existe ao menos um vault com ≥50 arquivos Markdown indexados, servindo como fixture de teste para M10
- [ ] A documentação `docs/plan/` (v1) foi arquivada e os arquivos `docs/v2/` criados
- [ ] Nenhum issue aberto com label `blocking` no repositório PKS
- [ ] O arquivo `pks/src/cli.rs` foi revisado e o comportamento atual de `run_stdio_server` está documentado inline antes de qualquer refatoração

---

## Critérios de Conclusão da v2

- [ ] **M10:** `pks --daemon` e `pks --stdio` rodam como processos separados; o processo `--stdio` nunca carrega BM25 em RAM; PID lockfile em `$XDG_RUNTIME_DIR/pks.pid` previne múltiplos daemons; `pks refresh` subcomando implementado; `notify` crate removido do `Cargo.toml`
- [ ] **M11:** `git rev-parse --git-common-dir` é o único mecanismo de derivação de `RepoId`; múltiplas worktrees do mesmo repositório compartilham o mesmo `RepoId` no `PrevalentState`; commits `pks-knowledge` não aparecem em `git status` da worktree principal
- [ ] **M12:** Após qualquer sessão com a LLM que resulte em commit, uma entrada é anexada automaticamente em `prometheus/90-ai-memory/YYYY-MM-DD_log.md` sem intervenção manual
- [ ] **M13:** `pks --stdio` sobe e responde queries BM25 com `OLLAMA_BASE_URL` ausente ou inválido; `/pks-install` atualizado com seção explícita "Modo BM25-only"; ferramenta `pks_execute` disponível no servidor MCP e testada com pelo menos 1 cenário de janela de contexto excedida
- [ ] **M14:** `pks init <path>` executa os 4 passos (orphan branch, stealth directory, hooks seguros, defaults zero-config) em <5 segundos; `/pks-init` invocado pela LLM produz um vault funcional sem nenhum input manual; `pks init` chama `pks refresh` internamente ao final do bootstrap
- [ ] `cargo test --workspace` passa 100% com cobertura de integração para cada novo módulo
- [ ] A documentação `docs/v2/` contém ao menos: este plano geral, um arquivo por milestone com critérios de aceite detalhados, e changelog de breaking changes em relação ao v1
