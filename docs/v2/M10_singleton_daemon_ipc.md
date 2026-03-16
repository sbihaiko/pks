# M10 — Singleton Daemon + IPC

| Campo            | Valor                          |
|------------------|--------------------------------|
| **Status**       | PENDENTE                       |
| **Data**         | 2026-03-15                     |
| **Depende de**   | M9 completo                    |
| **Branch alvo**  | `main`                         |
| **Arquivos-chave** | `pks/src/cli.rs`, `pks/src/main.rs`, `pks/src/ipc/mod.rs` |

---

## 1. Objetivo

Transformar o PKS em um singleton verdadeiro: um único processo Daemon que persiste em background e serve múltiplos clientes CLI/MCP via Unix Domain Socket. O binário `pks` passa a operar em dois modos exclusivos — `--daemon` (servidor) e modo cliente (padrão) — eliminando a possibilidade de múltiplas instâncias carregarem índices BM25 redundantemente na RAM.

---

## 2. Motivação Técnica

O PKS pós-M9 não impede que múltiplas abas da IDE (ou múltiplas invocações do Proxy MCP `--stdio`) subam processos independentes, cada um inicializando seu próprio `PrevalentState` com o índice BM25 completo. Isso gera três problemas críticos:

- **Consumo de RAM multiplicado**: cada instância carrega a totalidade do índice BM25 (`tantivy::Index`) em memória. Em um repositório médio (500 MB de código), esse custo pode ultrapassar 1 GB por instância adicional.
- **Race conditions em gravações**: múltiplos processos acessando a branch `pks-knowledge` via `git2-rs` simultaneamente corrompem o log de transações Prevayler sem lock explícito de processo.
- **Inconsistência de estado**: buscas realizadas em instâncias distintas retornam resultados divergentes caso os índices tenham sido atualizados em momentos diferentes.

A solução é um modelo cliente-servidor local onde apenas o Daemon detém o `Arc<Mutex<PrevalentState>>`, e todo acesso externo passa pelo socket `/tmp/pks.sock`.

---

## 3. Design da Arquitetura

### Topologia

```
┌─────────────────────────────────────────────────────────────┐
│  IDE / Claude Code                                          │
│                                                             │
│  ┌──────────────────┐      ┌──────────────────────────────┐ │
│  │  pks --stdio     │      │  pks search "query"          │ │
│  │  (Proxy MCP)     │      │  (CLI client)                │ │
│  │  thin client     │      │  thin client                 │ │
│  └────────┬─────────┘      └──────────────┬───────────────┘ │
│           │ JSON-RPC                       │ PksCommand      │
│           │ sobre IpcClient                │ sobre IpcClient │
└───────────┼───────────────────────────────┼─────────────────┘
            │                               │
            │     Unix Domain Socket        │
            │     /tmp/pks.sock             │
            └───────────────┬───────────────┘
                            │
            ┌───────────────▼───────────────────────────────────┐
            │  pks --daemon                                     │
            │  IpcServer::accept_loop()                         │
            │                                                   │
            │  Arc<Mutex<PrevalentState>>                       │
            │  ├── tantivy::Index (BM25)                        │
            │  ├── HashMap<RepoId, VaultMeta>                   │
            │  └── CommandChannel (Prevayler queue)             │
            └───────────────────────────────────────────────────┘
```

### Fluxo de Startup (Auto-Spawn Seguro)

```
pks search "query"
       │
       ▼
IpcClient::is_server_running()?
       │
   ┌───┴────────┐
  NÃO           SIM
   │             │
   ▼             ▼
Tentar adquirir  Enviar PksCommand::Search
flock em         via socket, aguardar
$TMPDIR/pks.pid  PksResponse, imprimir, sair
   │
   ▼
Lock adquirido?
   │
  SIM → spawn pks --daemon (std::process::Command::new)
         aguardar socket com exponential backoff (50ms→1.6s)
         enviar comando após socket pronto
   │
  NÃO → outro processo está fazendo spawn
         aguardar socket com backoff
         enviar comando após socket pronto
```

---

## 4. Subtarefas

| ID     | Descrição                                                    | Arquivos                                      | Depende de | Critério de Aceite                                                                                      |
|--------|--------------------------------------------------------------|-----------------------------------------------|------------|---------------------------------------------------------------------------------------------------------|
| T10.1  | Refatorar `cli.rs`: adicionar flag `--daemon`, separar modo cliente do modo servidor | `pks/src/cli.rs`                              | M9         | `pks --daemon` inicia servidor; `pks search "x"` sem `--daemon` age como cliente; erro claro se ambos usados |
| T10.2  | Implementar `ipc/mod.rs`: `IpcClient`, `IpcServer`, enums `PksCommand`/`PksResponse` | `pks/src/ipc/mod.rs`                          | T10.1      | `IpcClient::send_command` serializa/deserializa via `serde_json`; `IpcServer::accept_loop` processa requisições concorrentes com `tokio::spawn` por conexão |
| T10.3  | Atualizar `main.rs`: lógica de startup com detecção de instância, PID lockfile, backoff | `pks/src/main.rs`                             | T10.2      | Segunda invocação de `pks start` conecta como cliente sem subir novo Daemon; lockfile liberado em `SIGTERM`/`SIGINT` |
| T10.4  | Atualizar arquivos de serviço (`launchd`/`systemd`) para usar `--daemon`             | `pks/deploy/pks.plist`, `pks/deploy/pks.service` | T10.3      | `launchctl load` e `systemctl start pks` levantam o Daemon com `--daemon`; `pgrep pks` retorna exatamente um PID |
| T10.5  | Teste de integração: singleton e comunicação cliente-servidor                        | `pks/tests/singleton_ipc_test.rs`             | T10.3      | `cargo test` confirma: (a) apenas um socket criado; (b) segunda invocação retorna resposta sem novo processo; (c) socket removido após shutdown |

---

## 5. Critérios de Aceite do M10

- `pks --daemon` vincula `/tmp/pks.sock` e permanece em execução; `pgrep -c pks` retorna `1` mesmo após 10 invocações paralelas de `pks search`.
- O proxy MCP (`pks --stdio`) **não** instancia `PrevalentState`; toda a lógica de busca é delegada ao Daemon via `IpcClient::send_command`.
- `IpcClient::is_server_running()` retorna `false` se o socket existir mas o processo não responder (socket órfão), acionando cleanup e re-spawn.
- O PID lockfile em `$TMPDIR/pks.pid` usa `flock` exclusivo; apenas o processo que obtiver o lock executa `std::process::Command::new("pks").arg("--daemon")`.
- O exponential backoff aguarda o socket com intervalos `[50ms, 100ms, 200ms, 400ms, 800ms, 1600ms]`; após 6 tentativas sem sucesso, retorna `Err("daemon unavailable after spawn")`.
- `PksCommand` e `PksResponse` são serializados via `serde_json`; incompatibilidade de versão entre cliente e servidor retorna `PksResponse::Err("version mismatch: client=X server=Y")`.
- O socket `/tmp/pks.sock` é removido pelo Daemon no handler de `SIGTERM`/`SIGINT` via `ctrlc` crate.
- Todos os itens acima são verificados por `cargo test --test singleton_ipc_test` sem mocks.

---

## 6. Riscos e Mitigações

| Risco | Probabilidade | Mitigação |
|-------|---------------|-----------|
| **Socket órfão após crash do Daemon** — o arquivo `/tmp/pks.sock` permanece no disco; `IpcClient::is_server_running()` tenta conexão, falha, mas não distingue "crash" de "ainda subindo" | Alta | `is_server_running()` envia probe `PksCommand::Ping` com timeout de 200ms. Se a conexão recusar (`ConnectionRefused`) ou expirar, remove o socket stale e aciona re-spawn com lockfile |
| **Incompatibilidade Windows** — `UnixListener` não existe no Windows; a path `/tmp/pks.sock` é inválida | Média | Compilação condicional `#[cfg(unix)]` para `tokio::net::UnixListener`; `#[cfg(windows)]` usa `tokio::net::TcpListener` em `127.0.0.1:0` com porta gravada em `$TEMP\pks.port`; mesmo enum `PksCommand`/`PksResponse` em ambos os casos |
| **Starvation de clientes sob carga** — múltiplos `tokio::spawn` por conexão competindo pelo `Arc<Mutex<PrevalentState>>` causam latência alta para buscas longas | Baixa | O `Mutex` do `PrevalentState` é mantido apenas durante a execução da query; operações de I/O (serialização, envio da resposta) ocorrem fora do lock. Monitorar com `tokio-console` em ambiente de desenvolvimento |

---

## 7. Dependências Externas

| Crate              | Versão mínima | Uso                                                        |
|--------------------|---------------|------------------------------------------------------------|
| `tokio`            | 1.38          | Runtime assíncrono; `tokio::net::UnixListener`, `tokio::spawn`, `tokio::time::timeout` |
| `serde`            | 1.0           | Derivar `Serialize`/`Deserialize` para `PksCommand` e `PksResponse` |
| `serde_json`       | 1.0           | Codec do protocolo IPC sobre o socket                      |
| `ctrlc`            | 3.4           | Handler de `SIGTERM`/`SIGINT` para remoção do socket e lockfile no shutdown |
| `fs2`              | 0.4           | `FileExt::lock_exclusive` para o PID lockfile multiplataforma (macOS/Linux/Windows) |
| `notify`           | —             | **REMOVIDO** — ver seção 9 (Steering) |

Todas as crates acima já são maduras e sem dependências de sistema além das providas pelo Rust stdlib. Nenhuma linkagem dinâmica adicional é necessária no binário final.
## 8. Observações Críticas (v2 Feedback)

- **Risco de Deadlock no Startup:** O timeout de 1.6s pode ser curto.
- **Dica:** O Daemon deve estar pronto no socket ANTES de indexações pesadas. Use um worker thread para indexação.
- **Versioning:** Use `PKS_IPC_VER=2` no protocolo para evitar quebras em upgrades futuros.

---

## 9. Impacto do Steering: Remoção do FSWatcher

Conforme definido em `STEERING_remove_fswatcher.md`, o loop de FSWatcher (crate `notify`) deve ser removido da função `main.rs` do daemon durante a implementação de M10. Este mecanismo de auto-discovery em tempo real é substituído por um subcomando CLI explícito `pks refresh` que permite scan sob demanda do diretório `PKS_VAULTS_DIR`, registro de novos repositórios e purga de entradas stale.

| ID | Descrição | Arquivo |
|----|-----------|---------|
| T10.6 | Remover inicialização do FSWatcher (`notify`) de `main.rs`; remover crate `notify` de `Cargo.toml` | `pks/src/main.rs`, `pks/Cargo.toml` |
| T10.7 | Implementar subcomando `pks refresh`: scan de `{PKS_VAULTS_DIR}/`, registro/purge de repos, flags `--dry-run` | `pks/src/cli.rs` [MODIFY], `pks/src/commands/refresh.rs` [NEW] |

**Critério de Aceite para `pks refresh`:**
- Imprime diff com `[+]` (novo repo), `[-]` (removido), `[=]` (sem mudança)
- Exit code sempre `0` mesmo sem mudanças
- Flag `--dry-run` mostra mudanças sem aplicar ao índice
