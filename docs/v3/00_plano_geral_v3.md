# PKS v3 — Plano Geral

**Data:** 2026-03-19
**Status:** Concluído
**Baseline:** v2 (M10–M14) concluído e em produção local

---

## Contexto e Motivação

Uma auditoria do código v2 revelou três problemas que comprometem o valor real do PKS como sistema de memória técnica:

### Problema 1 — Shadow Journaling nunca funciona

O M12 foi implementado com a arquitetura certa (acumulação em memória → flush no fim da sessão via BareCommit) mas sem integração real. Nenhum código do fluxo principal invoca `record_tool_event()`. O `ipc/server.rs` tentou suprir isso, mas criou uma instância nova de `ShadowJournalHook` por evento e fez flush imediato — destruindo o modelo de acumulação. O resultado: **zero journals gerados em produção**.

A solução é mudar de arquitetura: em vez de tentar orquestrar a captura de eventos granular via IPC, usar os **hooks nativos do Claude Code** (`PostToolUse` e `Stop`) armazenando em JSONL e os **workflows do Antigravity** (`/commit`) para lote local. Após o commit final (BareCommit), o CLI envia passivamente uma notificação via IPC (Refresh) para o daemon atualizar o índice Tantivy, resolvendo a pesquisabilidade imediata sem violar lock concorrente.

### Problema 2 — Prometheus/ indexado junto com o repo pai

O walker do `boot_indexer.rs` não exclui `prometheus/` ao varrer um repositório. O worktree criado por `pks init` tem um arquivo `.git` (não diretório), e `prometheus/` não está na lista de exclusão do walker. Isso significa que os arquivos `.md` do vault (incluindo journals) são indexados com o mesmo `repo_id` do projeto pai — impossibilitando filtragem por origem.

### Problema 3 — `pks init` incompleto (corrigido em 2026-03-19)

O `pks init` não instalava o post-commit hook, não criava o worktree `prometheus/` e não inicializava a estrutura de pastas. `init_vault()`, `install_post_commit_hook()` e `create_pks_branch_and_worktree()` existiam no código mas nunca eram chamados. **Corrigido nesta sessão** — `pks init` agora executa o setup completo.

---

## Mapa de Milestones v3

| Milestone | Nome | Objetivo | Complexidade |
|---|---|---|---|
| **M15** | Shadow Journaling via Agent Hooks | Captura automática de sessões LLM via `PostToolUse`/`Stop` hooks com acumulação em JSONL | **M** |
| **M16** | Vault Isolation | Separar `prometheus/` do índice do repo pai com `repo_id` próprio e walker exclusion | **P** |
| **M17** | Simplificação Prometheus + Gatilhos | Reduzir de 6→3 pastas, todas com gatilhos CLI + MCP (`features/`, `decisions/`, `journals/`) | **M** |

---

## Dependências

```
M16 (Vault Isolation)     ← pode ser feito independente
M15 (Hooks)               ← pode ser feito independente
M17 (Simplificação)       ← depende de M15 e M16 concluídos
```

M15 e M16 são independentes entre si e podem ser implementados em paralelo.
M17 depende de ambos: M15 define o fluxo de journals, M16 isola o vault — M17 reorganiza a estrutura e adiciona gatilhos universais.

---

## O que NÃO está no escopo do v3

- Refatoração do IPC (mantido como está — `RecordToolEvent` será removido, não substituído)
- Novos tipos de fonte indexada além de `.md`
- Interface web ou dashboard
- Antigravity: Integrado via método Batch/Workflow, logo hooks granulares não estão no escopo
- Migração de vaults existentes com pastas antigas (projeto em fase de testes)

---

## Critérios de Conclusão da v3

- [x] **M15:** Após qualquer sessão Claude Code com ≥1 tool call de escrita (Edit/Write/Bash), um arquivo `journals/YYYY-MM-DD_{session_id}.md` existe na branch `pks-knowledge` sem intervenção manual
- [x] **M15:** `pks record-event` lê JSON do stdin e faz append em `~/.pks/sessions/{session_id}.jsonl` sem I/O adicional
- [x] **M15:** `pks flush-session <session_id>` gera o markdown consolidado, commita via BareCommit e limpa o arquivo de sessão
- [x] **M15:** `pks record-event` aplica redação de secrets antes de gravar no JSONL
- [x] **M15:** `pks submit-journal --agent <nome> --file <arquivo.md>` implementado para ingestão em lote (Antigravity workflows)
- [x] **M16:** `prometheus/` é excluído do walker do repo pai (nunca aparece no mesmo `repo_id`)
- [x] **M16:** O conteúdo de `prometheus/` é indexado com `repo_id` próprio derivado do seu `.git` (worktree pointer)
- [x] **M17:** `pks init` cria exatamente 3 pastas: `features/`, `decisions/`, `journals/`
- [x] **M17:** `pks decision <msg>` grava ADR em `decisions/` via BareCommit
- [x] **M17:** Ferramentas MCP `pks_add_decision` e `pks_add_feature` operacionais
- [x] **M17:** Referências a `90-ai-memory` eliminadas — tudo usando `journals/`
- [x] `cargo test --workspace` passa 100% após todos os milestones
- [x] `.claude/settings.json` do projeto PKS contém a configuração de hooks pronta para uso

---

## Referências

- Auditoria do shadow journal: conversa de 2026-03-19
- Correção do `pks init`: `src/cli/init.rs` commit de 2026-03-19
- Formato de hooks Claude Code: documentação oficial (PostToolUse, Stop)
- Estratégia Antigravity: Mudança aprovada para Ingestão em Lote via Workflows e Checklists (dispensando hooks do tipo payload)
