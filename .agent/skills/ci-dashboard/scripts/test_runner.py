from typing import Any, List, Dict, Optional, Tuple, cast, Callable, Union
import subprocess
import json
import os
import sys
import argparse
import glob
import shutil
import threading
import time
import pathlib
import re
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor, as_completed
import signal

def load_config(config_path: str = "ci-dashboard/config.json") -> Dict[str, Any]:
    if not os.path.exists(config_path):
        print(f"⚠️ Config not found at {config_path}, using defaults")
        return {}
    try:
        with open(config_path, 'r') as f:
            return cast(Dict[str, Any], json.load(f))
    except Exception:
        return {}

def _build_pythonpath_env() -> Dict[str, str]:
    """Garante que as pastas de código estejam no PYTHONPATH."""
    env = os.environ.copy()
    cwd = str(pathlib.Path.cwd())
    extra = os.pathsep.join([cwd, os.path.join(cwd, 'src', 'backend'), os.path.join(cwd, 'src')])
    existing = env.get('PYTHONPATH', '')
    env['PYTHONPATH'] = f"{extra}{os.pathsep}{existing}" if existing else extra
    return env

def finalize_status(args: Any) -> None:
    """Define o status final como concluído."""
    try:
        status_file = "ci-dashboard/tmp/status.js"
        timestamp = int(time.time())
        data_str = (
            f'window.CI_DASHBOARD_STATUS = {{'
            f'"status": "idle", '
            f'"progress": 100, '
            f'"message": "Concluído", '
            f'"test_label": "✅ Suite finalizada", '
            f'"eta_label": "Finalizado", '
            f'"timestamp": {timestamp}'
            f'}};'
        )
        with open(status_file, "w") as f:
            f.write(data_str)
        print("✅ Status do dashboard finalizado.")
    except Exception as e:
        print(f"⚠️ Erro ao finalizar status: {e}")

def _trimmed_mean_eta(pct: int, elapsed: float, history_durations: List[float]) -> Optional[float]:
    if pct <= 0 or pct >= 100: return None
    if len(history_durations) >= 3:
        sorted_d = sorted([float(d) for d in history_durations])
        count = len(sorted_d)
        if count > 2:
            trimmed_sum = 0.0
            for i in range(1, count - 1):
                trimmed_sum += float(sorted_d[i])
            avg_total = trimmed_sum / (count - 2)
            remaining = avg_total * (1.0 - float(pct) / 100.0)
            return max(0.0, float(remaining))
    total_est = float(elapsed) / (float(pct) / 100.0)
    return max(0.0, float(total_est - elapsed))

def _parse_pytest_line(line: str, start_t: float, history_durations: List[float]) -> Optional[Tuple[int, str, str]]:
    match = re.search(r'\[\s*(\d+)%\]', line)
    if not match: return None
    pct = int(match.group(1))
    clean_line = str(re.sub(r'\[\s*\d+%\]', '', line).strip())
    # Remove ANSI escape sequences
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    clean_line = str(ansi_escape.sub('', clean_line))
    elapsed = time.time() - start_t
    
    parts = clean_line.split("::")
    label: str = ""
    if len(parts) >= 2:
        file_name = os.path.basename(str(parts[0]).strip())
        method = str(parts[-1]).strip().split(" ")[0]
        icon = "🟢" if "PASSED" in clean_line else ("🔴" if "FAILED" in clean_line else "🟡")
        label = f"{icon} {file_name} ➔ {method}"
    else:
        cl_str: str = str(clean_line)
        if len(cl_str) > 50:
            label = cl_str[0:47] + "..." # type: ignore
        else:
            label = cl_str

    rem = _trimmed_mean_eta(pct, elapsed, history_durations)
    eta = f"ETA {rem / 60:.1f} min" if rem is not None else ""
    return pct, label, eta

def run_backend(config: Dict[str, Any], args: Any) -> None:
    print("\n🧪 Running Backend Tests (Pytest)...")
    test_dirs = config.get("test_directories", {}).get("backend", ["tests/backend"])
    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    
    workers = config.get("max_workers", 4) // 2 or 1
    cmd = [sys.executable, "-m", "pytest", "-n", str(workers)] + test_dirs
    if args.category: cmd.extend(["-m", args.category])
    cmd.extend(["--json-report", f"--json-report-file={args.output}", "--color=yes", "-v"])
    
    if not args.no_cov:
        src_dirs = cast(List[Dict[str, str]], config.get("source_directories", []))
        for src in [s["path"] for s in src_dirs if s.get("type") == "backend"]:
            if os.path.exists(src): cmd.extend([f"--cov={src}"])
        cmd.extend(["--cov-report=json:ci-dashboard/tmp/backend_coverage.json", "--cov-report=html:coverage/backend_html"])
    
    start_t, history = time.time(), []
    try:
        if os.path.exists("ci-dashboard/tmp/history.json"):
            with open("ci-dashboard/tmp/history.json", 'r') as f:
                history = [float(r.get("duration", 0)) for r in json.load(f).get("runs", []) if r.get("duration", 0) > 30]
    except: pass

    def write_status(p, m, l, e):
        try:
            with open("ci-dashboard/tmp/backend_status.json", "w") as f:
                json.dump({"progress": p, "message": m, "test_label": l, "eta": e, "ts": time.time()}, f)
        except: pass

    write_status(0, "Iniciando Pytest...", "", "")
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1, env=_build_pythonpath_env())
    
    out_stream = process.stdout
    if out_stream:
        for line in iter(out_stream.readline, ''):
            if not line: break
            print(line, end="")
            parsed = _parse_pytest_line(line, start_t, history)
            if parsed:
                write_status(int(parsed[0]), f"Backend [{parsed[0]}%]", str(parsed[1]), str(parsed[2]))
    
    if process: process.wait()
    write_status(100, "Concluído", "", "")
def run_rust(config: Dict[str, Any], args: Any) -> None:
    print("\n📦 Running Rust Tests (Cargo)...")
    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    
    # We use cargo test -- --format json if available, but standard cargo output is easier to parse for progress
    # For now, let's run cargo test and parse output
    cmd = ["cargo", "test", "--", "--color", "always"]
    
    def write_status(p, m, l, e):
        try:
            with open("ci-dashboard/tmp/backend_status.json", "w") as f:
                json.dump({"progress": p, "message": m, "test_label": l, "eta": e, "ts": time.time()}, f)
        except: pass

    write_status(0, "Iniciando Cargo test...", "", "")
    
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    
    results = []
    start_t = time.time()
    
    # Simple regex to find test names in cargo output
    # cargo output: "test path::to::test ... ok"
    test_pattern = re.compile(r'test ([\w:]+) \.\.\. (ok|FAILED|ignored)')
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    
    out_stream = process.stdout
    if out_stream:
        for line in iter(out_stream.readline, ''):
            if not line: break
            print(line, end="")
            
            clean_line = ansi_escape.sub('', line)
            match = test_pattern.search(clean_line)
            if match:
                test_name = match.group(1)
                outcome_str = match.group(2)
                outcome = "passed" if outcome_str == "ok" else ("failed" if outcome_str == "FAILED" else "skipped")
                results.append({
                    "nodeid": test_name,
                    "outcome": outcome
                })
                
                # Update progress based on some estimate if we knew total, or just incremental
                total_est = 180 # From my previous check
                pct = min(99, int((len(results) / total_est) * 100))
                write_status(pct, f"Rust [{pct}%]", f"🧪 {test_name}", "")

    if process: process.wait()
    
    # Save results to JSON
    if results:
        with open(args.output, 'w') as f:
            json.dump({"tests": results}, f, indent=2)
            
    write_status(100, "Concluído", "", "")

    # Run coverage if requested
    if not args.no_cov:
        print("\n📊 Collecting Rust Coverage (Tarpaulin)...")
        write_status(100, "Iniciando Tarpaulin...", "📊 Cobertura Rust", "")
        tarp_cmd = [
            "cargo", "tarpaulin", 
            "--out", "Json", 
            "--output-dir", "ci-dashboard/tmp/",
            "--lib", "--bins",
            "--exclude-files", "tests/*"
        ]
        try:
            subprocess.run(tarp_cmd, check=False)
            print("✅ Tarpaulin coverage finished.")
        except Exception as e:
            print(f"⚠️ Tarpaulin failed: {e}")

def run_frontend(config: Dict[str, Any], args: Any) -> None:
    print("\n🚀 Running Frontend Tests (Node/NYC)...")
    test_dirs = cast(List[str], config.get("test_directories", {}).get("frontend", ["tests/frontend"]))
    found: List[str] = []
    for d in test_dirs:
        if os.path.exists(d): 
            for p in ["/**/test-*.js", "/**/*_test.js", "/**/test_*.js", "/**/*.spec.js", "/**/*.test.js"]:
                found.extend(glob.glob(f"{d}{p}", recursive=True))
    
    if not found: 
        print("⚠️ No frontend tests found.")
        return
    
    instr_root = os.path.abspath("ci-dashboard/tmp/instrumented_addon")
    try:
        if os.path.exists(".nyc_output"): shutil.rmtree(".nyc_output")
        if os.path.exists(instr_root): shutil.rmtree(instr_root)
        os.makedirs(instr_root, exist_ok=True)
        for f in ["src", "tests"]: 
            if os.path.exists(f): 
                shutil.copytree(f, os.path.join(instr_root, f), ignore=shutil.ignore_patterns('node_modules', '.git', '__pycache__', '.pytest_cache', '.gradle', '.idea', '.vscode'))
        
        gw_nm = os.path.abspath("src/gateway/node_modules")
        if os.path.exists(gw_nm) and not os.path.exists(os.path.join(instr_root, "src/gateway/node_modules")):
            os.symlink(gw_nm, os.path.join(instr_root, "src/gateway/node_modules"))

        src_paths = cast(List[Dict[str, str]], config.get("source_directories", []))
        for src in [s["path"] for s in src_paths if s.get("type") == "frontend" or "src/tools" in s.get("path", "")]:
            if os.path.exists(src):
                subprocess.run(["npx", "nyc", "instrument", "--extension", ".js", "--extension", ".gs", src, os.path.join(instr_root, src)], check=True)
        exec_tests = [os.path.join(instr_root, tf) for tf in sorted(list(set(found)))]
    except Exception as e: 
        print(f"⚠️ NYC Prep failed: {e}"); exec_tests = found

    results: List[Dict[str, Any]] = []
    ansi = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    def run_test(test_file):
        res = []
        try:
            # Se for backoffice, o SRC_PATH deve apontar para src/gas/backoffice
            sub_path = "src/gas/backoffice" if "backoffice" in test_file else "src/gas/gdocs-addon"
            env = os.environ.copy(); env["SRC_PATH"] = os.path.join(instr_root, sub_path)
            
            is_gw = "gateway" in test_file
            cwd = os.path.join(instr_root, "src/gateway") if is_gw else os.getcwd()
            cmd = ["npx", "nyc", "--silent", "--no-clean", "node"] + (["--test-reporter=spec"] if is_gw else []) + [test_file]
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30, env=env, cwd=cwd)
            out = ansi.sub('', proc.stdout)
            
            # Novo Parser: Tenta encontrar múltiplos emojis se não houver sumário explícito
            summary_m = re.search(r"(?:Resultados: )?(\d+)\s*✅\s*\|\s*(\d+)\s*❌", out) or re.search(r"(?:✅|❌)\s*\((\d+)\s*tests?\)", out)
            
            if summary_m and summary_m.lastindex == 2:
                pc, fc = int(summary_m.group(1)), int(summary_m.group(2))
                for i in range(pc): res.append({"nodeid": f"{test_file}::p_{i}", "outcome": "passed"})
                for i in range(fc): res.append({"nodeid": f"{test_file}::f_{i}", "outcome": "failed", "stderr": proc.stderr})
                print(f"[{os.path.basename(test_file)}] {'✅' if fc==0 else '❌'} ({pc if fc==0 else fc} tests)")
            else:
                # Se não houver sumário, contamos os emojis ✅ e ❌ apenas no início das linhas
                lines = out.splitlines()
                pass_count = sum(1 for line in lines if line.strip().startswith("✅"))
                fail_count = sum(1 for line in lines if line.strip().startswith("❌"))
                
                if pass_count > 0 or fail_count > 0:
                    for i in range(pass_count): res.append({"nodeid": f"{test_file}::p_{i}", "outcome": "passed"})
                    for i in range(fail_count): res.append({"nodeid": f"{test_file}::f_{i}", "outcome": "failed", "stderr": proc.stderr})
                    print(f"[{os.path.basename(test_file)}] {'✅' if fail_count==0 else '❌'} ({pass_count + fail_count} tests)")
                else:
                    outcome = "passed" if proc.returncode == 0 else "failed"
                    res.append({"nodeid": test_file, "outcome": outcome, "stderr": proc.stderr})
                    print(f"[{os.path.basename(test_file)}] {'✅' if outcome=='passed' else '❌'}")
        except Exception as e: res.append({"nodeid": test_file, "outcome": "failed", "stderr": str(e)})
        return res

    def write_status(p, l):
        try:
            with open("ci-dashboard/tmp/frontend_status.json", "w") as f:
                json.dump({"progress": p, "message": f"Frontend [{p}%]", "test_label": l, "ts": time.time()}, f)
        except: pass

    write_status(0, "Iniciando testes frontend...")
    def run_f_test(tf: str) -> List[Dict[str, Any]]:
        return run_test(tf)

    with ThreadPoolExecutor(max_workers=cast(int, config.get("max_workers", 4))) as ex:
        futs = {cast(Any, ex.submit)(run_f_test, tf): tf for tf in exec_tests}
        for i, f in enumerate(as_completed(futs)):
            results.extend(cast(List[Dict[str, Any]], f.result()))
            pct = int(((i + 1) / len(exec_tests)) * 100)
            write_status(pct, f"🧪 {os.path.basename(futs[f])}")
            
    if results:
        os.makedirs(os.path.dirname(args.frontend_output), exist_ok=True)
        with open(args.frontend_output, 'w') as f: json.dump({"tests": results}, f, indent=2)
    try:
        subprocess.run(["npx", "nyc", "report"], capture_output=True)
        if os.path.exists("coverage/frontend_html/coverage-summary.json"):
            shutil.copy("coverage/frontend_html/coverage-summary.json", "ci-dashboard/tmp/frontend_coverage.json")
    except: pass

def run_gateway(config: Dict[str, Any], args: Any) -> None:
    print("\n📦 Running Gateway Tests (Node/C8)...")
    gw_dir = "src/gateway"
    if not os.path.exists(gw_dir):
        return print("⚠️ Gateway directory not found.")
    
    # Busca dinâmica na nova estrutura de testes
    found_tests = glob.glob("tests/gateway/*.test.js")
    test_files = [os.path.relpath(tf, gw_dir) for tf in found_tests]

    def write_status(p: int, l: str):
        try:
            with open("ci-dashboard/tmp/gateway_status.json", "w") as f:
                json.dump({"progress": p, "message": f"Gateway [{p}%]", "test_label": l, "ts": time.time()}, f)
        except: pass

    write_status(0, "Iniciando testes do Gateway...")
    
    # Run with c8 for coverage
    cmd = [
        "npx", "c8", "--include", "src", 
        "--clean", 
        "--temp-dir", ".v8_temp",
        "--reporter=json-summary", 
        "--reporter=html",
        "--report-dir=../../ci-dashboard/tmp/gateway_coverage",
        "node", "--test"
    ] + test_files
    
    results: list[dict[str, str]] = []
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=gw_dir)
    stdout_obj = process.stdout
    
    if stdout_obj:
        for line in iter(stdout_obj.readline, ''):
            if not line: break
            print(line, end="")
            
            # Node reporter output: 'ok 1 - test name' or 'not ok 1 - test name'
            clean_line = line.strip()
            if clean_line.startswith("ok ") or clean_line.startswith("not ok "):
                is_failed = clean_line.startswith("not ok ")
                parts = clean_line.split(" - ")
                if len(parts) > 1:
                    test_name = str(parts[1].split(" #")[0].split(" (")[0].strip())
                else:
                    test_name = clean_line
                
                results.append({
                    "nodeid": f"GW::{test_name}", 
                    "outcome": "failed" if is_failed else "passed"
                })
                
                total_est = 91 
                pct = int(min(99, int((len(results) / total_est) * 100)))
                tn_str_val: str = str(test_name)
                if len(tn_str_val) > 40:
                    disp_label: str = ""
                    for i_t, c_t in enumerate(tn_str_val):
                        if i_t < 37: disp_label += c_t # type: ignore
                    display_name = disp_label + "..." # type: ignore
                else:
                    display_name = tn_str_val
                write_status(pct, f"🧪 {display_name}") # type: ignore
    
    if process: process.wait()
    
    # Save results to JSON for dashboard_generator
    if results:
        os.makedirs(os.path.dirname(args.gateway_output), exist_ok=True)
        try:
            with open(args.gateway_output, 'w') as f:
                json.dump({"tests": results}, f, indent=2)
            print(f"✅ Gateway results saved: {len(results)} tests")
        except Exception as e:
            print(f"⚠️ Error saving gateway results: {e}")
    
    # Sync coverage file to standard location
    src_cov = "ci-dashboard/tmp/gateway_coverage/coverage-summary.json"
    dest_cov = "ci-dashboard/tmp/gateway_coverage.json"
    if os.path.exists(src_cov):
        shutil.copy(src_cov, dest_cov)

    write_status(100, "Concluído")

def generate_tofix_report(args: Any) -> None:
    print("\n📝 Generating TOFIX.md report...")
    lines: List[str] = ["# 🛠️ Tests to Fix\n\n"]
    found_count: int = 0
    for p in [str(args.output), str(args.frontend_output)]:
        if os.path.exists(p):
            try:
                with open(p, 'r') as f:
                    data = json.load(f)
                    items = data if isinstance(data, list) else data.get("tests", [])
                    for item in items:
                        if str(item.get("conclusion")) == "failure" or str(item.get("outcome")) in ["failed", "error"]:
                            lines.append(f"### ❌ {item.get('nodeid', item.get('displayTitle'))}\n")
                            found_count = found_count + 1 # type: ignore
            except: pass
    
    # Always update or clear the report to avoid stale data
    tofix_path = "ci-dashboard/tmp/TOFIX.md"
    if int(found_count) > 0:
        with open(tofix_path, "w") as f: f.write("".join(lines))
    elif os.path.exists(tofix_path):
        os.remove(tofix_path)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--category", choices=["unit", "integration", "sanity", "health"])
    parser.add_argument("--layer", choices=["backend", "frontend", "gateway", "fullstack"])
    parser.add_argument("--output", default="ci-dashboard/tmp/test_results.json")
    parser.add_argument("--frontend-output", default="ci-dashboard/tmp/frontend_results.json")
    parser.add_argument("--gateway-output", default="ci-dashboard/tmp/gateway_results.json")
    parser.add_argument("--no-cov", action="store_true")
    
    args = parser.parse_args()
    config = load_config()
    os.makedirs("ci-dashboard/tmp", exist_ok=True)
    for f in ["backend_status.json", "frontend_status.json", "gateway_status.json"]:
        path = f"ci-dashboard/tmp/{f}"
        if os.path.exists(path): os.remove(path)

    def monitor(stop_evt, args):
        while not stop_evt.is_set():
            bp, fp, gp = 0, 0, 0
            stats = {"BE": (0, "Ag...", ".."), "FE": (0, "Ag...", ".."), "GW": (0, "Ag...", "..")}
            try:
                if os.path.exists("ci-dashboard/tmp/backend_status.json"):
                    with open("ci-dashboard/tmp/backend_status.json", "r") as f:
                        d = json.load(f)
                        stats["BE"] = (d["progress"], d["message"], d["test_label"])
                if os.path.exists("ci-dashboard/tmp/frontend_status.json"):
                    with open("ci-dashboard/tmp/frontend_status.json", "r") as f:
                        d = json.load(f)
                        stats["FE"] = (d["progress"], d["message"], d["test_label"])
                if os.path.exists("ci-dashboard/tmp/gateway_status.json"):
                    with open("ci-dashboard/tmp/gateway_status.json", "r") as f:
                        d = json.load(f)
                        stats["GW"] = (d["progress"], d["message"], d["test_label"])
            except: pass
            
            # Identify active layers based on the requested execution
            active_keys = []
            if args.layer in [None, "backend", "fullstack"]: active_keys.append("BE")
            if args.layer in [None, "frontend", "fullstack"]: active_keys.append("FE")
            if args.layer in [None, "gateway", "fullstack"]: active_keys.append("GW")
            
            # Current process snapshot
            current_stats = [(k, stats[k][0], stats[k][1], stats[k][2]) for k in active_keys]

            # Smooth Progress Calculation using Weighted Average
            # Weights adjusted to reach total tests as indicated by user check
            weights = {"BE": 180, "FE": 10, "GW": 10}
            total_weight = sum(weights[k] for k in active_keys)
            
            if total_weight > 0:
                weighted_sum = sum(stats[k][0] * weights[k] for k in active_keys)
                global_p = weighted_sum / total_weight
            else:
                global_p = 0
            
            # Message and label from the process that is furthest from finishing (bottleneck)
            not_finished = [s for s in current_stats if s[1] < 100]
            if not_finished:
                not_finished.sort(key=lambda x: x[1])
                target = not_finished[0]
                msg = target[2]
                label = f"[{target[0]}] {target[3]}"
            else:
                global_p = 100
                msg = "Finalizando cobertura..."
                label = "Consolidando relatórios"

            # Map to 10-100 range for better UX
            display_p = min(100, 10 + int(global_p * 0.9))
            
            safe_label = label.replace("\"", "\\\"")
            js = f'window.CI_DASHBOARD_STATUS = {{"status": "running", "progress": {display_p}, "message": "{msg}", "test_label": "{safe_label}", "eta_label": "", "timestamp": {int(time.time())}}};'
            try:
                with open("ci-dashboard/tmp/status.js", "w") as f: f.write(js)
            except: pass
            time.sleep(0.4)

    stop_evt = threading.Event()
    mon = threading.Thread(target=monitor, args=(stop_evt, args), daemon=True)
    mon.start()

    try:
        # Usamos 3 workers se for fullstack
        max_workers = 3 if args.layer in [None, "fullstack"] else 1
        with ProcessPoolExecutor(max_workers=max_workers) as ex:
            tasks = []
            if args.layer in [None, "backend", "fullstack"]: 
                if os.path.exists("Cargo.toml"):
                    tasks.append(ex.submit(cast(Callable[..., Any], run_rust), config, args))
                else:
                    tasks.append(ex.submit(cast(Callable[..., Any], run_backend), config, args))
            if args.layer in [None, "frontend", "fullstack"]: 
                tasks.append(ex.submit(cast(Callable[..., Any], run_frontend), config, args))
            if args.layer in [None, "gateway", "fullstack"]: 
                tasks.append(ex.submit(cast(Callable[..., Any], run_gateway), config, args))
            for t in tasks: t.result()
    finally:
        stop_evt.set(); mon.join(timeout=1)
    
    generate_tofix_report(args)
    finalize_status(args)

if __name__ == "__main__":
    main()
