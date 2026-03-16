# Regra Global: Interação com GUIs via API

> **Diretiva**: Sempre use manipulação programática via API.

## Protocolo de Execução
1. **Prioridade Técnica**: Se a interface (ex: Google Sheets) possui uma API acessível (ex: Google Apps Script), qualquer alteração estrutural, correção de dados ou formatação em massa DEVE ser feita via código. Preferencialmente usando o clasp.
2. **Autonomia de Deploy (CLASP)**: O `clasp` é a ferramenta primária de sincronização entre o código local e o ambiente Google Apps Script. Caso o `clasp` apresente falhas ou perda de configuração, o agente deve priorizar o reparo imediato da ferramenta.
3. **Justificativa**: Alterações via API são atômicas, rastreáveis no código-fonte e imunes a latência de interface ou erros de seletor de DOM. A manutenção da infraestrutura de automação é fundamental para a saúde do projeto.
4. **Otimização de Contexto**: Para QUALQUER operação que envolva leitura de logs, análise de bases de dados ou processamento de arquivos >5KB, utilize mandatoriamente o **`context-mode`** (`ctx_execute`, `ctx_execute_file`, etc.) para evitar poluição da janela de contexto.
5. **Performance de Sandbox**: Recomenda-se o uso do runtime **Bun** (`bun`) para execução de scripts via `ctx_execute`. O Bun oferece um ganho de performance de até 5x em comparação com o Node.js puro, o que acelera significativamente as operações de análise programática.

