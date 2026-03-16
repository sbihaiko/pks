import subprocess
import json
import argparse
import sys
import os
import requests

def get_gh_status():
    try:
        result = subprocess.run(
            ["gh", "run", "list", "--limit", "1", "--json", "displayTitle,url"],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            runs = json.loads(result.stdout)
            if runs:
                return runs[0]
    except:
        pass
    return {"displayTitle": "N/A", "url": "#"}

def get_live_version(project_id):
    """Tenta obter a versão via endpoint HTTP /get_version em múltiplas regiões"""
    regions = ["us-central1", "southamerica-east1"]
    
    for region in regions:
        url = f"https://{region}-{project_id}.cloudfunctions.net/get_version"
        try:
            response = requests.get(url, timeout=5)
            if response.status_code == 200:
                data = response.json()
                return {
                    "status": "UP",
                    "version": data.get("version", "N/A"),
                    "commit": data.get("commit", "N/A"),
                    "deploy_date": data.get("deploy_date", "N/A"),
                    "project": project_id,
                    "is_live": True,
                    "region": region
                }
        except Exception:
            continue
    
    print(f"DEBUG: Failed to fetch live version for {project_id} in all regions")
    return None

def get_gcloud_status(project_id):
    # Primeiro tenta o endpoint ao vivo (mais rápido e preciso)
    live_info = get_live_version(project_id)
    if live_info:
        return live_info

    # Fallback para gcloud cli
    try:
        # Busca todas as funções e filtra no Python para evitar problemas com sintaxe de filtro do gcloud
        result = subprocess.run(
            ["gcloud", "functions", "list", "--format=json", f"--project={project_id}"],
            capture_output=True, text=True, timeout=12
        )
        if result.returncode == 0:
            data = json.loads(result.stdout)
            # Procura por 'get_version' no nome (que pode ser o path completo)
            version_func = next((f for f in data if "get_version" in f.get("name", "")), None)
            
            if version_func:
                # 2nd gen functions store env vars in serviceConfig
                env_vars = version_func.get("serviceConfig", {}).get("environmentVariables", {})
                # 1st gen functions store env vars in buildEnvironmentVariables or similar, 
                # but standard GCP env vars are in environmentVariables
                if not env_vars:
                    env_vars = version_func.get("environmentVariables", {})
                
                return {
                    "status": "UP",
                    "deploy_date": env_vars.get("DEPLOY_DATE", "N/A"),
                    "version": env_vars.get("GIT_TAG", "N/A"),
                    "commit": env_vars.get("GIT_COMMIT", "N/A"),
                    "project": project_id,
                    "is_live": False
                }
            return {"status": "NOT FOUND", "deploy_date": "N/A", "project": project_id, "is_live": False}
        
        # Check stderr for specific issues
        stderr = result.stderr.lower()
        if "reauthentication" in stderr or "login" in stderr:
            return {
                "status": "AUTH REQ", 
                "deploy_date": "N/A", 
                "project": project_id,
                "help": "Execute 'gcloud auth login' no terminal",
                "is_live": False
            }
        
        if "cloudfunctions.googleapis.com" in stderr and "not enabled" in stderr:
             return {
                "status": "NOT ENABLED", 
                "deploy_date": "N/A", 
                "project": project_id,
                "help": "API de Cloud Functions não habilitada neste projeto",
                "is_live": False
            }
            
    except subprocess.TimeoutExpired:
        return {
            "status": "TIMEOUT", 
            "deploy_date": "N/A", 
            "project": project_id,
            "help": "Timeout na CLI do gcloud. API pode estar lenta ou desabilitada.",
            "is_live": False
        }
    except Exception as e:
        print(f"DEBUG: Error checking {project_id}: {e}")
        pass
    
    # Adicionando verificação para Cloud Run se functions não encontradas
    try:
        result = subprocess.run(
            ["gcloud", "run", "services", "list", "--format=json", f"--project={project_id}"],
            capture_output=True, text=True, timeout=10
        )
        if result.returncode == 0:
            services = json.loads(result.stdout)
            if services:
                # Procura por um serviço que tenha get_version ou seja o backend principal
                target_service = next((s for s in services if "get_version" in s['metadata']['name'] or "backend" in s['metadata']['name']), services[0])
                url = target_service.get('status', {}).get('url')
                if url:
                    try:
                        resp = requests.get(f"{url}/get_version", timeout=5)
                        if resp.status_code == 200:
                            data = resp.json()
                            return {
                                "status": "UP",
                                "version": data.get("version", "N/A"),
                                "commit": data.get("commit", "N/A"),
                                "deploy_date": data.get("deploy_date", "N/A"),
                                "project": project_id,
                                "is_live": True,
                                "url": url
                            }
                    except:
                        pass
                
                # Fallback para info do Cloud Run se endpoint falhar
                status_obj = target_service.get('status', {})
                return {
                    "status": "UP",
                    "version": "Cloud Run",
                    "deploy_date": "N/A",
                    "project": project_id,
                    "is_live": False,
                    "url": url
                }
    except Exception as e:
        print(f"DEBUG: Error checking Cloud Run for {project_id}: {e}")
        pass
        
    return {"status": "UNKNOWN", "deploy_date": "N/A", "project": project_id, "is_live": False}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="ci-dashboard/tmp/environments.json")
    args = parser.parse_args()

    # Get Git Info (Local)
    try:
        # Busca dinâmica pela última release estável
        try:
            prod_base = subprocess.check_output(
                ["git", "log", "--grep=release: [0-9]\\+\\.[0-9]\\+\\.0", "-n", "1", "--format=%h"], 
                text=True
            ).strip()
            
            last_tag = subprocess.check_output(
                ["git", "log", "--grep=release: [0-9]\\+\\.[0-9]\\+\\.0", "-n", "1", "--format=%s"], 
                text=True
            ).strip().replace("release: ", "").replace("v", "")
            last_tag = f"v{last_tag}"
        except:
            prod_base = subprocess.check_output(["git", "describe", "--tags", "--abbrev=0"], text=True).strip()
            last_tag = prod_base
        
        commits_ahead = subprocess.check_output(["git", "rev-list", f"{prod_base}..HEAD", "--count"], text=True).strip()
    except:
        last_tag = "v?.?.?"
        commits_ahead = "0"

    # Get Local Version from Version.js
    local_version = "v?"
    possible_version_paths = [
        "src/addon/server/utils/Version.js",
        "src/octaviano/Version.js"
    ]
    for path in possible_version_paths:
        if os.path.exists(path):
            try:
                with open(path, "r") as f:
                    for line in f:
                        if "VERSION_NAME" in line:
                            local_version = line.split("'")[1]
                            break
                if local_version != "v?": break
            except:
                pass

    # Get GH CI status
    gh = get_gh_status()

    # Get Environments status
    stg = get_gcloud_status("stg-octo-v2")
    prod = get_gcloud_status("prod-octo-v2")

    status = {
        "stg": {
            "project_id": "stg-octo-v2",
            "version": stg.get("version") if stg.get("version") != "N/A" else f"release: v{local_version}",
            "commit": stg.get("commit", "N/A"),
            "url": gh["url"],
            "status": stg["status"],
            "last_deploy": stg["deploy_date"],
            "help": stg.get("help", ""),
            "commits_ahead": int(commits_ahead),
            "is_live": stg.get("is_live", False)
        },
        "prod": {
            "project_id": "prod-octo-v2",
            "version": prod.get("version") if prod.get("version") != "N/A" else last_tag,
            "commit": prod.get("commit", "N/A"),
            "status": prod["status"],
            "last_deploy": prod["deploy_date"],
            "help": prod.get("help", ""),
            "is_live": prod.get("is_live", False)
        }
    }

    # Ensure directory exists
    os.makedirs(os.path.dirname(args.output), exist_ok=True)

    with open(args.output, 'w') as f:
        json.dump(status, f, indent=2)
    print(f"Environment status saved to {args.output}")

if __name__ == "__main__":
    main()
