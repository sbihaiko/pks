use crate::ipc::{IpcClient, PksCommand, PksResponse};

pub async fn run_remove(repo_id: &str) -> i32 {
    if repo_id.is_empty() {
        eprintln!("✗ Uso: pks remove <repo_id>");
        eprintln!("  Use `pks search` ou `list_knowledge_vaults` para ver os IDs disponíveis.");
        return 1;
    }

    let cmd = PksCommand::Remove { repo_id: repo_id.to_string() };

    match IpcClient::send_command(&cmd).await {
        Ok(PksResponse::RemoveDone { repo_id }) => {
            println!("✓ Vault removido: {repo_id}");
            0
        }
        Ok(PksResponse::Err { message }) => {
            eprintln!("✗ Erro: {message}");
            1
        }
        Ok(_) => {
            eprintln!("✗ Resposta inesperada do daemon");
            1
        }
        Err(e) => {
            eprintln!("✗ Erro ao conectar ao daemon: {e}");
            eprintln!("  Verifique se o daemon está rodando com `pks --daemon` ou `pks status`.");
            1
        }
    }
}

/// Remove a vault locally (without daemon) by deleting its .pks/ directory.
pub fn run_remove_local(path: &std::path::Path) -> i32 {
    let pks_dir = path.join(".pks");
    if !pks_dir.exists() {
        eprintln!("✗ Nenhum vault PKS encontrado em {}", path.display());
        return 1;
    }
    match std::fs::remove_dir_all(&pks_dir) {
        Ok(()) => {
            println!("✓ Vault removido: {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("✗ Erro ao remover {}: {e}", pks_dir.display());
            1
        }
    }
}
