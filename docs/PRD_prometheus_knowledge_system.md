# PRD  Prometheus Knowledge System (`PKS`)

> **Objetivo**: Criar um sistema de "Memória Infinita" para usuários do Antigravity (e Claude Code), centrado no Obsidian e versionado por Git, com indexação vetorial + semântica local via um Daemon Prevalente em Rust. O sistema aprende continuamente registrando novos contextos e atualizando o conhecimento existente automaticamente a cada commit. Auto-contido como uma SKILL integrável.

> **Por que Prometheus?** Na mitologia grega, Prometeu foi o titã que *roubou o fogo dos deuses e o entregou à humanidade*  o ato fundador do conhecimento e da civilização. O `PKS` carrega essa mesma essência: ele captura o conhecimento que nasce e morre disperso em silos (código, conversas, tickets) e o consolida como uma chama viva e persistente na memória dos agentes de IA. Assim como Prometeu não criou o fogo, mas o *preservou e distribuiu*, o PKS não cria conhecimento  ele o *indexa, preserva e disponibiliza instantaneamente*, sem deixar que se apague.

---

## Contexto e Evolução

Historicamente, o fluxo de trabalho de engenharia gera artefatos de altíssimo valor que nascem e morrem em **silos isolados**:
- Modelagens e ADRs em repositórios **Git** específicos de cada microsserviço.
- Requisitos e bugs presos em **Issue Trackers** (Notion, Jira, Linear).
- Acordos técnicos e brainstorms perdidos em **Mensageiros** (Slack, WhatsApp, Email).
- E o contexto das sessões de pair-programming com agentes de **IA** (Antigravity/Claude) que evapora quando o terminal é fechado.

O *Prometheus Knowledge System* (**PKS**) nasce para solucionar a fragmentação desse conhecimento corporativo. Mais do que uma simples automação de arquivos `.md`, ele é um **Motor de RAG Contínuo e Resiliente**.

O resultado é uma "Memória Infinita" unificada: um **Daemon Único (Singleton)** em **Rust** que roda em background (via launchd/systemd) e mantém a verdade consolidada de N repositórios continuamente viva em RAM quente. Ao expor essa inteligência consolidada via **MCP (Model Context Protocol)** através de um **Cliente Proxy leve (`pks --stdio`)**, agentes de IA ganham percepção instantânea — cruzando domínios em sub-milissegundos sem estourar o limite de memória da máquina.

###  Visão Geral das Capacidades

Pense no PKS como um **bibliotecário que nunca dorme**. Você commita código, ele já sabe. Você abre uma nova sessão com o Antigravity após semanas de férias  ele lembra de tudo.

| Capacidade | O que parece para o usuário | Fase |
|---|---|---|
|  **Vault por Projeto** | Cada repo Git tem seu próprio caderno Obsidian. Organizado, navegável, humano. | MVP |
|  **Git Journal** | Commits disparam indexação instantânea. Post-commit hook faz append de commits filtrados (Conventional Commits) em `${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/YYYY-MM-DD_log.md`. Branch `pks-knowledge` isola notas do fluxo de código. | MVP |
|  **Daemon + RAM quente** | O índice vive em RAM 24/7. O Daemon nunca para. A resposta já está viva. | MVP |
|  **Busca BM25** | Busca por palavras-chave em sub-milissegundo via Tantivy. Funciona sem Ollama. | MVP |
|  **MCP Search** | `search_knowledge_vault` expõe tudo via MCP. A IA pergunta, o Daemon responde. | MVP |
|  **Embeddings vetoriais** | Busca semântica híbrida (BM25 + Cosine SIMD) via Ollama. Upgrade incremental. | Fase C |
|  **Ingestão de trackers** | Ticket do Notion/Jira vira nota Markdown em segundos, linkada e rastreável. | Fase C |
|  **Shadow Repos** | Slack, WhatsApp e e-mails viram commits. O PKS não sabe a diferença — indexa tudo. | Pos-Fase C (Apendice A) |

---

## Personas

O PKS é uma camada de memória universal. A interface varia por persona — o motor é o mesmo.

### Personas do MVP e Fase C (escopo atual)

| Persona | Como acessa o PKS | O que ganha |
|---|---|---|
|  **Desenvolvedor / Engenheiro** | Obsidian + Git commits + Antigravity CLI | Contexto persistente entre sessões de AI, busca cross-domain entre projetos |
|  **AI Agent** (Antigravity / Claude) | Ferramenta MCP `search_knowledge_vault` | Memória multi-projeto em sub-ms, sem carregar arquivos do disco |

### Personas futuras (Shadow Repositories  Fase 2.3)

| Persona | Como acessa o PKS | O que ganha |
|---|---|---|
|  **Profissional não-técnico** (gestor, analista) | App móvel ou interface web simples sobre o MCP | Busca por decisões passadas, contratos, e-mails e reuniões sem saber que o Git existe |
|  **Usuário Leigo** (qualquer pessoa) | WhatsApp ou app de mensagens nativo | Manda áudio ou texto  o Shadow Daemon converte, commita e indexa. A IA responde no mesmo chat |
|  **Organização / Time** | Slack + Email + repositórios Git do time | Conhecimento da empresa unificado: código, decisões, contratos, conversas |

>  **Visão de futuro:** A mãe do programador manda um áudio no WhatsApp sobre uma receita. O Shadow Daemon transcreve, commita como `2026-03-06_receitas.md` e indexa. Semanas depois ela pergunta no mesmo chat: *"qual era o tempo de forno do frango?"*  a IA responde com o trecho exato. Ela nunca soube que um repositório Git e um Daemon em Rust estavam por baixo. *(ver Fase 2, M7 e Apêndice A)*

---

## Requisitos de Sistema

O PKS e suas dependências exigem recursos mínimos para operar sem degradação. A tabela abaixo define o baseline e o recomendado:

| Requisito | Mínimo | Recomendado |
|---|---|---|
| **Sistema Operacional** | macOS 13+ (Ventura) / Linux kernel 5.15+ | macOS 14+ (Sonoma, Apple Silicon) |
| **RAM** | 8 GB | 16 GB+ |
| **Armazenamento** | SSD (HDD não suportado) | NVMe |
| **Ollama** | Opcional. Apenas se desejar busca semântica vetorial pós-M5. Busca BM25 funciona 100% offline sem Ollama. | GPU dedicada ou Apple Neural Engine |
| **Git** | 2.30+ | 2.40+ (melhorias em worktree) |
| **Git LFS** | 3.0+ (apenas para M6 — sincronizacao de snapshots vetoriais) | 3.4+ |
| **Rust** | 1.75+ (apenas para build do Daemon) | Stable mais recente |

> **Nota MVP:** No MVP (BM25-only), Ollama não é necessário. O Daemon funciona 100% sem Ollama instalado. Consumo de RAM é irrisório (dezenas de KB por 1000 notas). Watermarks (`PKS_MAX_VECTORS`) relevantes apenas pós-M5 (Embeddings).

---

## Estratégia de Autenticação (`.env`)

Toda autenticação do PKS e dos MCPs conectados usa **variáveis de ambiente** como mecanismo primário, carregadas de um arquivo `.env` na raiz do projeto. Alternativamente, credenciais podem ser lidas do **keychain do SO** (`security` no macOS, `secret-tool` no Linux).

### Fluxo e Arquivos

1.  `.env.example`: Versionado como template com todas as variáveis vazias e documentação.
2.  `.env`: NÃO versionado (coberto pelo `.gitignore`), contém os valores reais das credenciais.
3.  `.mcp.json`: Referencia `${VAR_NAME}` para injetar variáveis de ambiente nos MCP servers.

### Variáveis Principais

| Variável | Uso |
|----------|-----|
| `NOTION_TOKEN` | Integração com Notion (M7) |
| `PKS_VAULTS_DIR` | Diretório raiz dos vaults (M1) |
| `PKS_EMBEDDING_PROVIDER` | Provider de embeddings (default: ollama) (M5) |
| `PKS_VECTOR_REMOTE_URL` | URL do repositório Git LFS satélite dedicado a este projeto específico para sync de snapshots vetoriais (Ex: 1 repo de cache para 1 repo de código) |
| `PKS_VAULT_ROOT` | Caminho do vault relativo ao repo root (default: `prometheus`; use `.` para a raiz do projeto como vault) |
| `PKS_GIT_LOG_ENABLED` | Habilita/desabilita append de commits no log diário (default: `true`) |
| `PKS_GIT_ALLOW_PREFIXES` | Prefixos Conventional Commits aceitos no Git Journal, CSV (default: `feat,fix,docs,perf,refactor,arch,test`) |
| `PKS_GIT_MIN_WORDS` | Descarta mensagens com menos de N palavras no Git Journal (default: `5`) |
| `PKS_GIT_IGNORE_AUTHORS` | Autores a ignorar no Git Journal, CSV (ex: `github-actions[bot],dependabot`) |

> **Credenciais Git LFS:** O PKS delega autenticação ao sistema Git do usuário (SSH keys ou tokens configurados via `git credential`). Não há variáveis de credencial proprietárias — o Daemon reutiliza o transporte já autenticado da máquina.

---

---

## MVP O Motor Local Prevalente (Vault + Git Journal + Daemon + BM25 + MCP)

### Objetivo
Entregar um sistema funcional end-to-end: **cada repositorio Git possui seu proprio Obsidian Vault isolado**, com um **Daemon em Rust** rodando continuamente em RAM, indexando commits em tempo real via **Git Journal** e expondo busca BM25 via **MCP**. A busca vetorial (embeddings) e integracao com trackers ficam para a Fase C. O MVP prove valor imediato: um agente de IA com memoria persistente baseada exclusivamente em palavras-chave, sem dependencia de Ollama.

> **Nota editorial:** As features F1.3 a F1.6 (Trackers) são descritas nesta seção por completude do design do produto, mas sua **implementação está planejada para a Fase C**.

### Features

#### F1.1 Estrutura do Vault por Projeto

Em vez de um monólito global de Markdown, cada código-fonte possui uma hierarquia dedicada:

```text
[Qualquer Repo Git]/
├── src/                   # Código fonte da aplicação
└── prometheus/            # O Obsidian Vault deste projeto
    ├── 01-domains/        # Regras de negócio deste serviço
    ├── 02-features/       # Tickets e desenvolvimentos ativos
    ├── 03-testing/        # BDDs e Testcases
    ├── 04-workflows/      # Processos padrão do projeto
    ├── 05-decisions/      # ADRs e Log global do repositório
    └── 90-ai-memory/      # Logs diários brutos de sessão e Git Journal de commits filtrados
```

Essa estrutura privilegia o contexto **humano**: o Antigravity pode buscar qualquer informação pela busca híbrida (BM25 + Vec) do MCP, mas o desenvolvedor humano navega organicamente por pastas com semântica clara.

>  **Convenção, não contrato.** O PKS é **agnóstico à estrutura e à localização do vault**. O indexador parseia qualquer arquivo `.md` encontrado recursivamente a partir da raiz do repositório Git. O vault pode ser `prometheus/` (padrão do `pks init`) ou qualquer diretório configurado via `PKS_VAULT_ROOT`, incluindo `.` para usar a raiz do projeto como vault — independente de hierarquia, nomenclatura ou presença de metadados. A estrutura acima é um exemplo para equipes de engenharia. Um repositório de pesquisa acadêmica pode seguir o padrão Zettelkasten. Um receitário pessoal pode ter só arquivos `YYYY-MM-DD.md` na raiz. O PKS indexa tudo da mesma forma.

#### F1.2  Integração Git Distribuída (A Branch de Conhecimento `pks-knowledge`)

Para evitar a "poluição" visual do histórico primário (`main`/`feature`) com dezenas de commits automáticos de notas ou tickets, o PKS adota a estratégia de branch dedicado:

- O PKS utiliza nativamente as propriedades do Git para manter um branch de serviço paralelo à árvore principal, ex: `pks-knowledge`. A pasta local `prometheus/` é conectada como um `git worktree` atrelada a este branch. Para ocultar a pasta do `git status` sem prejudicar as IDEs, o diretório `prometheus/` é adicionado ao `.git/info/exclude` do repositório pai, e **não** em um `.gitignore` público. Essa distinção cirúrgica garante que a pasta seja invisível ao Git, mas editores como VS Code continuem indexando-a normalmente nas buscas textuais globais, eliminando a fricção de usabilidade (IDEs costumam pular o que está no `.gitignore`).
- **Visibilidade Híbrida e Resolução Otimista de Conflitos:** Para humanos e IDEs (VS Code, Obsidian), os arquivos residem naturalmente na pasta e são facilmente editáveis ao lado do código. Em caso de divergência ou co-edição no Obsidian vs alteração automatizada via Git Worktree, o PKS resolve isto de forma otimista: em nome da Alta Disponibilidade (AP), ele não congela telas com *merge conflicts*. Prioriza-se a última versão viável ou se descarta silenciosamente o commit problemático fazendo "Amortização Destrutiva" (forçando o sync duro local) para restabelecer coerência temporal.
- **Commit Linking:** Quando uma decisão arquitetônica se relaciona especificamente ao código, o PKS armazena o `SHA` do commit que o originou em `main` injetando-o silenciosamente nos metadados yaml do Markdown derivado em `pks-knowledge`.
- **Índice Unificado na RAM:** Para o PKS Daemon, não importa em quais branches os arquivos residem no disco. O Antigravity formula consultas contextuais e o Daemon PKS responde em sub-milissegundo cruzando harmoniosamente a branch atual de código e a branch imutável de conhecimento.

**Riscos conhecidos do Worktree e mitigacoes:**

A estrategia de `git worktree` + `.git/info/exclude` e poderosa mas possui fragilidades praticas que devem ser tratadas com ferramentas dedicadas:

| Risco | Cenario | Mitigacao |
|---|---|---|
| **Worktree orfa** | `git clean -fdx` ou ferramentas de CI podem destruir a worktree silenciosamente | `pks doctor` detecta e repara automaticamente |
| **Interacao com Obsidian** | Obsidian usa file watcher proprio; reset do branch pode corromper `.obsidian/` | `pks init` configura `.obsidian/` no `.gitignore` interno do branch `pks-knowledge` |
| **Onboarding** | `git clone` nao traz o worktree configurado — dev novo nao tem `prometheus/` | `pks init` e obrigatorio apos clone (documentado no README do projeto) |
| **GUI Git clients** | GitKraken, SourceTree, VS Code Git panel podem se confundir com worktrees | Risco aceito; worktrees sao padrao Git oficial desde v2.5 |
| **`.git/info/exclude` e local** | Cada maquina nova precisa reconfigurar; nao viaja com o clone | `pks init` configura automaticamente — o dev so roda 1 comando |

**CLI de Setup e Diagnostico (implementados no M4):**

- **`pks init <path>`**: Configura o repositório para uso com PKS. Cria o branch orphan `pks-knowledge`, configura `git worktree add prometheus/ pks-knowledge`, adiciona `prometheus/` ao `.git/info/exclude`, instala o post-commit hook e cria a estrutura padrão de diretórios (se vazia). O post-commit hook instalado inclui lógica de **Git Journal Append**: filtra commits por prefixo Conventional Commits e faz append em `${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/$(date +%Y-%m-%d)_log.md`.
- **`pks doctor <path>`**: Diagnostica e repara estados degradados. Verifica: worktree existe e aponta pro branch correto? `.git/info/exclude` contém `prometheus/`? Hook post-commit está instalado? Se detectar problema, oferece reparo automático.
- **`list_knowledge_vaults`**: Ferramenta MCP para descoberta de vaults registrados.

#### F1.3  Ingestão de Tracker Externo  `prometheus/` (Import)

Um **Tracker** é qualquer sistema externo que guarda informação estruturada com ID: Notion, Jira, Linear, GitHub Issues, Trello, e até uma planilha. O fluxo `/tracker-to-prometheus` puxa essa informação e a transforma em uma nota Markdown rastreada dentro do repositório local.

**Mecanismo:**
1. O usuário (ou o Antigravity) invoca o workflow com um identificador do tracker.
2. O PKS busca os dados via MCP do tracker correspondente.
3. Gera um arquivo `.md` no diretório correto dentro do `prometheus/` com o conteúdo e metadados de rastreabilidade.
4. O arquivo é commitado automaticamente no branch `pks-knowledge` (ver F1.2), linkando o SHA do commit de origem em `main` nos metadados YAML quando aplicável.

**Exemplos por tipo de repositório:**

| Repositório | Tracker | Resultado no `prometheus/` |
|---|---|---|
| `payment-api/` | Jira: ticket `PAY-4421` (bug de timeout) | `02-features/Checkout/Context_PAY-4421.md` com steps de reprodução e critérios de aceite |
| `meu-tcc/` | Notion: página de orientação com notas do professor | `01-domains/Metodologia/Orientacao_2026-03.md` com todo o feedback estruturado |
| `infra-devops/` | GitHub Issues: issue `#312` (cert expirando) | `05-decisions/Infra/Cert_Renewal_312.md` com prazo, impacto e decisão tomada |
| `e-commerce/` | Linear: ticket `EC-118` (fluxo de Black Friday) | `02-features/Black_Friday/Context_EC-118.md` com critérios de aceite e dependências |

**Metadados gerado automaticamente:**
```yaml
---
tracker_id: PAY-4421
tracker: jira
status: in_progress
tags: [checkout, timeout, backend]
synced_at: 2026-03-06T14:00:00Z
---
```

#### F1.4  Exportação `prometheus/`  Tracker Externo (Export)

O fluxo inverso: uma nota criada localmente (ou refinada com o Antigravity) é publicada de volta no tracker, fechando o ciclo. Ideal para quando a decisão ou documentação matura no Obsidian e precisa ser compartilhada com o time.

**Mecanismo:**
1. O usuario aponta o arquivo `.md` que quer exportar.
2. O PKS le o metadados para saber o tracker destino e a data local `synced_at`.
3. **Collision Detection (OCC):** O PKS checa o `updated_at` no tracker destino. Se a versao na web for *mais nova* que o `synced_at` local (ex: um colega editou o ticket no Jira agorinha), a exportacao falha e alerta o usuario (*Mid-Air Collision* prevenida).
4. Se o caminho estiver livre, publica o conteudo no tracker via MCP correspondente.
5. Atualiza o metadados com o ID gerado e a nova data de sincronizacao (`synced_at`).

**Exemplos por tipo de repositório:**

| Repositório | Arquivo local | Exportado para |
|---|---|---|
| `auth-service/` | `05-decisions/ADR-007_JWT.md` (Architecture Decision Record) | Confluence/Notion como página de documentação oficial |
| `meu-tcc/` | `02-features/Capitulo3_Revisao.md` (versão revisada com AI) | Notion como página para o orientador revisar |
| `e-commerce/` | `02-features/Black_Friday_Spec.md` (especificação escrita no Obsidian) | Linear como ticket novo, já com critérios de aceite |
| `blog-pessoal/` | `01-domains/Ideia_Post_Rust.md` (rascunho) | GitHub Issues como draft de pauta para o backlog público |

#### F1.5  Armazenamento seletivo (o que vai ao `prometheus/`?)

A regra geral: **vai tudo que tem valor de retrieval futuro e que não existe em outro lugar de forma mais adequada.** O `prometheus/` não é um dump — é um destilado. Em M6, isso evolui para uma **Política de Armazenamento Seletivo** centralizada (system-wide) para trackers.

| Repositório | Vai para o `prometheus/` | Não vai |
|---|---|---|
| `payment-api/` | ADR de escolha do gateway, BDDs | Logs de CI, código-fonte bruto |
| `auth-service/` | Sumário de pair-programming | Histórico completo do diálogo bruto |

>  **Princípio:** Se a informação já existe de forma estruturada em outro sistema (tracker, código, banco), o `prometheus/` guarda o **contexto e o raciocínio**, não a informação em si.

#### F1.6  Tracker Sync Queue (FIFO + Produtor/Consumidor)

As operações com trackers externos (APIs do Jira, Notion, Linear) são inerentemente **lentas e falhíveis**: rate limits, timeouts de rede, indisponibilidade. Bloquear o Antigravity ou o Daemon enquanto aguarda a API responder não é aceitável.

A solução é uma **Fila FIFO de Sincronização** independente, com um consumidor dedicado:

```text
 Antigravity / Usuário
        │  enfileira operações
        ▼
  FIFO Tracker Sync
  [ import PAY-4421 ]
  [ import PAY-4422 ]
  [ export ADR-007  ]
        │
        ▼
  CONSUMIDOR (Thread dedicada)
  - Chama API do tracker (Jira, Notion...)
  - Gera/atualiza .md no prometheus/
  - Faz git commit automático
  - Retry com backoff exponencial em falha
        │
        ▼
  Fila 1 do PKS Daemon
  (indexação dos .md gerados)
```

**Pontos-chave:**
-  **Não bloqueia:** O Antigravity enfileira um import e continua trabalhando via Cliente MCP Proxy. O `.md` aparece no `prometheus/` (ou é commitado bare) quando o consumidor terminar.
-  **Lote nativo:** "Importe todos os 30 tickets abertos do sprint" é apenas 30 entradas na fila. O consumidor processa em sequência respeitando rate limits.
-  **Retry automático:** Falha de rede não perde a operação. Volta ao final da fila com backoff exponencial.
-  **Separada do Daemon Central:** A Fila 1 do PKS é sagrada para indexação em RAM dentro do processo em background. A Tracker Sync Queue é uma task assíncrona irmã do Daemon que alimenta essa Fila 1 indiretamente (via commits Git gerados usando a biblioteca nativa `git2-rs`).

---

## Fase C — Inteligência Expandida (Embeddings + Trackers + Shadow Repos)

### Objetivo
Adicionar **busca semântica vetorial** (Ollama), **integração bidirecional com trackers** (Notion, Jira) e **ingestão universal via Shadow Repositories** (Slack, WhatsApp, Email).

### Inspiração: OpenClaw + Prevayler

O [OpenClaw](https://github.com/openclaw/openclaw) introduziu a ideia de organizar a memória de agentes AI em **duas camadas de Markdown locais** (um arquivo curado + logs diários), indexadas por embeddings via Ollama. O PKS absorve essa lógica, mas armazenando os logs em `prometheus/90-ai-memory/YYYY-MM-DD.md` na estrutura do projeto.

Porém, em vez de replicar o motor original do OpenClaw (Python + SQLite vector), o PKS eleva essa ideia usando um **Monolito em Rust baseado no padrão [Prevalência (Prevayler)](https://github.com/prevayler/prevayler)**.

### Referências Core e A Magia da Prevalência

A Prevalência elimina completamente o Gargalo do Banco de Dados: **não há I/O por query e não há parse de linhas para objetos (marshal/unmarshal)**. 
- *Conceito do [Prevayler (Java)](https://prevayler.org/)*: A RAM contém todo o modelo de objetos vivo; a durabilidade é garantida por um Journal append-only de Commands + Snapshots periódicos.

**Por que Rust e não Java (como o Prevayler original)?**
Rust **não tem Garbage Collector**. No Java Prevayler, heaps de 4GB+ sofrem com pausas de GC (stop-the-world) que podem causar latência imprevisível. No Rust, a memória é gerida por ownership/borrowing em tempo de compilação  **zero pausas de GC, zero overhead de runtime**. Isso significa que o limite prático de RAM é o hardware físico da máquina, não o GC. Um vault com 10.000+ notas e milhões de vetores roda sem soluços.

O estado inteiro da busca vive nativamente em RAM. O disco serve *apenas* para durabilidade. Buscar o chunk mais relevante semântico (Cosine SIMD) ou por palavra (BM25 via `tantivy`) é tão rápido quanto iterar sobre um array local  **Microssegundos**.

### Arquitetura do PKS (Daemon MCP Federado)

Para garantir o benefício máximo da Prevalência (busca na casa dos microssegundos), **o estado deve permanecer continuamente vivo em RAM**.
O diferencial da nova arquitetura federada: **Um único daemon escuta e unifica inúmeros repositórios ao mesmo tempo**.

```text
    Repo GIT Local (Proj A)        Repo Remoto (Proj B, ex: GitHub API)
        (Vault A)                         (Vault B)
            │                                 │
     (Hook + OS FS Events)          (Clone local + OS FS Events)
            ▼                                 ▼
   ┌────────────────────────────────────────────────────────┐
   │             FILA 1 (Transações Brutas)                 │
   └────────────────────────┬───────────────────────────────┘
                            │ Double-Buffered Pipeline
   ┌────────────────────────▼───────────────────────────────┐
   │             CONSUMIDOR (Thread BG)                     │
   │ - Parseia diffs  - Identifica Origem (Projeto)         │
   │ - Chunking       - Chama Ollama                        │
   └────────────────────────┬───────────────────────────────┘
                            ▼
               FILA 2 (Mutações Prontas)
                            │ Swap Atômico (NYC Style)
   ┌────────────────────────▼───────────────────────────────┐
   │          PKS Daemon / Serviço Central (Rust)           │
   │          [ VIVO 100% DO TEMPO EM RAM ]                 │
   │  - Índices Unificados: Projeto A + Projeto B           │
   └─────────────┬──────────────────────────────────────────┘
                 │ Unix Domain Socket (IPC)
                 ▼
   ┌──────────────────────────┐
   │  pks --stdio (Proxy MCP) │  <-- Processo leve, repassa chamadas e 
   │  [ INICIADO PELA IDE ]   │      executa ferramentas de host no shell
   └─────────────┬────────────┘
                 │ Model Context Protocol
                 │ `search_knowledge_vault(query, filters=[ProjA])`
                 ▼
          ┌──────────────┐
          │ Antigravity  │  <-- ("Quero usar um padrão do Proj B
          │ Claude Code  │       para refatorar o Proj A agora")
          └──────────────┘
```

### Modelo de Dados (High-Level)

Para um sistema Prevalente onde a RAM e o banco de dados, o schema das structs e a definicao central da arquitetura. Abaixo, o modelo conceitual do estado mantido em memoria pelo Daemon:

```text
PrevalentState
├── repos: HashMap<RepoId, RepoIndex>
│   ├── RepoId: PathBuf (caminho absoluto do .git comum, via git rev-parse --git-common-dir)
│   └── RepoIndex
│       ├── status: Cold | Warm | Hibernated
│       ├── head_commit: String (SHA da HEAD indexada)
│       ├── bm25_index: tantivy::Index
│       ├── chunks: Vec<Chunk>
│       │   └── Chunk
│       │       ├── embedding: Option<Vec<f32>> (None se pendente)
│       │       └── chunk_hash: [u8; 32] (SHA-256 p/ dedup)
│       └── tombstones: HashSet<String> (file_paths deletados)
│
├── vector_clock: HashMap<RepoId, String> (vector clock por repo/branch)
│
├── embedding_debt: VecDeque<PendingEmbedding> (Dívida de Ollama em RAM)
│
└── global_stats
    ├── total_vectors: u64
    ├── pks_tracker_sync_queue_depth: u64
    └── pks_embedding_debt_entries: u64
```

**Notas sobre o modelo:**
- O `RepoId` é derivado do caminho absoluto retornado por `git rev-parse --git-common-dir`. Múltiplas worktrees do mesmo repositório compartilham o mesmo `RepoId`. Renomear a pasta da worktree não afeta a identidade do projeto.
- O campo `embedding: Option<Vec<f32>>` permite que chunks existam no indice BM25 antes da vetorizacao pelo Ollama (degradacao gradual).
- O `vector_clock` e serializado junto ao snapshot `bincode` e usado na reidratacao para determinar se a HEAD atual avancou alem do snapshot.
- O `embedding_debt` vive em RAM durante a operacao normal e e serializado para `embedding_debt.jsonl` apenas em shutdown gracioso ou quando o limite de 50MB e atingido.

### Features Técnicas

#### F2.1  A Transação Prevalente via Git

O próprio **Git** garante a durabilidade imutável e ordenada do sistema (Append-only Journal).

**Estratégia de detecção de mudanças (hierarquia unificada):**

O PKS usa três mecanismos complementares, cada um com escopo distinto:

| Mecanismo | Escopo | Função |
|---|---|---|
| **Post-commit hook** (primário) | Detecção de commits | Notifica o Daemon instantaneamente após cada `git commit`. |
| **OS FS Events** (fallback otimizado) | Detecção de mudanças na `HEAD` | Monitora mudanças no arquivo `.git/refs/heads/` nativamente via `fsevents`/`inotify`. Captura `pull`, `merge`, `rebase` e `amend` com zero overhead de CPU e sem polling agressivo. |
| **FS watch** (`notify`) | Registro/desregistro de repos | Monitora `~/pks-vaults/` para detectar `git clone` (novo `.git/`) ou `rm -rf` (remoção de repo). Não é usado para detecção de commits individuais. |

> **Idempotência e Debounce na Entrada (Resolvendo Double-Firing):** Dado que o hook de *post-commit* e o FS Event no `.git/refs/heads/` disparam em milissegundos quase coincidentemente na maioria das operações do desenvolvedor, a Fila 1 implanta uma trava de **Debounce** acoplada a um cache rápido. Se a mesma transação (mesmo *Commit SHA* ou *Tree Hash*) bater na porta duas vezes numa janela de curta duração, ela é considerada duplicada e descartada (idempotência), eliminando engasgos desnecessários do `Double-Firing`.

- O Daemon **não faz restart nem congela** para processar o commit. Ele age instantaneamente, empurrando a notificação para a **Fila 1 (Transações Brutas)**.
- **Apuracao precisa de Diffs (Git-Native):** O Daemon nunca varre o disco às cegas tentando descobrir o que mudou. Ao acordar a Fila 1, ele invoca nativamente a biblioteca `git2-rs` executando um `git diff-tree -r HEAD <Vector_Clock>`. O próprio Git diz estruturalmente quais arquivos foram Modificados, Adicionados ou Deletados (Tombstones). Apenas este delta microscópico sofre o parsing.
- O **estado em RAM do Daemon acompanha as HEADs relevantes de cada repositório Git**: tanto o branch de trabalho ativo (`main`, `feature/*`) quanto o branch de conhecimento `pks-knowledge` (ver F1.2). O OS FS Events em `.git/refs/heads/` captura mudanças em ambos. Focar nas HEADs garante resiliência imediata contra reescritas da história (`git rebase`, `git commit --amend`, `git push --force`).
- **Snapshot**: O cache vetorial pesado é serializado periodicamente em `snapshots/<repo_id>.bin` (formato bincode segmentado por repo, ver D12) para cold start rápido e sem picos de RAM. Se um snapshot corromper, o sistema apenas relê os `.md` puros daquele repositório.
- Granularidade Atômica: Um commit com 50 arquivos modificados entra inteiro como uma única transação no pipeline.

#### F2.2  Daemon Contínuo + Servidor MCP Federado

O PKS roda como um **serviço de sistema operacional** (`launchd` no macOS, `systemd` no Linux).
- Mantém Múltiplos índices agnósticos de repositório vivos em RAM.
- É acessado internamente por um **Proxy MCP leve** (`pks --stdio`), configurado no `.mcp.json` do projeto como `"type": "stdio"`. O Proxy não carrega os índices — ele atua apenas como intermediário JSON-RPC, conectando-se ao Daemon via Unix Domain Socket (`/tmp/pks.sock` ou Named Pipe seguro no Windows) para repassar queries de leitura enquanto blinda chamadas de execução OS dentro da própria hierarquia do Proxy.

**Ferramentas expostas:** 
1. `list_knowledge_vaults()`: Retorna a lista de todos os `RepoId` registrados e WARM no Daemon. Essencial para o LLM "olhar ao redor" e descobrir o contexto da maquina antes de engajar buscas estruturadas (ex: descobre que o dev tem `payment-api` e `auth-service` ativos).
2. `search_knowledge_vault(query: str, top_k: int, projects_filter: list[str] = None)`
   - A RAM ja tem os vetores de N projetos carregados.
   - O LLM formula queries: `search_knowledge_vault("padrao de retry", projects_filter=["payment-api", "auth-service"])`.
   - O Daemon responde instantaneamente via SIMD Cosine + Tantivy. Se a LLM pedir consulta de um projeto B enquanto esta mexendo no projeto A, a ferramenta centraliza a descoberta de arquiteturas paralelas.

#### F2.3  Double-Buffered Pipeline (Modelo NYC-Style Refinado)

O PKS usa **duas filas FIFO** para garantir que a thread principal de queries **nunca congele** durante indexação:

```text
  Git Commit ───▶ FILA 1 (Transações Brutas)
                     │
                     ▼
              ┌─────────────────────────────┐
              │  CONSUMIDOR (Thread BG)     │
              │  - Parse do diff            │
              │  - Chunking 400t/80t        │
              │  - Chama Ollama (embeds)    │
              │  - Monta mutação BM25 pronta│
              └─────────────┬───────────────┘
                            │
                            ▼
                 FILA 2 (Mutações Prontas)
                            │
                   ┌────────▼───────────────────┐
                   │  THREAD PRINCIPAL (NYC)    │
                   │  - Query? Responde <1ms    │
                   │  - Idle? Aplica Fila 2     │
                   │    atômicamente no índice  │
                   └────────────────────────────┘
```

**Como funciona:**
1. **Fila 1 (Transações Brutas):** O hook do Git empurra diffs para cá. Sem processamento pesado.
2. **Consumidor (Thread Background):** Consome a Fila 1, parseia os `.md`, faz chunking, chama o Ollama para gerar vetores, e monta "pacotes de mutação" completos (chunk + vetor + entrada BM25 pronta). Empurra para a Fila 2.
3. **Fila 2 (Mutações Prontas):** Contém mutações já processadas, prontas para swap atômico no índice.
4. **Thread Principal (NYC-Style):** Processa **queries em série** (<1ms cada). Durante **janelas de ociosidade** (quando não há query pendente), consome a Fila 2 e aplica as mutações atomicamente no índice vivo. Sem locks, sem RwLock, sem contention.

**Configurações de Throttle e Hardware:**
- O Consumidor BG possui limites de ingestão estritos (Throttle) configuráveis para operar de maneira silenciosa sem degradar a máquina principal do usuário em picos longos.
- `PKS_THROTTLE_MS=200`: Pausa de respiro entre chamadas de vetorização (pesadas) pro Ollama. A vetorização profunda em background priorizará as frestas inativas (modo *idle*) e suportará escalabilidade remota otimizada em roadmap estendido (ver *Apêndice C* para a visão futura sobre Offload de CPU e Gestão Elétrica).

**Backpressure e limites das filas:**
- **Fila 1:** Limite de `PKS_FILA1_MAX=1000` transações brutas. Se saturada, tolerância à perda — a transação é descartada do buffer. Recuperável perfeitamente pela validação do Vector Clock subsequente ou reconciliação de `HEAD`.
- **Fila 2:** Limite de `PKS_FILA2_MAX=500` mutações prontas. Se atingido, o Consumidor BG aguarda as limpezas atômicas darem vazão pelas janelas disponíveis.
- **Ollama offline e Dívida Técnica (Debt):** Chunks aguardando vetorização acumulam na sub-fila `embedding_backlog`. Se houver transbordo longo, os itens não são simplesmente descartados. O Daemon exila e serializa essas tarefas pendentes num registro leve em disco (ex: `embedding_debt.jsonl`). Quando o Ollama retorna, o PKS re-ingere essa "dívida vetorial" silenciosamente em background, curando as áreas cegas do índice (auto-healing) sem exigir o disparo manual de um `pks validate`.

**Resultado sem Locks (Otimismo e Auto-recuperação):** A thread de RAG para queries **nunca usa mutexes bloqueantes estritos**. A ocorrência rara de inconsistências temporárias durante um swap atômico do índice em RAM é abordada com tolerância pragmática: A pura Disponibilidade (availability sub-milissegundo para o Agente) suplanta eventuais inconsistências (AP do teorema CAP) momentâneas no dado retornado. Reparos cirúrgicos ou desvios ocorrem com atualizações corretivas assíncronas no decorrer do diário atômico do Rust ou por expurgos orientados (pks sync).

#### F2.4  Estratégia de Chunking

A qualidade do retrieval depende diretamente de como os documentos Markdown são segmentados. O PKS adota uma estratégia de **chunking semântico por heading** com fallback por sliding window:

1. **Split primário por headings:** Cada seção (`##`, `###`) vira um chunk independente. Isso preserva a unidade semântica natural dos documentos Markdown.
2. **Sliding window para seções longas:** Se uma seção exceder 400 tokens, ela é subdividida em chunks de 400 tokens com overlap de 80 tokens para manter continuidade contextual entre fragmentos.
3. **Seções curtas são agrupadas:** Seções com menos de 100 tokens são concatenadas com a seção seguinte para evitar chunks com contexto insuficiente para gerar embeddings de qualidade.
4. **Deduplicação por Hash Restrito (Economia de Ollama e CPU):** Para evitar que um arquivo longo seja integralmente revetorizado pelo Ollama quando o humano digita apenas uma vírgula num trecho semântico distante, a camada de parsing extrai um hash nativo (`SHA-256`) a nível do parágrafo fragmentado. Apenas e tão somente os blocos exatos que demonstrem alteração no seu próprio hash irão atritar processamentos novos contra o LLM, enquanto todo restante herda os vetores estáticos, poupando ciclos cruciais.
5. **Metadados preservados:** Cada chunk carrega `(repo_id, file_path, heading_hierarchy, chunk_index, chunk_hash)` para rastreabilidade e para que o LLM saiba de onde veio a informação.
6. **Garbage Collection Ativa e Compactação (Tombstones):** Se a `HEAD` mudar deletando um arquivo `.md` (ou renomeando-o), o parser gera um evento de "Tombstone". A Thread Principal varre a RAM e ejeta instantaneamente todos os chunks associados àquele `file_path`. Para evitar que metadados fantasmas inchem o disco ao longo de meses, o ato de salvar o `snapshots/<repo_id>.bin` executa um processo de **Compactação**, expurgando os Tombstones permanentemente. Sem vazamento de RAM ou disco ao longo da vida do projeto.

**Parâmetros configuráveis:**
- `PKS_CHUNK_MAX_TOKENS=400` (tamanho máximo do chunk)
- `PKS_CHUNK_OVERLAP_TOKENS=80` (overlap entre chunks de sliding window)
- `PKS_CHUNK_MIN_TOKENS=100` (tamanho mínimo antes de agrupar)

#### F2.5  Tolerância a Falhas e Recuperação

O Obsidian vault é sempre a **fonte da verdade**. O índice do Daemon é derivado.

| Falha/Evento | Recuperação |
|---|---|
| Reboot da máquina | Carrega snapshots `bincode` (lazy). |
| Snapshot corrompido | Deleta o binário e inicia reidratação total do Git (M3). |
| Ollama indisponível | Modo **Degradação Gradual** (ver abaixo). |
| Rebase/Amend | Reindexação automática (Drop & Rebuild) se a causalidade for rompida. |

**Degradação Gradual do Ollama (5 Estados):**

O PKS opera de forma resiliente conforme a disponibilidade do serviço de embeddings:

| Estado | Comportamento no PKS | Capacidade |
|---|---|---|
| **Ausente/Fail** | Daemon inicia em modo BM25-only. | Busca por palavras-chave (100% OK) |
| **Model Pending** | Tenta `ollama pull` uma vez; falha -> BM25-only. | BM25-only |
| **Temp Offline** | Chunks vão para `embedding_debt` em RAM. Auto-healing no retorno. | Híbrida parcial |
| **Dívida Longa** | Se dívida > 50MB, pausa acumulação, log `ERROR`. | BM25 p/ novos; Híb. p/ antigos |
| **Retorno/Recovery** | Re-ingere dívida com throttle (`PKS_THROTTLE_MS`). | Convergência gradual p/ híb. total |

> **Principio:** A ausencia do Ollama **nunca** impede o Daemon de iniciar, responder queries ou indexar commits. A busca vetorial e um upgrade incremental sobre o BM25, nao um pre-requisito.

#### F2.6  Memória Reflexiva Global

Para cada projeto:
- O Antigravity sumariza as decisões efêmeras locais em `prometheus/90-ai-memory/YYYY-MM-DD.md` no encerramento da sessão.
- O resumo é commitado no branch `pks-knowledge` (ver F1.2), com referência ao SHA do último commit de trabalho da sessão nos metadados YAML. Isso garante rastreabilidade bidirecional: do código para o contexto e do contexto para o código.
- O histórico de conhecimento viaja junto com o repositório Git (qualquer `git clone` ou `git fetch` traz o branch `pks-knowledge`), tornando a memória onipresente.

**Git Journal Append (extensão do T4.4):**
O post-commit hook (T4.4) é estendido para registrar commits relevantes diretamente nos logs de memória reflexiva:
- Filtra commits por prefixo Conventional Commits (`PKS_GIT_ALLOW_PREFIXES`) e comprimento mínimo (`PKS_GIT_MIN_WORDS`)
- Faz append de uma linha formatada em `${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/$(date +%Y-%m-%d)_log.md`
- Formato de cada entrada: `- **HH:MM** - \`<sha>\` - <autor>: <mensagem>`
- O FSWatcher do Daemon detecta a modificação e re-indexa o arquivo automaticamente
- Commits do branch `pks-knowledge` são ignorados para evitar recursão

#### F2.7  Indexação Multi-Repositório Distribuída

O indexador atua como um hub para "Clusters de Obsidian".
- Suporta **Repositórios Locais**: Referência via `file://`, onde o Daemon usa a hierarquia de detecção definida em F2.1 (post-commit hook + OS FS Events + FS watch para registro).
- Suporta **Repositórios Remotos**: O daemon pode monitorar repos via API do GitHub/GitLab (ou clone puro via SSH) para engolir repositórios distantes da organização.
- Permite "Cross-pollination" da IA: Quando trabalhando no *Projeto B*, o Agente pode buscar e aplicar aprendizados ou lógicas previamente decodificadas do *Projeto A*, pois todos os clusters estão carregados homogeneamente na RAM do mesmo daemon.

#### F2.8  O PKS Federado no Teorema CAP

Tratando-se de um sistema distribuído onde a "Verdade" vive particionada em N repositórios Git distintos e é consolidada na RAM do PKS Daemon, o sistema se posiciona clara e intencionalmente como **AP (Availability and Partition Tolerance)** no Teorema CAP:

- **Partition Tolerance (Tolerância a Partição) - FORTE:** Se um repositório remoto cair, ou o GitHub ficar fora do ar, ou a rede falhar ao tentar buscar diffs de um Vault de outro projeto, o Daemon continua vivo respondendo perfeitamente para todos os outros repositórios locais ou os clusters já cacheados em RAM.
- **Availability (Disponibilidade) - FORTE:** Graças ao design *Double-Buffered Pipeline* e *Single-Thread NYC-Style*, o nó MCP garante disponibilidade 100% do tempo. Uma query do Antigravity **sempre** retornará o estado mais recente que o Daemon tem conhecimento em sub-milissegundo, sem nunca bloquear esperando um parse remoto.
- **Consistency (Consistência) - EVENTUAL OTIMISTA:** O índice do PKS **não é a fonte absoluta da verdade** (o Git/FS hospedeiro e o Tracker real são). Em uma visão determinística resiliente, o sistema aceita corriqueiramente pequenos atrasos ou defasagens em RAM. Se dois pushes opostos em repositórios assinados conflitarem com atraso global num Webhook, a IA será alimentada provisoriamente por dados minimamente passados antes de toda a Fila 1 digerir, atuar no `Ollama` e atualizar na RAM da máquina. Havendo alguma degeneração grave em algum *branch pointer*, a chave respectiva será ejetada silenciosamente pela rotina e regenerada posteriormente, muito barata pelo processamento indexador do Rust.

Tratando sistemas de RAG complexos em Software, o lado **AP** é infinitamente superior ao dogmatismo **CP**. Entender isso liberta restrições: O Agente de IA aceita a pequena defasagem de minutos pacificamente perante travas violentas, trancamentos de RAM ou a indisponibilidade total do kernel aguardando rotinas de um repositório inatingível distante.

#### F2.9  Event Sourcing Multi-Journal e Ordenação Causal

O pressuposto de sistemas Prevalentes puros (à la Prevayler ou Event Sourcing) é garantir **Determinismo**: iniciar da RAM vazia, re-rodar o diário inteiro (Journal), e obrigatoriamente chegar no *mesmo exato estado* byte-for-byte.

Quando o PKS vira "Multi-Repositório", ele perde o luxo de ter 1 único diário linear sequencial. Se o *Repositório A* recebe um commit às 10:00 e o *Repositório B* também às 10:00, em qual ordem o Daemon as reidrata ao reboot?

**A Solução de Sincronização Distribuída:**

1. **Namespacing do Índice em RAM:** A struct primária particiona hashes e vetores agrupados por `(Repo_ID, Path)`. Commits do Projeto A não colidem com `readme.md` do Projeto B. Sem merge conflicts transversais entre repositórios.
2. **Commit Hash como Logical Clock:** O Consumidor da Fila 1 estampa nas mutações PRONTAS (Fila 2) uma tupla global: `(Timestamp_Ingestão_Daemon, Repo_ID, Commit_SHA)`. A ordem de aplicação no índice é determinística dentro de cada repositório; a ordem *entre* repositórios é irrelevante (eles são namespaced).
3. **Vector Clock (O Marca-Páginas Múltiplo):** Em um banco clássico, você faria backup do banco todo num timestamp X. No PKS, o estado é derivado do Git. Quando o Daemon grava os snapshots segmentados (ex: ao meio-dia), ele precisa saber exatamente **em qual commit de cada repositório** aquela "foto" da RAM foi tirada.

   Um **Vector Clock** aqui é simplesmente um dicionário que diz: *"Este snapshot representa exatamente o estado onde o Repositório A estava no commit X, e o B estava no commit Y"*.

   **Exemplo didático do Vector Clock salvo no Snapshot:**
   ```json
   {
      "payment-api": "commit-8f92bd",
      "auth-service": "commit-92ca31",
      "receitas-da-mae": "commit-00cd2a"
   }
   ```

4. **Reidratação Tolerante no Reboot:** Se a máquina reiniciar ou o Daemon cair, ao subir, ele carrega o snapshot para a RAM. Se a versão compilada do Daemon for diferente do "Magic Header Version" daquele Snapshot, ele o descarta. Reconstruirão-se as tabelas com base na `HEAD` vigente.
   - **Tolerância a Reescrita (Rebase/Amend):** Rebases destroem as referências passadas de uma árvore e reescrevem seus pais. Quando o PKS depara-se com essa ruptura fatal, a premissa de Single User e priorização de sanidade vence: "Rebase = Reindex". A árvore corrompida referenciada pelo snapshot é expurgada e todo o repositório sofre reenfileiramento desde o zero com base no commit da HEAD. Operação rápida pra BM25, assíncrona tolerante pra vetores.

#### F2.10  Gestao de Memoria: Carga e Descarga Manual de Projetos

Na versao inicial, a gestao de memoria e **deliberadamente manual**. O desenvolvedor controla quais projetos estao ativos no Daemon pela presenca ou ausencia do repositorio no diretorio monitorado (`~/pks-vaults/`).

**Principio:** Clone = Carregar. Delete = Descarregar. Sem magica, sem surpresas.

**Ciclo de vida de um repositorio no Daemon (MVP):**

```text
 AUSENTE (nao existe em ~/pks-vaults/)
      | git clone <url> ~/pks-vaults/proj-x
      v
  CARREGADO (Indices BM25 vivos em RAM)
  - Daemon detecta novo .git/ via FS watch
  - Faz parse + chunking + indexacao BM25
  - Queries respondem em sub-ms
       | rm -rf ~/pks-vaults/proj-x
       v
 DESCARREGADO
  - Daemon detecta remocao do .git/
  - Purga a particao da RAM e do snapshot
```

**Implicacoes praticas:**
- Com 5-10 projetos ativos, a RAM consumida e trivial (indice BM25 puro: dezenas de KB por 1000 notas).
- Se a maquina estiver pressionada, o desenvolvedor simplesmente remove o projeto menos usado de `~/pks-vaults/`. O Daemon libera a RAM instantaneamente.
- Nao ha LRU automatico, watermarks, nem hibernacao nesta versao. O humano e o gerenciador de memoria.

> **Evolucao futura:** Para cenarios com dezenas de projetos simultaneos, o Apendice C descreve a visao de **HOT RAM / COLD INDEX** — uma politica automatica de eviccionamento baseada em LRU + watermarks de vetores que promove e rebaixa repositorios entre RAM quente e disco frio sem intervencao humana.

#### ~~F2.11~~ **[SUBSTITUÍDA — ver STEERING_remove_fswatcher.md]** Filesystem-as-Config (Clone = Registrar, Delete = Desregistrar)

O PKS adota o **filesystem local como fonte de verdade de configuração**  sem arquivos `.toml`, sem painel de controle, sem CLI de registro.

O Daemon monitora um diretório raiz configurável (ex: `~/pks-vaults/`). Qualquer repositório Git que aparecer nessa pasta é automaticamente integrado ao índice unificado. Qualquer repositório deletado é automaticamente purgado.

> **Nomenclatura:** `~/pks-vaults/` é o diretório raiz onde repositórios Git são clonados para serem monitorados pelo Daemon. Não confundir com `prometheus/`, que é a pasta *dentro* de cada repositório onde residem as notas Markdown do vault Obsidian (ver F1.1).

```text
~/pks-vaults/
├── payment-api/          ← PKS indexa (WARM)
├── auth-service/         ← PKS indexa (WARM)
├── frontend-app/         ← PKS indexa (WARM)
└── old-monolith/         ← PKS indexa (WARM)
```

**Fluxo de vida:**

| Ação do usuário | O que o PKS faz |
|---|---|
| `git clone <url> ~/pks-vaults/proj-x` | FS watch detecta novo diretório `.git`, faz parse + chunking + indexação BM25. Repo fica WARM. |
| Trabalha ativamente no projeto | Repo permanece WARM; post-commit hook + OS FS Events atualizam o índice (ver F2.1) |
| `rm -rf ~/pks-vaults/proj-x` | FS watch detecta remoção do `.git`, purga a partição `proj-x` da RAM e do snapshot |

**Vantagens:**
-  **Zero burocracia:** O Terminal e o Finder são a interface de administração do PKS.
-  **Sem estado oculto:** O que está em `~/pks-vaults/` é exatamente o que o Daemon conhece. Sempre.
-  **Fluxo natural:** Desenvolvedores já clonam repos. Agora clonar também dá memória à IA automaticamente.

#### F2.12  Segurança e Controle de Acesso

O PKS roda localmente como daemon de um único usuário (ou máquina de time). Ainda assim, medidas de segurança são necessárias:

**Acesso ao MCP Server:**
- Escopo totalmente voltado a **Single Player** e **Uso Próprio** neste momento.
- O servidor MCP opera via transporte stdio (`pks --stdio`), invocado pelo cliente MCP (Claude Code). Sem porta de rede exposta — comunicação é por stdin/stdout do processo.
- Auth Token em headers, TLS/CORS restritos, ou isolamento multi-tenant ficam explicitamente postergados (Tech Debt Consciente) até o sistema obter maturidade ou expansão para uso corporativo em time.

**Credenciais de trackers (Fase 1):**
- Credenciais de APIs externas (Jira, Notion, Linear) são armazenadas via variáveis de ambiente ou keychain do sistema operacional (`security` no macOS, `secret-tool` no Linux). Nunca em arquivos de configuração versionados no Git.

**Sanitização de conteúdo importado:**
- Conteúdo importado de trackers passa por sanitização de Markdown antes do commit: remoção de HTML inline, scripts embutidos e links potencialmente maliciosos. O arquivo `.md` resultante contém apenas Markdown puro.

**Segurança do Parser Markdown (Prevenção de ReDoS):**
- Para evitar interrupções catastróficas ou travamentos propositais da Thread de Background gerando 100% de CPU infinito, o PKS recruta obrigatoriamente Crate Parsers rigorosos contra *Regular Expression Denial of Service* (ReDoS), a exemplo da `pulldown-cmark`. Adicionam-se defesas mecânicas como "Timeout máximo por varredura" e "Limite de aninhamento sintático", mitigando a invasão paralela de inputs intencionalmente caóticos.

#### F2.13  Observabilidade

Um daemon 24/7 exige visibilidade operacional. O PKS expõe:

**Logs estruturados:**
- Formato JSON via `tracing` (Rust). Níveis: `ERROR`, `WARN`, `INFO`, `DEBUG`.
- Rotação automática por tamanho (`PKS_LOG_MAX_SIZE=50MB`) em `~/.pks/logs/`.
- Eventos-chave logados: commit detectado, chunks processados, queries respondidas, erros de Ollama, hibernação/wake de repos.

**Health check endpoint:**
- `GET http://localhost:3030/health` retorna status do daemon, repos registrados, tamanho das filas, e uptime.

**Métricas expostas:**
| Métrica | Descrição |
|---|---|
| `pks_fila1_depth` | Itens pendentes na Fila 1 (Transações Brutas) |
| `pks_fila2_depth` | Itens pendentes na Fila 2 (Mutações Prontas) |
| `pks_tracker_sync_queue_depth` | Operações de Import/Export pendentes (M6) |
| `pks_embedding_debt_entries` | Chunks aguardando vetorização Ollama (M5) |
| `pks_ollama_queue_depth` | Chunks na fila ativa de vetorização Ollama (M5) |
| `pks_query_latency_us` | Latência de queries em microsegundos (p50, p95, p99) |
| `pks_repos_warm` | Quantidade de repositórios em estado WARM |
| `pks_last_commit_indexed` | SHA do último commit indexado por repositório |
| `pks_repos_hibernated` | Quantidade de repositórios em estado HIBERNATED (M5) |
| `pks_ram_usage_bytes` | Consumo de RAM total do Daemon (índices + vetores) |

**CLI de diagnóstico:**
- `pks status` — resumo de repos, estado das filas, saúde do Ollama.
- `pks validate` — compara hashes SHA256 dos `.md` com o índice.

#### F2.14  Contingência (Disaster Recovery)

Como o índice vive localmente e os embeddings semânticos exigem tempo de computação da máquina local (Ollama CPU/GPU), formatar um MacBook ou trocar de desenvolvedor geraria a necessidade de refazer milhares de vetores.

Como lidar com a contingência se a máquina queimar ou houver um crash fatal de disco?

1. **A Fonte da Verdade já é Global:** O próprio repositório Git remoto hospedado (GitHub/GitLab) é o backup absoluto e imutável do conhecimento (pasta `prometheus/`). A perda total do cache do PKS nunca resulta em perda de contexto intelectual.
2. **Snapshots via Git LFS Sub-Repository (Isolamento 1:1):** Para garantir segurança e isolamento de contexto, o PKS gerencia de forma autônoma um repositório Git satélite exclusivo para cada projeto (ex: `projeto-x-pks-cache.git`). Este repositório é configurado com `.gitattributes` forçando `*.bin filter=lfs`. O Daemon isola o cache vetorial nesse segundo repositório para evitar inchar a árvore de código principal e respeitar as permissões de acesso originais do projeto. Periodicamente, o Daemon executa `git add snapshots/*.bin`, `git commit --amend` (ou via orphan branches para poupar storage LFS), e `git push --force` no sub-repo correspondente. Na máquina nova, basta um `git clone` do sub-repo de cache para trazer os embeddings via Git LFS automaticamente, reidratando o índice em segundos sem re-vetorização. O mecanismo alavanca as cotas de LFS dos provedores de hospedagem (GitHub, GitLab, Bitbucket) já utilizados pela equipe.

**Verificação de Cota LFS:** O Daemon estará preparado para erros `429` (cota LFS excedida no provedor). Nesse cenário, executa *degradação graciosa*: fallback para computar o RAG usando apenas CPU local, sem conseguir baixar/escrever os `.bin` via git-lfs. Quando a cota é liberada, o sync retoma automaticamente.

**Compressão configurável:** Antes do push LFS, snapshots podem ser comprimidos com zstd via variável `PKS_BACKUP_COMPRESS` (default: `false`). Quando habilitada, reduz significativamente o tamanho dos `.bin` transferidos, economizando cota LFS.

**Reescrita Bruta (Amortização):** Como o LFS no sub-repo pode encarecer a longo prazo com histórico acumulado, o PKS usa táticas agressivas de amortização — *squash* ou *orphan branches* que destroem histórico no repositório LFS satélite — mantendo apenas o `.bin` mais recente vivo na nuvem do provedor, sem empilhar bytes cobrados.

---

## Abordagem TDD

O desenvolvimento do PKS seguirá **Test-Driven Development** rigoroso:

| Camada | Framework | O que testar |
|---|---|---|
| Unitários (Rust) | `cargo test` | Chunking, parsing MD, SIMD cosine, BM25 scoring, serialização snapshot |
| Integração | `cargo test --features integration` | Pipeline completo: .md -> chunk -> vetor -> query -> resultado |
| Golden Dataset | Arquivos `.md` de referência | Dataset fixo de 50 notas + queries esperadas com scores mínimos |
| MCP E2E | Cliente MCP mock | Enviar `search_knowledge_vault` e validar resposta JSON/markdown |

**Ciclo RED-GREEN-REFACTOR**: Cada feature nova começa pelo teste que falha.

---

## Estrutura do Monolito (Repositório)

```text
pks/                          # O binário em Rust
 ├── src/
 │  ├── main.rs               # Daemon contínuo + launchd/systemd lifecycle
 │  ├── cli.rs                # CLI de diagnóstico (pks status, pks validate)
 │  ├── mcp_server.rs         # Servidor MCP via stdio (pks --stdio)
 │  ├── auth.rs               # Autenticação e controle de acesso (F2.12)
 │  ├── state.rs              # PrevalentState (RAM: index struct)
 │  ├── snapshot.rs           # Serialização/deserialização bincode segmentada (snapshots/<repo_id>.bin)
 │  ├── git_journal.rs        # Git Hook Tracker via git2-rs + OS FS Events fallback
 │  ├── debounce.rs           # Idempotência e deduplicação de eventos (SHA cache + janela temporal)
 │  ├── repo_watcher.rs       # FS watch para registro/desregistro de repos [REMOVIDO — STEERING_remove_fswatcher.md]
 │  ├── memory_manager.rs     # Gestão de repos em RAM — manual no MVP, LRU futuro (Apêndice C)
 │  ├── fifo_embedder.rs      # Fila FIFO assíncrona para vetorização via Ollama
 │  ├── observability.rs      # Logs estruturados, métricas, health check (F2.13)
 │  ├── indexer/
 │  │  ├── pipeline.rs        # Parsing MD, SHA256 dedup
 │  │  └── chunker.rs         # Chunking semântico por heading + sliding window (400t/80t overlap)
 │  └── search/
 │      └── retriever.rs      # Hybrid search (SIMD Cosine + Tantivy BM25)
 ├── tests/
 │  ├── golden_dataset/       # 50 notas .md de referência
 │  ├── test_chunking.rs
 │  ├── test_search.rs
 │  ├── test_backpressure.rs  # Testes de limites de fila e backpressure
 │  └── test_mcp_e2e.rs
 └── Cargo.toml               # Dependências
```

---

## Roteiro de Implementacao (Milestones)

A execucao deste sistema nao se prendera a burocracia de Sprints temporais fixos. O MVP entrega valor imediato com busca BM25 pura (sem dependencia de Ollama). A Fase 2 adiciona camadas incrementais de inteligencia na ordem: embeddings vetoriais, trackers externos, e ingestao universal via Shadow Repos.

```text
FASE A: Motor Prevalente de RAG (M1, M2, M3) ✅ 2026-03-01
   │
   ├─ M1: A Engine Rústica ✅
   │    F2.2 Servidor MCP stdio (Mock/Fixos) ✅
   │    F2.4 Estratégia de Chunking (Parse MD) ✅
   │    Recuperação por Palavras-chave (BM25 Tantivy) ✅
   │    ~~F2.11~~ **[SUBSTITUÍDA por `pks refresh`]** Filesystem-as-Config (Vault discovery) ✅
   │    F2.2 ext `list_knowledge_vaults` ✅
   │
   ├─ M2: Pipeline NYC-Style ✅
   │    F2.3 Double-Buffered Pipeline (BM25-only) ✅
   │    F1.5 Armazenamento Seletivo (Policy básica) ✅
   │
   └─ M3: Persistência e Resiliência ✅
        F2.9 Event Sourcing Local / Vector Clock ✅
        D12 Serialização de Snapshots segmentados ✅
        F2.5 Auto-healing (BM25 recovery) ✅

FASE B: Git Journaling & Vaults (M4) ✅ 2026-03-05
   │
   └─ M4: Conexão com o Hospedeiro ✅
        F1.1 Estrutura do vault (prometheus/) ✅
        F1.2 Integração com Branch (pks-knowledge, worktree) ✅
        CLI `pks init` + `pks doctor` (Setup & Reparo) ✅
        F2.1 Hook Post-Commit / FSEvents ✅
        F2.7 Repositórios Remotos/Distribuídos ✅

FASE C: Embeddings, Trackers & Operação (M5, M6, M7) ✅ 2026-03-09
   │
   ├─ M5: Embeddings Vetoriais ✅
   │    Integração Local com Ollama (Background Queues) ✅
   │    Busca Híbrida (BM25 + Cosine SIMD + RRF) ✅
   │    Deduplicação por hash SHA-256 de parágrafo ✅
   │    Degradação Gradual — 5 estados (Dívida de Embedding) ✅
   │    LRU + Hibernação automática (watermark PKS_MAX_VECTORS) ✅
   │    Re-idratação via bincode snapshot ✅
   │
   ├─ M6: Maturação e Operação ✅
   │    F2.13 Observabilidade (Tracing, Metrics) ✅
   │    F2.14 Contingência (Cloud Snapshots via Git LFS) ✅
   │    Nível de Produção (Graceful shutdown, units) ✅
   │
   └─ M7: Trackers e Expansão ✅
        F1.3 Import / F1.4 Export (Notion, Jira) ✅
        F1.6 Tracker Sync Queue (FIFO + retry) ✅
        F1.5 Política de Armazenamento Seletivo (Trackers) ✅

FASE D: Git Journal & Validação (M8) ✅ 2026-03-10
   │
   └─ M8: Git Journal Append (T4.4 Extension) ✅
        F2.6 ext Git Journal: filtro Conventional Commits no hook ✅
        Append em ${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/YYYY-MM-DD_log.md ✅
        CLI `pks hook-post-commit` extendido via `git_journal_append.rs` ✅

FASE E: Singleton Daemon & IPC (M10, M11, M12, M13) ✅ 2026-03-15
   │
   ├─ M10: Singleton Daemon + IPC ✅
   │    Unix socket IPC /tmp/pks.sock ✅
   │    PID lockfile e auto-spawn daemon ✅
   │    Migração MCP client para IPC Proxy ✅
   │
   ├─ M11: RepoId + Bare Commits ✅
   │    RepoIdentity via git-common-dir ✅
   │    BareCommit sem dirty tree (git2-rs plumbing) ✅
   │    Suporte multi-worktree nativo ✅
   │
   ├─ M12: Shadow Journaling Passivo ✅
   │    Flush para pks-knowledge via BareCommit ✅
   │    Secrets redaction e fallback offline ✅
   │
   └─ M13: Ollama Opcional + pks_execute ✅
        Busca BM25-only 100% funcional ✅
        pks_execute sandbox + context-mode ✅

FASE F: Onboarding & Hooks (M14, M15, M16) ✅ 2026-03-25
   │
   ├─ M14: Zero-Config Onboarding ✅
   │    pks init automatizado em <30s ✅
   │    Slash command /pks-init ✅
   │
   ├─ M15: Shadow Journaling via Hooks ✅
   │    Captura automática via PostToolUse/Stop hooks ✅
   │    Scripts de flush anti-loop ✅
   │
   └─ M16: Vault Isolation ✅
        prometheus/ excluído do walker do repo pai ✅
        Indexação isolada como {repo}-vault ✅

FASE G: Estabilização e Verificação ✅ 2026-04-01
   │
   └─ M17: Verificação Real de Sistema ✅
        Deploy em ambiente de produção (Antigravity Antigrav) ✅
        Verificação de multi-repo search E2E ✅
        Consolidação da documentação (Diagrama + PRD) ✅
```
> PKS Team Node reside no backlog abstrato (Apendice B).
> Metas ambiciosas de Seguranca (Tokens, mTLS) estao postergadas em nome do uso pessoal inicial (Loopback Localhost exclusivo).
> Escala Extrema (HNSW, Battery Awareness, Cloud Offload) permanece no Apendice C.

### Criterios de Aceite (por Milestone)

**M1 — A Engine Rustica**
- [x] Servidor MCP stdio responde via `pks --stdio` (mock com dados fixos no primeiro momento).
- [x] Ferramenta MCP `list_knowledge_vaults` disponivel retornando os repos atualmente geridos pelo Daemon.
- [x] Parser de Markdown extrai chunks por heading (`##`, `###`) e aplica sliding window para secoes longas (>400 tokens).
- [x] Secoes curtas (<100 tokens) sao agrupadas com a secao seguinte.
- [x] Busca BM25 via Tantivy retorna resultados relevantes para queries textuais sobre um vault de teste.
- [x] Filesystem-as-Config: clonar um repo em `~/pks-vaults/` registra-o automaticamente; deletar o diretorio o desregistra.
- [x] `pks status` exibe resumo basico dos repos detectados.

**M2 — Git Journal e Conexao com o Hospedeiro**
- [x] `pks init <path>` configura o repositorio: cria branch orphan `pks-knowledge`, worktree, `.git/info/exclude` e hook post-commit.
- [x] `pks doctor <path>` diagnostica e repara estados degradados (worktree desconectado, hook ausente, exclude faltando).
- [x] Branch `pks-knowledge` e criado automaticamente via `pks init`; commits de conhecimento nao poluem `main`/`feature`.
- [x] Vault com a estrutura `prometheus/` abre no Obsidian sem erros e com navegacao funcional entre notas via wikilinks.
- [x] Post-commit hook notifica o Daemon instantaneamente apos cada `git commit`.
- [x] OS FS Events em `.git/refs/heads/` captura `pull`, `merge`, `rebase` e `amend` com zero polling.
- [x] Commit via CLI + FS Event disparados simultaneamente geram apenas 1 entrada na Fila 1 (idempotencia por Debounce).
- [x] Rebase agressivo aciona reindex completo do repo afetado (Drop & Rebuild).

**M3 — Daemon Continuo e Pipeline**
- [x] Daemon roda como servico de SO (`launchd`/`systemd`) e mantem indices BM25 vivos em RAM.
- [x] Double-Buffered Pipeline funcional: queries na thread principal nunca bloqueiam durante indexacao background.
- [x] `search_knowledge_vault` retorna resultados BM25 em <1ms consumindo zero disco.
- [x] Logs estruturados JSON via `tracing` com rotacao automatica por tamanho.
- [x] Health check endpoint (`/health`) retorna status do daemon, profundidade das filas e repos registrados.
- [x] Metricas expostas conforme tabela do PRD: `pks_fila1_depth`, `pks_fila2_depth`, `pks_query_latency_us`, `pks_repos_warm`.
- [x] Servidor MCP escuta apenas em `localhost`; conexoes externas recusadas por padrao.
- [x] `pks validate` compara hashes SHA-256 dos `.md` com o indice e reporta divergencias.

**M4 — Persistencia e Resiliencia**
- [x] Snapshot `bincode` segmentado por repo (`snapshots/<repo_id>.bin`) serializa e deserializa corretamente.
- [x] Snapshot com Magic Version Header: mismatch de versao deleta o `.bin` e aciona reindex completo.
- [x] Carga/descarga manual funcional: `git clone` em `~/pks-vaults/` carrega o repo na RAM; `rm -rf` descarrega e purga RAM + snapshot.
- [x] Vector Clock salvo no snapshot: reidratacao no reboot detecta commits novos alem do snapshot e reprocessa o delta.
- [x] Reindex completo do vault (1000 notas) converge em <2 minutos (BM25 puro, sem vetorizacao).
- [x] Perda total da maquina: `git clone` + reindex completo reconstitui o indice funcional (BM25 imediato).

**M5 — Embeddings Vetoriais** ✅ concluído (2026-03-09)
- [x] Integracao com Ollama (`nomic-embed-text`): chunks sao vetorizados em background via FIFO. *(T5.1e — `fifo_embedder.rs`, `embedding_provider.rs`)*
- [x] Busca hibrida (BM25 + Cosine SIMD) retorna resultados combinados com scoring unificado (Reciprocal Rank Fusion, k=60). *(T5.2e — `search/hybrid.rs`)*
- [x] Deduplicacao por hash SHA-256: edicao parcial de um `.md` resulta em re-vetorizacao apenas dos chunks cujo hash de paragrafo mudou (chave posicional por `file_path::chunk_index`). *(T5.3e — `indexer/dirty_tracker.rs`)*
- [x] Daemon inicia e opera normalmente sem Ollama instalado (5 estados de degradacao gradual: nao instalado, modelo ausente, offline temporario, offline prolongado, retorno). *(T5.1e — enum `OllamaState`)*
- [x] `search_knowledge_vault` retorna resultados hibridos em <10ms (cosine similarity via iteradores, RRF em-memoria). *(T5.2e — `search_hybrid`)*
- [x] Golden dataset: testes de busca hibrida passam; golden dataset completo (50 notas, 90%+ top-3) a validar com Ollama rodando. *(T5.2e — 9 testes aprovados)*
- [x] Watermark de vetores (`PKS_MAX_VECTORS`): ao atingir o limite, o repo LRU e ejetado da memoria via `evict_if_over_watermark`. *(T5.4e — `lru_manager.rs`)*
- [x] Cold-start de 1 repo hibernado: vetores recarregados do snapshot em <200ms. *(resolvido na implementação do M6)*

**M6 — Maturação e Operação** ✅ concluído (2026-03-09)
- [x] Daemon roda como serviço de SO (`launchd`/systemd) com auto-restart e logs estruturados.
- [x] Health check endpoint (`/health`) retorna métricas em formato JSON.
- [x] `pks status` e `pks validate` funcionais para operação e diagnóstico.
- [x] Sincronização LFS em repositório satélite exclusivo (Isolamento 1:1) implementada.
- [x] Perda total da máquina com cloud: `git clone` do sub-repo LFS reconstitui snapshots sem re-vetorização.
- [x] Robustez: Daemon drena filas e salva estado em disco no SIGTERM (Graceful Shutdown).

**M7 — Trackers Contextuais** ✅ concluído (2026-03-09)
- [x] Import de Tracker (Notion/Jira) gera `.md` com frontmatter YAML e commit no `pks-knowledge`.
- [x] Export para Tracker publica conteúdo local e atualiza metadados com detecção de colisão (OCC).
- [x] Tracker Sync Queue processa operações em background com retry e respeito a rate limits.
- [x] Sanitização de conteúdo importado (XSS/HTML) ativa com limite de tamanho.
- [x] Política de Armazenamento Seletivo (`docs/STORAGE_POLICY.md`) define o que entra no vault.

**M8 — Git Journal Append (T4.4 Extension)** ✅ concluído (2026-03-10)
- [x] Branch `pks-knowledge` é ignorado: commits nessa branch não geram append (sem recursão).
- [x] Commits sem prefixo Conventional Commits são descartados (ex: `wip`, `fix typo`, sem prefixo `feat:`, `fix:`, etc.).
- [x] Commits com prefixo válido mas menos de `PKS_GIT_MIN_WORDS` palavras são descartados (default: 5).
- [x] Autores listados em `PKS_GIT_IGNORE_AUTHORS` são ignorados (ex: `github-actions[bot]`).
- [x] Arquivo `${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/YYYY-MM-DD_log.md` recebe append com linha formatada.
- [x] Formato exato: `- **HH:MM** - \`<sha7>\` - <author>: <subject>` — conforme STEERING_3_6 §3.2.
- [x] Daemon PKS re-indexa o arquivo modificado automaticamente via FSWatcher (F2.1 — sem mudanças no daemon). **[FSWatcher removido — STEERING_remove_fswatcher.md; re-indexação agora acionada via `pks refresh`]**
- [x] `cargo test` passa, incluindo testes unitários de `git_journal_append.rs`.
- [x] `.env.example` contém todas as variáveis de ambiente do Git Journal: `PKS_VAULT_ROOT`, `PKS_GIT_LOG_ENABLED`, `PKS_GIT_ALLOW_PREFIXES`, `PKS_GIT_MIN_WORDS`, `PKS_GIT_IGNORE_AUTHORS`.

**M9 — Instalador One-Click Base** ✅ concluído (2026-03-10)
- [x] Workflow `/pks-install` (Core) detecta o OS (macOS via `uname`, Windows via PowerShell) e executa a suite correta **sem Ollama**.
- [x] No macOS: Homebrew é instalado se ausente; `git` e `gh` instalados via `brew`; Rust via `rustup`. VETORIZAÇÃO É OPCIONAL E SEGREGADA.
- [x] No Windows: `git`, `gh` e `rustup` instalados via `winget` com flags `--accept-*`; PATH atualizado. VETORIZAÇÃO É OPCIONAL E SEGREGADA.
- [x] Todos os comandos de instalação são idempotentes (guards `if not exists` evitam reinstalação).
- [x] Verificação pós-instalação confirma: `rustc --version`, `git --version`, `gh --version`, `ollama list` mostra `nomic-embed-text`.
- [x] Nenhum passo da instalação exige input manual do usuário — flags `-y` (Homebrew/rustup) e `--accept-source-agreements --accept-package-agreements` (Winget) garantem execução totalmente silenciosa.
- [x] Arquivo `.agent/workflows/pks-install.md` existente já implementa o fluxo completo.

---

## Decisões Tomadas

| # | Decisão | Opções Avaliadas | Decisão Final |
|---|---|---|---|
| F2 | Identidade do Projeto | Pasta Local vs Git Common Dir (UUID) | **Git Common Dir (UUID)** — o Daemon usa `git rev-parse --git-common-dir` para unificar 10 worktrees da mesma branch em 1 único espaço alocado na memória. |
| D1 | Onde fica o vault? | Isolado num repo central vs Autocontido junto a cada código fonte |  **Autocontido por projeto Git**  um único Daemon unifica N repos via MCP. |
| D2 | Embedding model | `nomic-embed-text`, `mxbai-embed-large`, `all-minilm` |  `nomic-embed-text` (melhor custo-benefício local). |
| D3 | Índice Prevalente | CLI Efêmera vs Daemon Contínuo |  **Singleton Daemon Contínuo** em background (RAM quente 100% do tempo), sendo acessado apenas por Proxies leves IPC (Cliente/Servidor local). |
| D4 | Integração AI | Script custom vs Tool MCP Nativa |  Tool MCP Nativa (`search_knowledge_vault` via `pks --stdio` atuando como proxy JSON-RPC). |
| D5 | Durabilidade | journal.bin append vs Git History |  **Git History** como Journal Prevalente (Commits). Nas worktrees secundárias isso ocorre "bare" via `git2-rs`, evintado erros filesystem. |
| D6 | MCP Transport | stdio vs SSE vs Streamable HTTP |  **stdio** (`pks --stdio`) atuando apenas como um proxy para a comunicação com o Daemon de background via Sockets/TCP Locais. Configuração via `.mcp.json`. |
| D7 | Concorrência | Multi-thread com RwLock vs Single-thread |  **Single-thread (NYSE-style)** no Motor central indexador, sem locks de escrita, sem contention. As requisições IPC entram numa MPSC Queue. |
| D8 | Vetorização | Síncrona (bloqueia query) vs Assíncrona (FIFO) |  **FIFO assíncrono**  BM25 always-ready, vetores convergem em background no Daemon de sistema. |
| D9 | Linguagem | Java (Prevayler original) vs Rust |  **Rust**  zero GC, limite de RAM = hardware físico. |
| D10 | Chunking | Fixed sliding window vs Semântico por heading vs Sentence-level |  **Semântico por heading** com fallback sliding window. Preserva unidades Markdown. |
| D11 | Interceptação MCP | Desligada vs Shadow Journaling Inteligente | Após M9: Inserir ferramentas MCP (`pks_execute`) para que o proxy do PKS engula comandos grandes, BM25-ize o conteúdo e livre o Context Window da IA (*Apendizado do project-context-mode*). |
| D12 | Segurança MCP | Aberto (sem auth) vs Token-based vs mTLS |  **Localhost-only.** Unix Domain Sockets com restrição de permissão de usuário atual. |
| D13 | Formato do snapshot | bincode (Segmentado por Repo) vs Arquivo Monolítico |  **bincode segmentado c/ "Magic Version Header"** (`snapshots/<repo_id>.bin`). Se o schema evoluir, reconstrói sem quebrar o Singleton. |
| D14 | Branch strategy | `prometheus/` ativa vs Branch `pks-knowledge` (histórico limpo) |  **Branch dedicado `pks-knowledge`**. Na worktree "Master", ela é materializada. Em worktrees de features (criadas aleatoriamente), ela não existe fisicamente na pasta: o Proxy avisa o Daemon, que faz as injeções "Bare Commit" lá na base original. Trade-off: evita erro `fatal: already checked out` e unifica cérebros da IDE primária com as IDEs em shadow. |

---

## Planejamento de Implementação: M8 e M9

### M8 — Git Journal Append

#### Subtarefas

| # | Tarefa | Arquivos | Depende de |
|---|--------|----------|------------|
| T8.1 | Implementar lógica de filtro em `git_journal_append.rs`: lê env vars, filtra prefixo CC e min_words, ignora autores; resolve path `${PKS_VAULT_ROOT:-prometheus}/90-ai-memory/YYYY-MM-DD_log.md`; formato de linha conforme STEERING_3_6 §3.2: `- **HH:MM** - \`<sha7>\` - <author>: <subject>` | `pks/src/git_journal_append.rs` [CREATE/EXISTS] | — |
| T8.2 | Estender `pks hook-post-commit` para invocar `git_journal_append` com SHA, autor e subject do commit atual | `pks/src/cli.rs` [EXISTS] | T8.1 |
| T8.3 | Ignorar commits do branch `pks-knowledge` no hook (lê branch atual via git2-rs) | `pks/src/git_journal_append.rs` [EXISTS] | T8.1 |
| T8.4 | Garantir idempotência de escrita de curta duração na Fila 1 (Debounce delegável do Consumidor) para mitigar eventuais double-firings do GUI/MacOS | `pks/src/git_journal_append.rs` [EXISTS] | T8.1 |
| T8.5 | Adicionar variáveis de ambiente ao `.env.example` com valores default e comentários | `.env.example` [CREATE/EXISTS] | — |
| T8.6 | Escrever testes unitários em `git_journal_append.rs` (`#[cfg(test)]`) cobrindo: filtro CC, min_words, ignore_authors, formato de linha exato | `pks/src/git_journal_append.rs` [EXISTS] | T8.1, T8.3, T8.4 |
| T8.7 | Criar harness de integração isolado `git_journal_harness.rs` (separado do harness de busca) com testes end-to-end: commit real → append no arquivo de log → verificação de conteúdo | `pks/tests/git_journal_harness.rs` [CREATE] | T8.2, T8.6 |
| T8.8 | Smoke-test FSWatcher: escrever arquivo de log, verificar que o daemon detecta a modificação via log estruturado (critério 7 do PRD §M8 — sem mudanças no daemon) | `pks/tests/git_journal_harness.rs` [EXISTS] | T8.7 |

#### Critério de conclusão de M8

`cargo test` passa com ≥ 1 teste cobrindo cada critério de aceite do PRD §M8.

---

### M9 — Instalador One-Click

#### Subtarefas

| # | Tarefa | Arquivos | Depende de |
|---|--------|----------|------------|
| T9.1 | Validar workflow existente `.agent/workflows/pks-install.md` contra critérios do PRD §M9 incorporando passo de Build cargo (T9.5) | `.agent/workflows/pks-install.md` [EXISTS] | — |
| T9.2 | Consertar falso-positivo das validações via `brew list` e garantir comandos Posix via `command -v` | `.agent/workflows/pks-install.md` [EXISTS] | T9.1 |
| T9.3 | Tratamento de Race Condition do Ollama: substituir `sleep 5` preguiçoso por polling Curl e Invoke-RestMethod com tolerância de 30s | `.agent/workflows/pks-install.md` [EXISTS] | T9.1 |
| T9.4 | Garantir refresh de `$env:Path` após qualquer instalação via winget (não apenas Rust) — PATH do Ollama deve estar disponível antes do `ollama pull` | `.agent/workflows/pks-install.md` [EXISTS] | T9.1 |
| T9.5 | Acrescentar diretiva de Compilação Local com `cargo install --path pks` fechando o workflow em ambas as OS | `.agent/workflows/pks-install.md` [EXISTS] | T9.1 |

#### Critério de conclusão de M9

Workflow `/pks-install` executa do zero em macOS sem input manual, conclui com `ollama list` mostrando `nomic-embed-text`, e é idempotente (segunda execução não reinstala nada).

---

### Dependências entre M8 e M9

M8 e M9 são **independentes** — podem ser executados em paralelo. M9 não depende de nenhum artefato de M8.

---

## Apendice A  Detalhamento Arquitetural: Shadow Repositories (Fase 2, M7)

> **Status:** Planejado para Fase 2, Milestone 7. Depende da estabilizacao do MVP e dos Embeddings (M5).

Em um horizonte futuro, como o sistema indexará fontes caóticas e externas como Slack, WhatsApp, E-mails ou mesmo código-fonte arbitrário? O PKS se manterá fiel à sua premissa fundamental: **O Git é o Journal Prevalente.**

Para indexar origens que não são nativamente arquivos textuais versionados (como APIs de chat ou caixas de e-mail), o PKS não criará acoplamento com APIs de terceiros. Em vez disso, usaremos o padrão **Shadow Git Repositories**.

### A Arquitetura de "Shadow Daemons"

```text
[Slack API]      [Email IMAP]      [WhatsApp API]
     │                 │                 │
     ▼                 ▼                 ▼
  Daemon         Daemon          Daemon   (Conectores Isolados)
  Slack          Email           Wpp
     │                 │                 │
     ▼                 ▼                 ▼
[Repo Slack-Log]  [Repo Email-Log]  [Repo Wpp-Log]
 (Git nativo)      (Git nativo)      (Git nativo)
     │                 │                 │
     └─────────────────┼─────────────────┘
                       ▼
               FILA 1 (Transações Brutas do PKS)
```

1. **Daemons Conectores (Git-Ops Style):** Pequenos serviços complementares (Python, Go, etc) ou Cronjobs conectarão nas APIs de terceiros (ex: hook do Slack).
2. **Transformação para Markdown:** O daemon recebe a transcrição do áudio do Zap ou a thread do Slack, formata como `2026-10-14_marketing_zap.md` e gera um commit automatizado em um repositório Git isolado na máquina (`/vaults/wpp-shadow-repo`).
3. **Escuta Transparente do PKS:** Como o PKS indexa N repositórios Git, ele olhará para esses `Shadow Repositories` de forma nativa e reagirá aos commits como faria com um código normal.
4. **Código-fonte Aberto:** Da mesma forma, projetos de software (Java, Python, JS) inteiros não precisam ter o Obsidian aberto neles  o PKS pode ser configurado para monitorar clones bare-metal desses repositórios. O Fila 1 do PKS fará parse semântico de trechos em `main.rs` com seus metadados da mesma forma que processa resumos em Markdown do Slack.

**Benefício:** A integridade de busca, persistência, durabilidade e concorrência do PKS permanece intacta. Toda fonte externa mal estruturada é forçada a ser "limpa, renderizada para Markdown e persistida no Git" antes de chegar no PKS. O índice MCP responde N domínios perfeitamente.

> **Criterios de Aceite:** Ver M7 na secao de Milestones.

---

## Apêndice B  Visão de Futuro: PKS Team Node (Nó Semente Colaborativo)

> **Status:** Visao exploratoria. Depende da estabilizacao da Fase 2 e maturidade operacional do sync via Git LFS Sub-Repository.

O PKS Team Node é a evolução natural do Git LFS Sub-Repository: um servidor do time rodando na Nuvem ou On-Premise que age como Worker 24/7. Ele indexa nativamente os grandes monólitos da empresa e pré-computa os snapshots segmentados (`snapshots/<repo_id>.bin`).

**Entrega via Git LFS:** O Team Node simplesmente faz *push LFS* dos snapshots pré-computados no sub-repo satélite (`pks-vector-cache.git`). As máquinas clientes dos desenvolvedores fazem *git pull LFS* desse sub-repo em background, recebendo os embeddings atualizados sem intervenção manual.

**Cenário:** Quando um Desenvolvedor Júnior entra no projeto e clona o sub-repo LFS, o PKS local detecta os snapshots e carrega os embeddings pré-computados para a RAM em segundos, evitando horas de vetorização que a máquina local teria que suportar.

**Impacto arquitetural:** Esta feature transforma o PKS de um daemon local single-user num sistema distribuído via Git LFS, o que exige decisões adicionais sobre versionamento de snapshots e cotas LFS dos provedores. Será detalhada em PRD próprio.

---

## Apêndice C  Visão de Futuro: Escala Extrema e Eficiência Híbrida

> **Status:** Direcionamentos arquitetônicos desenhados para adentrar o roadmap assim que os limites iniciais da Fase 2 de adoção sofrerem as pressões inerentes do crescimento exponencial.

### 1. Indexação Espacial Aproximada (HNSW)
**Gargalo Original:** Iniciar o PKS com busca Exata (O(N) *SIMD Cosine/Flat Index*) é incrivelmente rápido — e estrategicamente o caminho ideal em projetos de pequeno a médio porte (milhares de chunks isolados na RAM). Contudo, em cruzamentos massivos estendendo sobre os domínios corporativos com múltiplos repositórios e dezenas de milhões de nós associados, a latência linear O(N) degradará proporcionalmente ao volume, corrompendo as metas *sub-ms*.
**Evolução Planejada:** Começamos intencionalmente pequenos e isolados com a conta matemática exata. No limiar detectado de desgaste e expansão, a arquitetura comuta organicamente rumo à Indexação Espacial mediante Grafo **HNSW (Hierarchical Navigable Small World)** conectado no próprio Daemon. Aceitamos a irrisória desidratação da precisão (*Recall*) nos centésimos a fim de estabilizar de maneira perpétua os índices de acerto, comutando da complexidade linear O(N) para a complexidade logarítmica O(log N).

### 2. Offload Externo e Gestao Eletrica (Battery Awareness)
**Gargalo Original:** Processamento tenso provido por um LLM embarcado (*Ollama*) sobre onera drasticamente o maquinario dos desenvolvedores, esgotando sua bateria local vertiginosamente sob contextos prolongados, fora da rede eletrica CA, inviabilizando edicoes pesadas.
**Evolucao Planejada:** O sistema ira preferir delegar este fluxo para escorar nas oscilacoes puras de frestas ociosas do disco ("system idle"); num segundo estopim de maturidade o sistema abracara uma diretriz profunda de bateria, estagnando o processo do Ollama completamente quando desplugado de forca nominal: textos meramente passarao ao indice levinho do `BM25`. Por via final, e com flexibilizacoes modulares prontas do Daemon, preparamos roteamentos plenos para externalizar o rebarbador de chunks para Cloud APIs ou nos servidores dedicados da instituicao (preservando o Notebook livre do fardo de computo, em detrimento do custo de transito em rede leve via gRPC).

### 3. HOT RAM / COLD INDEX (Gestao Automatica de Memoria)

**Gargalo Original:** No MVP, a gestao de memoria e manual: o desenvolvedor controla quais projetos estao vivos na RAM clonando ou removendo repositorios de `~/pks-vaults/`. Isso funciona perfeitamente para 5-10 projetos. Porem, em cenarios corporativos com dezenas de repositorios, esperar que o humano gerencie manualmente quais projetos consomem RAM e insustentavel.

**Evolucao Planejada:** O PKS adotara um modelo de **duas camadas de temperatura** para indices:

```text
┌─────────────────────────────────────────────────┐
│             HOT RAM (Indice Quente)              │
│  - Indices BM25 + Vetores vivos em memoria       │
│  - Queries respondem em sub-milissegundo         │
│  - Repos usados ativamente pelo desenvolvedor    │
│  - Limite: PKS_MAX_VECTORS (ex: 500.000)         │
└───────────────────────┬─────────────────────────┘
                        │ Sem queries por X dias
                        │ OU watermark estourado
                        v
┌─────────────────────────────────────────────────┐
│           COLD INDEX (Indice Frio)               │
│  - Snapshot bincode persiste em disco            │
│  - Nenhuma RAM consumida                         │
│  - Query dispara reaquecimento sob demanda       │
│  - Reaquecimento: <200ms para BM25, ~1s vetores  │
└─────────────────────────────────────────────────┘
```

**Mecanica de Promocao/Rebaixamento:**

| Evento | Transicao | Acao |
|---|---|---|
| Nova query em repo frio | COLD -> HOT | Daemon carrega snapshot do disco para RAM (lazy load). Resposta BM25 imediata, vetores aquecem em background. |
| Repo sem queries por `PKS_HIBERNATE_DAYS` (ex: 7) | HOT -> COLD | Vetores descarregados primeiro (pesados: ~1-4MB/1000 chunks). BM25 pode permanecer como ultima camada antes da evicao total. |
| Watermark `PKS_MAX_VECTORS` atingido | HOT -> COLD (LRU) | O repo menos recentemente consultado e ejetado integralmente da RAM para abrir espaco. Evicao cirurgica, sem swap de OS. |
| Repo removido de `~/pks-vaults/` | HOT/COLD -> PURGADO | RAM e snapshot sao deletados permanentemente. |

**Por que nao no MVP?**
- Com BM25-only (MVP, sem vetores), o consumo de RAM e irrisorio: dezenas de KB por 1000 notas. A gestao manual e mais que suficiente.
- A complexidade de LRU automatico, watermarks e lazy-load justifica-se apenas quando vetores densos (768-D floats, ~3KB por chunk) entram em jogo na Fase 2 (M5), multiplicando o footprint de memoria por ordens de grandeza.
- A implementacao de HOT/COLD e planejada como extensao natural do M5 (Embeddings), podendo ser adicionada como M5.1 se a pressao de memoria se manifestar.

**Snapshotting Segmentado (pre-requisito do COLD INDEX):** A serializacao `bincode` ja e segmentada por repo (`snapshots/<repo_id>.bin`) desde o M4, o que torna o COLD INDEX uma extensao natural: cada repo pode ser descarregado e recarregado independentemente, sem tocar nos demais.

### 4. Provider de Embeddings: Ollama vs MLX vs Rust Nativo

**Gargalo Original:** O Ollama, embora conveniente e estavel, utiliza `llama.cpp` como backend no Apple Silicon. Isso introduz overhead de marshalling entre camadas, comunicacao HTTP (serializar JSON, roundtrip, deserializar) e copia de tensors entre CPU e GPU. Para vetorizacao em lote (reindex de milhares de chunks), esse overhead acumula.

**Analise Comparativa (Apple Silicon):**

| Provider | Throughput (1 chunk 400t) | Reindex 1000 chunks | Reindex 10.000 chunks | Integracao com Rust |
|---|---|---|---|---|
| **Ollama** (llama.cpp + Metal) | ~15-25ms | ~20-25s | ~4 min | HTTP API (overhead de rede local) |
| **MLX** (Apple nativo) | ~6-12ms | ~8-12s | ~1.5-2 min | FFI Swift-Rust (complexo) ou subprocess Python |
| **candle-rs** (Rust + Metal) | ~8-15ms | ~10-15s | ~2-3 min | Nativo, in-process, zero overhead |

> **Nota:** Valores estimados com base em benchmarks publicados de inferencia BERT-base no MLX (38ms M2 Max) vs llama.cpp (179ms M1), ajustados para a tarefa mais leve de embedding (forward pass unico, sem geracao auto-regressiva). O ganho real depende do modelo, quantizacao e hardware especifico.

**Por que o MLX e mais rapido no Apple Silicon?**
1. **Unified Memory zero-copy:** O MLX opera diretamente na memoria unificada do Apple Silicon. Nao ha copia de tensors entre CPU e GPU — o ponteiro e compartilhado. O llama.cpp (Ollama) ainda faz marshalling entre camadas.
2. **Metal kernels nativos:** O MLX foi escrito pela Apple com kernels Metal otimizados para seus proprios chips. O llama.cpp usa Metal como backend secundario com camada de abstracao.
3. **Sem overhead de servidor:** O Ollama roda como HTTP server separado. Cada chamada de embedding envolve: serializacao JSON -> socket -> deserializacao -> compute -> serializacao -> socket -> deserializacao. Com MLX ou candle-rs in-process, o vetor sai direto na memoria do Daemon.

**Caminho Evolutivo Recomendado:**

```text
  M5 (Fase 2)           Futuro proximo             Futuro distante
  ─────────────────────────────────────────────────────────────────
  Ollama (padrao)  -->  Ollama c/ MLX runner  -->  candle-rs nativo
  - Estavel, facil      - Ganho ~2x gratis         - Zero overhead
  - API conhecida       - Sem mudanca no PKS       - In-process
  - Cross-platform      - Apenas Apple Silicon     - Metal + CUDA
```

1. **M5 (Embeddings):** Manter Ollama como provider padrao. API estavel, `nomic-embed-text` suportado, funciona em qualquer OS. O throttle `PKS_THROTTLE_MS` absorve a latencia.
2. **Ollama + MLX runner:** O Ollama 0.17+ ja inclui MLX runner experimental para modelos selecionados (Gemma 3, Llama, Qwen3). Quando o suporte a embedding models estabilizar, o PKS ganha ~2x de throughput **sem alterar uma linha de codigo** — o Ollama roteia internamente para o backend mais eficiente.
3. **candle-rs (longo prazo):** Para eliminar completamente o overhead de IPC, a alternativa definitiva e usar `candle` (framework ML em Rust puro com suporte Metal e CUDA). O modelo `nomic-embed-text` pode ser carregado in-process pelo Daemon, gerando vetores direto na memoria sem nenhum roundtrip. Trade-off: aumenta a complexidade do build e o tamanho do binario.

**Decisao Arquitetural:** O PKS abstrai o provider de embeddings atras de uma trait Rust (`EmbeddingProvider`), permitindo trocar entre Ollama, MLX (via FFI) ou candle-rs sem impacto na pipeline. A trait expoe apenas `fn embed(text: &str) -> Vec<f32>`. A escolha do backend e configuravel via `PKS_EMBEDDING_PROVIDER={ollama|candle}`.
