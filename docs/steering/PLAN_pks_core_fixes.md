# PLANO: Correções Estruturais e Otimizações de Memória do PKS

**Data:** Março de 2026
**Contexto:** Este plano foi extraído da análise do épico de quantização vetorial (TurboQuant). Descobrimos que existem problemas arquiteturais atuais (bugs estruturais) que afetam a estabilidade, a precisão e o consumo de recursos (RAM e disco) do motor de busca Híbrida e do gerenciador de estado.

A resolução desses problemas é prioritária em relação a qualquer nova funcionalidade algorítmica ou de quantização agressiva.

---

## 1. Vazamento de Dados e Inchaço nos Snapshots (`save_all_snapshots`)

> ✅ **Corrigido** em commits `a173755` e `36393e2` (Março 2026). `save_all_snapshots()` agora filtra chunks por `repo_id` via `chunks_for_repo_snapshot()`.

### O Problema
No arquivo `state.rs` (por volta da linha 132), a função `save_all_snapshots` está utilizando um filtro `filter(|_| true)` ao iterar sobre o `HashMap` global de vetores.
Isso significa que o PKS está fazendo o dump de **todos os vetores residentes em memória para dentro de cada snapshot de repositório individual**.
- **Consequências:** 
  - Snapshots com tamanho exponencialmente inflado (cada repo guarda a base inteira).
  - Isolamento quebrado (dados de Vault A indo parar no snapshot da Vault B).

### Solução Proposta
1. Alterar a lógica de iteração no momento de salvar os snapshots.
2. É necessário iterar apenas sobre as chaves/valores que de fato pertencem ao `RepoId` sendo salvo.
3. Isso pode ser feito validando cada chunk contra seu `repo_id` de origem.

---

## 2. Perda de Metadados em Hits Puramente Vetoriais (`build_ranked_rrf_results`)

> ✅ **Corrigido** em commit `c05c86e` (Março 2026). RRF fusion agora recupera metadados para vector-only hits.

### O Problema
No arquivo `hybrid.rs` (por volta das linhas 53-55), a função `build_ranked_rrf_results` agrupa os scores de RRF (Reciprocal Rank Fusion) do Tantivy (BM25) e do Dense Retriever.
Quando um `chunk` aparece **apenas no resultado vetorial** (não teve match léxico no Tantivy), o sistema falha em carregar os metadados. A resposta atual atribui `file_path: ""` e `repo_id: ""` para o hit no retorno da API.
- **Consequências:** Experiência quebrada para o LLM final ou usuário que clica no trecho de código, pois os metadados de procedência do trecho vetorial foram descartados.

### Solução Proposta
1. O Retriever semântico deve manter ou ter acesso a um índice/mapa secundário mapeando o id/texto do chunk para as suas propriedades `(file_path, repo_id, heading_hierarchy)`.
2. Como fallback, modificar o agrupador do RRF para sempre recuperar (lookup) os dados de proveniência do chunk correspondente do estado principal (State/Snapshot) quando o Tantivy não retornar aquele texto na sua query.

---

## 3. Sobrecarga Indevida de RAM nas Chaves do HashMap Vetorial

### O Problema
Atualmente, o cache prevalente em memória (`HashMap<String, Vec<f32>>` no Retriever) utiliza o texto completo do chunk (`chunk.text.clone()`) como sua chave (`String`).
Levando em conta uma média de 500 bytes por texto de chunk:
- Para 100.000 chunks, apenas **as chaves do HashMap consomem cerca de 50 MB de RAM**.
- Em escala, o tamanho bruto das chaves começará a competir gravemente pelos limites do processo e afastar o GC/Alocador de alocações amigáveis a CPU cache.

### Solução Proposta
1. O chunking no momento de indexação já provê um campo determinístico curto e imutável para a representação do chunk: o `chunk_hash` (um hash SHA/Hex).
2. Refatorar o dicionário de vetores para `HashMap<String, Vec<f32>>` (onde a chave String é estritamente o `chunk_hash`) ou um tipo ainda mais eficiente (`[u8; 32]` para hashes).
3. Isso reduzirá imediatamente o overhead estimado para 100k chunks de 50 MB para ~6 MB, sem perder a integridade da query.
4. **Dependência:** Garantir a atualização simétrica nos updates, deletes e buscas. Ao passar a query para o LLM e obter o vetor retornado para a busca de similaridade, a chave não precisa ser o texto de busca, já que a busca varre os valores (`f32`) e retorna a chave para um lookup final.

---

## Próximos Passos (Execução)

**Status atual:**
- ✅ **B1 — Vazamento de Dados nos Snapshots (seção 1):** Corrigido (commits `a173755`, `36393e2`).
- 🔲 **B2 — Sobrecarga de RAM nas Chaves do HashMap (seção 3):** Em aberto. Migração de `chunk.text` para `chunk_hash` como chave ainda não realizada.
- ✅ **B3 — Perda de Metadados em Hits Vetoriais (seção 2):** Corrigido (commit `c05c86e`).

**Ação pendente (apenas B2):**
- Avaliar a tipagem do state global para acoplar `chunk_hash` em vez do raw `chunk.text`.
- Refatorar o dicionário de vetores para usar `chunk_hash` como chave (`HashMap<String, Vec<f32>>` ou `HashMap<[u8; 32], Vec<f32>>`).
- Garantir atualização simétrica nos fluxos de insert, update, delete e busca.
- Executar e testar (testes unitários e manuais via CLI) as rotinas de busca vetorial após a migração.
