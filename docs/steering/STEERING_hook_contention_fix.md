# STEERING: Contenção de Lock no Git Hook post-commit

**Data:** Março de 2026
**Severidade:** Alta — afeta a experiência de uso do PKS em projetos com commits frequentes
**Componentes:** `vault_init.rs`, `cli/mod.rs`, `git_journal_append.rs`, `git::bare_commit`

---

## Contexto

O PKS instala um git hook `post-commit` que executa `pks hook-post-commit` em background (`&`). Esse comando:

1. Escreve um trigger file (`.git/pks_hook_trigger`) — leve, sem problema
2. Chama `append_commit_to_daily_log()` — **abre o repositório via libgit2, lê o branch `pks-knowledge` e faz um commit real (`BareCommit::write_file`)**

O passo 2 é a raiz do problema.

---

## 1. Contenção de Lock Git (Enfileiramento)

### O Problema

`BareCommit::write_file` precisa adquirir o lock do repositório (`.git/index.lock` ou equivalente interno do libgit2). Em cenários de desenvolvimento ativo — especialmente com Claude Code, que pode gerar múltiplos commits em sequência rápida — vários processos `pks hook-post-commit &` são disparados quase simultaneamente.

Cada processo tenta:
- `Repository::open` → ok (compartilhado)
- `read_log_from_branch` → ok (leitura)
- `bc.write_file(...)` → **bloqueia esperando lock** se outro processo já está commitando

Isso cria uma **fila de processos background** competindo pelo mesmo lock, degradando a performance do git e potencialmente atrasando o próximo commit do usuário.

**Arquivos envolvidos:**
- `src/vault_init.rs:104` — instalação do hook
- `src/cli/mod.rs:134-149` — `run_hook_post_commit`
- `src/git_journal_append.rs:153-167` — `append_line_to_branch` (onde o lock é adquirido)

---

## 2. Race Condition na Leitura/Escrita

### O Problema

`append_line_to_branch` faz:
```
existing = read_log_from_branch(...)   // lê conteúdo atual
new_content = existing + line          // concatena
bc.write_file(new_content)             // escreve
```

Dois processos concorrentes podem:
1. Ambos lerem o mesmo `existing`
2. Ambos escreverem `existing + sua_linha`
3. O segundo sobrescreve o primeiro → **perda de entrada no journal**

Isso não é protegido pelo lock do git porque a leitura acontece antes da escrita.

---

## Solução Proposta: Append-then-Flush (mesmo padrão do `record-event`)

O PKS já resolve esse problema corretamente no `record-event` (hook PostToolUse do Claude Code): eventos são appendados a um arquivo JSONL local e depois flushed em batch. O hook post-commit deve adotar o mesmo padrão.

### Fase 1 — Hook leve (append-only)

1. **Modificar `run_hook_post_commit`** para apenas appendar uma linha JSONL. **Recomendação arquitetura:** Salvar no diretório do próprio repositório em `.git/pks_pending_commits.jsonl` (ao invés de um cache global como `~/.pks/hooks/{repo_hash}...`) para evitar perda de contexto se o repo for movido:
   ```json
   {"sha":"abc1234","branch":"main","repo":"/path/to/repo","ts":1711800000}
   ```
2. **Remover** a chamada direta a `append_commit_to_daily_log` do hook
3. O append é atômico em modo `O_APPEND` — sem race condition, sem lock git

### Fase 2 — Flush batched

1. **Adicionar lógica de flush** no daemon (`refresh`) ou no `flush-session`:
   - Ler todos os eventos pendentes do JSONL
   - Agrupar por data
   - Fazer **um único `BareCommit::write_file`** por dia, com todas as linhas
   - **Mover (rename atômico)** o JSONL para um arquivo temporário (ex: `.git/pks_pending_commits.processing.jsonl`) *antes* de processar. Após o commit bem-sucedido, deletar o temporário. **Jamais truncar/remover após leitura direta**, pois isso introduz uma *race condition* (novos commits ocorrendo durante o processo de flush podem ser apagados e perdidos).
2. Alternativa: flush via debounce no daemon (ex: 5s após último evento)

### Fase 3 — Limpeza

1. Remover `append_line_to_branch` (ou manter apenas para uso direto via CLI)
2. Atualizar `pks doctor` para validar o novo formato
3. Atualizar a instalação do hook em `vault_init.rs` — o `&` continua, mas agora o processo é tão rápido que a concorrência não é mais problema

---

## Benefícios Esperados

| Antes | Depois |
|-------|--------|
| Cada commit dispara um `git commit` no branch pks-knowledge | Cada commit faz um append de ~100 bytes num arquivo local |
| Lock contention com N processos simultâneos | Zero contention — append atômico via OS |
| Race condition pode perder entradas | Sem race condition |
| Latência do hook: ~200-500ms (git open + read + commit) | Latência do hook: <5ms (file append) |
| Acúmulo de processos background | Processo termina quase instantaneamente |

---

## Riscos e Mitigações

- **Crash antes do flush**: Eventos ficam no JSONL e serão processados no próximo flush. Sem perda.
- **JSONL cresce demais**: O flush periódico (daemon ou session-end) mantém o arquivo pequeno. Adicionar um guard de tamanho máximo se necessário.
- **Migração**: Repositórios existentes com o hook antigo continuarão funcionando — o novo `run_hook_post_commit` simplesmente muda o comportamento interno, sem alterar a assinatura CLI.
- **Consistência Pontual (Sincronia)**: Como os dados vão para o branch apenas no flush (consistência eventual), comandos de leitura do CLI (se houver algum que dependa desses dados frescos) devem invocar ou forçar um flush antes de ler do `pks-knowledge`.
- **Locks em Cross-Platform (Windows)**: O append ou o rename atômico concorrentes podem sofrer com bloqueios estritos em sistemas NTFS. A implementação em Rust deve usar `std::fs::OpenOptions` com as flags de compartilhamento adequadas via extensões do SO (ex: `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`).

---

## Checklist de Implementação

- [x] Criar módulo `src/hooks/commit_event_log.rs` com append JSONL
- [x] Refatorar `run_hook_post_commit` para usar append-only
- [x] Adicionar flush de commit events no `flush-session` ou `refresh`
- [x] Atualizar `pks doctor` para o novo formato
- [x] Adicionar testes unitários para append e flush
- [ ] Testar manualmente com commits rápidos em sequência
- [ ] Remover código morto (`append_line_to_branch` se não mais necessário)
