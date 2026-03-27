# STEERING: Evolução PKS — Compressão Vetorial via TurboQuant

**Data:** Março de 2026
**Referência:** arXiv:2504.19874 — Google Research / Google DeepMind / NYU
**Contexto:** Evolução arquitetural do Prometheus Knowledge System (PKS) para suportar bases de conhecimento de escala industrial mantendo o Daemon inteiramente em RAM.

---

## 1. O Problema Atual

O PKS é um sistema Prevalente: **todos os vetores vivem em RAM 24/7**. Cada chunk indexado carrega um `Vec<f32>` com 768 a 4096 floats (modelo dependente), consumindo:

- **nomic-embed-text** (768 dims): ~3 KB por chunk em f32
- 100.000 chunks → ~300 MB só de vetores no `HashMap<String, Vec<f32>>`

Isso força limitadores agressivos (`PKS_MAX_VECTORS`, LRU hibernation). **Nota:** os snapshots `bincode` (`SnapshotData`) *não* armazenam vetores — contêm apenas `ChunkRecord` (file_path, heading_hierarchy, chunk_index, chunk_hash, chunk_text). O problema de memória é exclusivamente de RAM em tempo de execução.

---

## 2. A Solução: TurboQuant

TurboQuant é um quantizador vetorial **online e data-oblivious** desenvolvido por Google Research e DeepMind. Ele alcança taxas de distorção próximas do ótimo teórico, comprimindo cada vetor de f32 para **2.5–3.5 bits por dimensão** com dois estágios:

### Estágio 1 — Rotação + Quantização Escalar MSE

1. **Rotação aleatória (Randomized Hadamard Transform):** Aplica `Π = D·H` ao vetor de entrada, onde `H` é a transformada Walsh-Hadamard e `D` é uma matriz diagonal com sinais aleatórios `{+1, -1}`. O resultado é que cada coordenada do vetor rotacionado segue uma **distribuição Beta concentrada** — independentemente de como era o vetor original. Isso é o segredo: agora todas as coordenadas têm a mesma distribuição, permitindo usar o mesmo quantizador escalar ótimo para todas.

2. **Quantização escalar por coordenada:** Com a distribuição Beta conhecida e concentrada, o paper pré-computa os thresholds ótimos para b bits via o **algoritmo Max-Lloyd** (resolução de um k-means 1D contínuo sobre a distribuição Beta). Esses thresholds são fixos (tabelados para 1, 2, 3, 4 bits) e aplicados independentemente em cada coordenada. Resultado: `b` bits por dimensão com distorção MSE near-optimal.

3. **A matriz de rotação precisa ser a mesma na indexação e na busca.** A seed `D` (os sinais aleatórios) é gerada uma vez por vault (ou globalmente) e persistida. Todos os vetores de query e de documento são rotacionados com a mesma `Π`.

### Estágio 2 — QJL no Resíduo (Correção do Inner Product)

MSE-optimal ≠ Inner-Product-optimal. Quantizadores MSE introduzem **viés sistemático** no produto interno — isso causa degradação de qualidade no ranking semântico.

A solução do paper é aplicar uma transformada **Quantized Johnson-Lindenstrauss (QJL) de 1 bit** sobre o **resíduo** (diferença entre o vetor original e a reconstrução MSE). Especificamente:

- Computa-se o resíduo `r = x - x̂` (onde `x̂` é a reconstrução do Estágio 1).
- Uma projeção JL aleatória é aplicada a `r`, e o sinal (1 bit) é armazenado.
- Na busca, esse bit é usado como corretor do produto interno, eliminando o viés.

**Resultado:** estimador de produto interno **unbiased** (sem viés), com distorção próxima do lower-bound de Shannon (~2.7x fator constante).

### Números reais do paper (KV Cache e Nearest Neighbor Search)

| Bits/dim | Qualidade (KV cache LLM) | Recall (ANN) |
|----------|--------------------------|--------------|
| 3.5 | **Neutralidade absoluta** vs float | Superior ao Product Quantization |
| 2.5 | Degradação **marginal** aceitável | Ainda superior ao PQ |

---

## 3. Impacto Arquitetural Honesto no PKS

### 3.1 Redução de RAM (estimativa conservadora e corrigida)

- Vetor `nomic-embed-text` em f32: 768 × 4 bytes = **3.072 bytes**
- Com TurboQuant a 3.5 bits: 768 × 3.5 / 8 ≈ **336 bytes** + overhead de metadados (~24 bytes) = ~360 bytes
- **Ganho no componente de valores do HashMap: ~8.5x** *(apenas os valores f32 — as chaves String não são comprimidas; veja correção abaixo)*

**Porém**, o `HashMap<String, Vec<f32>>` atual usa `chunk.text.clone()` como chave (retriever.rs:134) — texto completo de cada chunk. Com ~500 bytes/chave em média e 100k chunks, isso representa ~50 MB de `String` keys que **não são comprimidas** pelo TurboQuant. O cálculo real do HashMap:

- Antes: ~50 MB (chaves) + ~300 MB (valores f32) = ~350 MB
- Depois: ~50 MB (chaves) + ~35 MB (valores quantizados) = ~85 MB
- **Ganho real no HashMap: ~4x** (não 8.5x)

Para alcançar o ganho teórico de 8.5x no componente vetorial, seria necessário migrar as chaves de `chunk.text` para `chunk_hash` (já disponível no `ChunkRecord`). Além disso, o índice BM25 (Tantivy) armazena texto completo para scoring e provavelmente domina o footprint total de RAM. Estimativa conservadora: **~1.5–2x no footprint total de RAM**, não 4–6x.

### 3.2 Snapshots Git LFS — sem impacto

Os snapshots `.bin` (bincode) contêm `SnapshotData { repo_id, chunks: Vec<ChunkRecord>, vector_clock_sha, created_at_secs }`. O `ChunkRecord` armazena apenas texto e metadados — **nenhum `Vec<f32>`**. Portanto, TurboQuant tem **zero efeito** sobre o tamanho dos snapshots ou sobre o Git LFS. O argumento de eficiência aplica-se exclusivamente à RAM de execução.

### 3.3 Double-Buffered Pipeline (compatibilidade Zero-Cost)

Por ser **online e data-oblivious**, o TurboQuant processa um vetor por vez, sem depender de lote. A Background Thread Consumidora (Fila 1 → Fila 2) pode quantizar cada embedding imediatamente após a resposta do Ollama, **sem bloqueio adicional**. Latência de quantização em tempo de indexação é o custo de uma multiplicação Hadamard (O(d log d)) e thresholding tabular — **sub-milissegundo por vetor** em tempo de inserção.

### 3.4 Retriever: estratégia de dequantização

A busca de Cosine similarity **não opera diretamente sobre os bits comprimidos** (isso requereria implementação de dot product aproximado complexo e de alto risco de precisão). A estratégia adotada é:

1. **Query:** passa pelo mesmo Estágio 1 (rotação), sem quantizar (mantém f32 rotacionado).
2. **Documento:** é dequantizado de volta a f32 (reconstrução rápida via lookup tabular **agressivamente vetorizada via AVX2 / NEON**) antes do dot product.
3. O **corretor QJL** do Estágio 2 é aplicado no score f32 reconstituído para remover o viés.

Esta abordagem é mais simples, segura e ainda preserva o ganho de RAM (os documentos são armazenados quantizados; apenas a query e a janela de busca são expandidas na hora).

### 3.5 Análise de Latência de Busca

O "sub-milissegundo" descrito na Seção 3.3 aplica-se ao **tempo de indexação** (quantizar um único vetor). O caminho de busca é diferente: `search_hybrid()` (hybrid.rs:88-94) itera **todos os vetores armazenados** a cada query. Com TurboQuant, cada vetor deve ser dequantizado (inversa Hadamard + lookup tabular) antes do dot product — sem HNSW ou IVF, a complexidade é O(N) dequantizações por query.

Estimativas de latência de busca (scan bruto, sem ANN):

| Vetores armazenados | Latência estimada (f32 puro) | Latência estimada (TurboQuant) |
|---|---|---|
| 10.000 | ~5 ms | ~3–4 ms |
| 50.000 | ~25 ms | ~15–20 ms |
| 100.000 | ~50 ms | ~30–40 ms |

**Conclusão:** TurboQuant melhora RAM, não latência de busca de forma significativa. O gargalo real é o scan linear O(N). A trajetória futura para escala industrial é a adoção de ANN (HNSW via `hnswlib` ou `usearch`, ou IVF), o que mudaria a estratégia de integração do TurboQuant — nesse cenário, os vetores quantizados seriam inseridos diretamente no índice ANN sem dequantização intermediária.

---

## 4. Pontos Arquiteturais que Requerem Decisão

### Decisão 1: Seed da Rotação e PRNG Determinístico

A matriz `D` (sinais aleatórios) deve ser gerada com uma **semente determinística fixada por vault** (`RepoId`).
- **Recomendação:** Hash SHA-256 do `RepoId` serve como seed inicial para instanciar um PRNG de estabilidade criptográfica, como `rand_chacha::ChaCha8Rng`. Isso previne regressões de entropia localizadas, e assegura que a rotação sorteada no Linux será idêntica byte a byte rodando em Mac ARM. A seed bruta é salva no snapshot `bincode`.

### Decisão 2: Onde Ocorre a Quantização no Pipeline

O `embedding_debt.jsonl` armazena **texto pendente de indexação** (`chunk_text: String`) — não vetores f32. O `EmbeddingDebtEntry` (state.rs:48-54) contém: `repo_id`, `file_path`, `chunk_index`, `chunk_hash`, `chunk_text`. Quando o Ollama volta online, o pipeline drena a fila de texto pendente da seguinte forma:

`chunk_text → Ollama → Vec<f32> → TurboQuant quantiza → insere no HashMap como bytes quantizados`

**Não há migração de formato de arquivo necessária.** A quantização acontece **imediatamente após a resposta do Ollama**, antes da inserção no `HashMap`. Isso é mais simples do que uma migração v2 do arquivo de debt — o arquivo de debt permanece como está (armazena texto), e a quantização é um detalhe do passo de inserção no índice vetorial.

### Decisão 3: Fallback de Bits Estático (Guard Rail)

- Configurar os thresholds fixos para suportes a **2, 3 e 4 bits**.
- Iniciar teste base com `PKS_QUANTIZER_BITS=3`. Se os testes não baterem 95% Recall@5, ter a flag facilmente manobrável no run test sem refatorações amplas em código de base. Thresholds de 4-bits estarão implementados em dia útil.

---

## 5. Critérios de Qualidade Mensuráveis

Para considerar a implementação bem-sucedida, o golden dataset deve validar:

| Métrica | Critério de Aceite |
|---|---|
| `Recall@5` no golden dataset | ≥ 95% do baseline f32 |
| Definição do golden dataset | ≥ 500 pares (query, chunk esperado) por vault representativa |
| `pks_ram_usage_bytes` com 10k chunks | ≤ 50–60% do baseline f32 (reflete ganho real ~4x no HashMap, não 8.5x) |
| Latência de query `pks_query_latency_us` | sem regressão (p95 mantido) |
| `pks validate` após migração | 100% dos chunks reindexados sem erro |

**Nota:** O critério de tamanho de snapshot foi removido — snapshots não contêm vetores e portanto não são afetados pela quantização.

---

## 6. Pré-requisitos no Codebase

Antes de implementar TurboQuant, três problemas existentes no codebase merecem atenção:

### 6.1 `save_all_snapshots` itera todos os vetores para cada repo (state.rs:132)

O filtro `filter(|_| true)` na função `save_all_snapshots` dumpa *todos* os vetores do `HashMap` global em *cada* snapshot por-repo, independentemente do `repo_id`. Isso causa snapshots incorretos (chunks de outros repos misturados) e seria agravado com TurboQuant porque a seed de rotação é por-vault — misturar vetores de vaults diferentes com seeds diferentes corromperia as reconstruções.

**Pré-requisito:** corrigir o filtro para usar `chunk_hash`-to-`repo_id` mapeamento correto antes de qualquer trabalho de quantização.

### 6.2 `build_ranked_rrf_results` descarta metadados para hits apenas-vetorial (hybrid.rs:53-55)

Quando um chunk aparece no resultado vetorial mas não no BM25, o `match lookup.get(chunk_text)` retorna `None` e o resultado recebe `file_path: ""` e `repo_id: ""`. Isso significa que hits relevantes encontrados apenas pela similaridade semântica chegam ao usuário sem metadados de origem.

**Pré-requisito:** manter um mapa secundário de `chunk_text → (file_path, repo_id, heading_hierarchy)` no retriever para preencher esses campos corretamente.

### 6.3 HashMap keyed by `chunk.text` — migração para `chunk_hash`

Como descrito na Seção 3.1, as chaves do `HashMap<String, Vec<f32>>` são strings de texto completo (~500 bytes cada), representando ~50 MB a 100k chunks. Migrar para `chunk_hash` (string hexadecimal de ~64 bytes) reduziria esse overhead de ~50 MB para ~6 MB, desbloqueando o ganho teórico de 8.5x no componente vetorial e simplificando o filtro de `save_all_snapshots`.
