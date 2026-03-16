---
description: fazer release e criar tag
---

1. Verifique se o código foi subido para o repositório via `git status`. Se houver mudanças, comite e suba com `git push`.
2. Verifique se há novos commits desde a última tag usando `git log $(git describe --tags --abbrev=0)..main --oneline`. Se o resultado for vazio, avise ao usuário que não há nada novo para release e pare por aqui.
3. Verifique se o CI (GitHub Actions) passou usando o comando `gh run list --branch main --limit 1`.
4. Execute o script de `release.sh` (opcionalmente passando a versão como argumento, ex: `./release.sh v4.7.9`).