import json
import os
import argparse
import shutil
import subprocess
from datetime import datetime
import math
from typing import Any, List, Dict, Optional, Tuple, cast, Union

def _round(val: Any, prec: int = 1) -> float:
    try:
        fval = float(val)
        return float(f"{fval:.{prec}f}")
    except:
        return 0.0

def _slice_list(l: Any, start: int = 0, end: Optional[int] = None) -> list:
    res = []
    lst = cast(list, l)
    s_idx = max(0, start)
    e_idx = len(lst) if end is None else end
    for i in range(s_idx, min(len(lst), e_idx)):
        res.append(lst[i])
    return res

def _slice_str(s: Any, start: int = 0, end: Optional[int] = None) -> str:
    ss = str(s)
    s_idx = max(0, start)
    e_idx = len(ss) if end is None else end
    res = ""
    for i in range(s_idx, min(len(ss), e_idx)):
        res += ss[i]
    return res

def get_git_remote_url() -> str:
    try:
        url_bytes = subprocess.check_output(["git", "config", "--get", "remote.origin.url"], 
                                           stderr=subprocess.DEVNULL)
        url_raw = url_bytes.decode("utf-8").strip()
        url_str = str(url_raw)
        if url_str.endswith(".git"):
            url_str = _slice_str(url_str, 0, len(url_str) - 4)
        if url_str.startswith("git@github.com:"):
            url_str = "https://github.com/" + _slice_str(url_str, len("git@github.com:"))
        return url_str
    except Exception:
        return "#"

# Simple template engine in case Jinja2 is not available
def render_template(template_path: str, context: dict) -> str:
    with open(template_path, 'r', encoding='utf-8') as f:
        template = str(f.read())
    
    # Simple replacement for basics
    for key, value in context.items():
        if isinstance(value, (int, float, str)):
            template = str(template).replace('{{ ' + str(key) + ' }}', str(value))
    
    # Try to use Jinja2 if available, otherwise fallback to simple replacement
    try:
        # Standardize import for linting
        import jinja2 # type: ignore
        t_dir = os.path.dirname(template_path)
        t_file = os.path.basename(template_path)
        l_dr = jinja2.FileSystemLoader(t_dir)
        j_env = jinja2.Environment(loader=l_dr)
        j_tmpl = j_env.get_template(t_file)
        return str(j_tmpl.render(context))
    except (ImportError, Exception):
        print("Warning: jinja2 issues. Using primitive replacement (loops will be empty).")
        return template

def _process_file_for_tree(root: str, file: str, detailed_data: dict, coverage_url_map: dict, sections: list, repo_base_url: Optional[str] = None) -> None:
    if file.endswith(('.py', '.js', '.gs', '.ts', '.jsx', '.tsx')) and not file.endswith('__init__.py'):
        rel_path = str(os.path.normpath(os.path.relpath(os.path.join(root, file), ".")).replace("\\", "/"))
        
        # Determine section based on path prefix
        target_section = None
        for section in sections:
            sec_path = str(section["meta"].get("path", ""))
            # Normalize for comparison
            if rel_path == sec_path or rel_path.startswith(sec_path + os.sep):
                target_section = section
                break
        
        if not target_section:
            return
        
        # Get coverage info if exists
        keys_to_try = [
            rel_path,
            rel_path.replace("src/", ""),
            rel_path.replace("backend-api/", ""),
            rel_path.replace("src/backend-api/", ""),
            os.path.basename(rel_path)
        ]
        info = None
        for k in keys_to_try:
            if k in detailed_data:
                info = cast(dict, detailed_data[k])
                break

        url = str(coverage_url_map.get(rel_path, ""))
        
        if not url:
            variations = [
                rel_path.replace("src/", ""),
                rel_path.replace("src/backend/", "src/backend-api/"),
                rel_path.replace("src/backend-api/", "src/backend/"),
                rel_path.replace("backend/", "backend-api/"),
                rel_path.replace("backend-api/", "backend/"),
                "src/" + rel_path if not rel_path.startswith("src/") else rel_path
            ]
            for v in variations:
                if v in coverage_url_map:
                    url = str(coverage_url_map[v])
                    break
        
        github_url = f"{repo_base_url.rstrip('/')}/blob/main/{rel_path}" if repo_base_url else "#"
        if not url:
            url = github_url

        path_parts = []
        t_meta = cast(dict, cast(dict, target_section).get("meta", {}))
        full_prefix = str(t_meta.get("path", ""))
        
        if full_prefix and rel_path.startswith(full_prefix + os.sep):
             path_parts.append(full_prefix)
             r_slice = _slice_str(str(rel_path), int(len(full_prefix))+1)
             path_parts.extend(str(r_slice).split('/'))
        elif rel_path == full_prefix:
             path_parts.append(full_prefix)
        else:
             path_parts = str(rel_path).split('/')

        current_level = cast(dict, cast(dict, target_section).get("tree", {}))
        
        path_parts_list = cast(list, path_parts)
        for i in range(len(path_parts_list) - 1):
            part = str(path_parts_list[i])
            if part not in current_level:
                current_level[part] = {"type": "pkg", "children": {}, "percent": 0.0}
            node_p = cast(dict, current_level[part])
            current_level = cast(dict, node_p["children"])
        
        filename = path_parts[-1]
        percent = 0.0
        missed = 0
        total = 0
        classes: dict = {}
        functions: dict = {}
        
        if info:
            s_info = cast(dict, cast(dict, info).get("summary", {}))
            percent = _round(s_info.get("percent_covered", 0.0))
            total = int(float(str(s_info.get("num_statements", s_info.get("total", 0)))))
            # Safe extraction of missed lines
            m_raw = s_info.get("num_missed", s_info.get("missing_lines", 0))
            if isinstance(m_raw, list):
                missed = len(m_raw)
            else:
                try: missed = int(float(str(m_raw)))
                except: missed = 0
            
            if missed == 0 and total > 0 and percent < 100:
                 missed = int(float(total) * (1.0 - (float(percent)/100.0)))

            info_classes = cast(dict, cast(dict, info).get("classes", {}))
            for class_name, class_info in info_classes.items():
                if class_name:
                    c_info = cast(dict, class_info)
                    c_percent = _round(c_info.get("summary", {}).get("percent_covered", 0.0))
                    classes[class_name] = {"percent": c_percent}

            info_funcs = cast(dict, info.get("functions", {}))
            for func_name, func_info in info_funcs.items():
                if func_name and str(func_name) != "":
                    f_info = cast(dict, func_info)
                    f_percent = _round(f_info.get("summary", {}).get("percent_covered", 0.0))
                    functions[str(func_name)] = {"percent": f_percent}
        
        cast(dict, current_level)[filename] = {
            "type": "file",
            "percent": percent,
            "missed_lines": missed,
            "total_lines": total,
            "url": url if url else "#",
            "github_url": github_url,
            "rel_path": rel_path,
            "classes": classes,
            "functions": functions
        }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="ci-dashboard/config.json", help="Path to config.json with project metadata")
    parser.add_argument("--test-results", default="ci-dashboard/tmp/test_results.json")
    parser.add_argument("--coverage", default="ci-dashboard/tmp/coverage_summary.json")
    parser.add_argument("--backend-coverage", default="ci-dashboard/tmp/backend_coverage.json")
    parser.add_argument("--frontend-coverage", default="ci-dashboard/tmp/frontend_coverage.json")
    parser.add_argument("--gateway-coverage", default="ci-dashboard/tmp/gateway_coverage.json")
    parser.add_argument("--ci-status", default="ci-dashboard/tmp/ci.json")
    parser.add_argument("--risks", default="ci-dashboard/tmp/risks.json")
    parser.add_argument("--history", default="ci-dashboard/tmp/history.json")
    parser.add_argument("--env-status", default="ci-dashboard/tmp/environments.json")
    parser.add_argument("--frontend-results", default="ci-dashboard/tmp/frontend_results.json")
    parser.add_argument("--gateway-results", default="ci-dashboard/tmp/gateway_results.json")
    parser.add_argument("--output", default="ci-dashboard/tmp/dashboard.html")
    args = parser.parse_args()
    
    # Load Project Config
    project_config = {
        "project_name": "Project",
        "project_subtitle": "Test Dashboard",
        "repo_url": "#"
    }
    
    if args.config and os.path.exists(args.config):
        try:
            with open(args.config, 'r') as f:
                loaded = json.load(f)
                project_config.update(loaded)
        except: pass
    
    # Auto-detect repo_url if still default
    if project_config["repo_url"] == "#":
        detected_url = get_git_remote_url()
        if detected_url != "#":
            print(f"📡 Git URL detectada: {detected_url}")
            project_config["repo_url"] = detected_url
        else:
            print("⚠️ AVISO: repo_url não está configurado e não foi detectado automaticamente. Links do GitHub estarão quebrados.")

    context = {
        "generation_time": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "project_name": project_config["project_name"],
        "project_subtitle": project_config["project_subtitle"],
        "repo_url": project_config["repo_url"],
        "tests": {"total": 0, "passed": 0, "failed": 0, "pass_rate": 0, 
                  "be_rate": 0, "fe_rate": 0, "hl_rate": 0,
                  "be_passed": 0, "fe_passed": 0, "hl_passed": 0,
                  "be_total": 0, "fe_total": 0, "hl_total": 0},
        "coverage": {"backend": 0, "frontend": 0, "gateway": 0, "total": 0},
        "reports": {
            "backend": "coverage/backend_html/index.html", 
            "frontend": "coverage/frontend_html/index.html",
            "gateway": "ci-dashboard/tmp/gateway_coverage/index.html"
        },
        "ci_runs": [],
        "risks": [],
        "risks_total": 0,
        "top_large": [],
        "top_complex": [],
        "top_breaking": [],
        "history": [],
        "env_status": {"stg": {"status": "N/A"}, "prod": {"status": "N/A"}}
    }
    
    current_tests_map = {}
    test_layers_map = {} # nodeid -> 'BE', 'FE'

    # Load Test Results (Backend)
    if os.path.exists(args.test_results):
        try:
            with open(args.test_results, 'r') as f:
                data = json.load(f)
                for t in data.get("tests", []):
                    outcome = t.get("outcome", "none").lower()
                    nodeid = t["nodeid"]
                    current_tests_map[nodeid] = outcome
                    
                    if outcome == "failed":
                        err_msg = ""
                        if "call" in t and "longrepr" in t["call"]:
                            err_msg = t["call"]["longrepr"]
                        elif "setup" in t and "longrepr" in t["setup"]:
                            err_msg = t["setup"]["longrepr"]
                        if err_msg:
                            current_tests_map[nodeid + "__errormsg__"] = err_msg
                    
                    test_layers_map[nodeid] = "BE"
        except: pass

    # Load Frontend Results
    if os.path.exists(args.frontend_results):
        try:
            with open(args.frontend_results, 'r') as f:
                fe_data = json.load(f)
                fe_tests = fe_data if isinstance(fe_data, list) else fe_data.get("tests", [])
                
                fe_nodeid_counts = {}
                for t in fe_tests:
                    outcome = t.get("outcome", "none").lower()
                    raw_nodeid = t["nodeid"]
                    
                    if raw_nodeid not in fe_nodeid_counts:
                        fe_nodeid_counts[raw_nodeid] = 0
                    fe_nodeid_counts[raw_nodeid] += 1
                    
                    final_nodeid = raw_nodeid
                    if fe_nodeid_counts[raw_nodeid] > 1:
                        final_nodeid = f"{raw_nodeid}::{fe_nodeid_counts[raw_nodeid]}"
                        
                    current_tests_map[final_nodeid] = outcome
                    if outcome == "failed" and t.get("message"):
                        current_tests_map[final_nodeid + "__errormsg__"] = t["message"]
                    
                    test_layers_map[final_nodeid] = "FE"
        except: pass

    # Load Gateway Results
    if os.path.exists(args.gateway_results):
        try:
            with open(args.gateway_results, 'r') as f:
                gw_data = json.load(f)
                gw_tests = gw_data if isinstance(gw_data, list) else gw_data.get("tests", [])
                
                for t in gw_tests:
                    outcome = t.get("outcome", "none").lower()
                    nodeid = t.get("nodeid", "unknown_gw")
                    msg = t.get("message", "")
                    
                    # Special handling for aggregated gateway results
                    if nodeid == "gateway_suite" and "tests passed" in msg:
                        try:
                            # Extract number from "X tests passed"
                            actual_count = int(msg.split()[0])
                            for i in range(actual_count):
                                current_tests_map[f"{nodeid}_{i}"] = outcome
                            test_layers_map[nodeid] = "GW" # Key layer by original nodeid prefix or similar
                            continue
                        except: pass

                    current_tests_map[nodeid] = outcome
                    if outcome in ["failed", "error"] and msg:
                        current_tests_map[nodeid + "__errormsg__"] = msg
                    
                    test_layers_map[nodeid] = "GW"
        except: pass

    # Update History
    history_data = {"runs": [], "tests": {}}
    if os.path.exists(args.history):
        try:
            with open(args.history, 'r') as f:
                history_data = json.load(f)
                if isinstance(history_data, list):
                    history_data = {"runs": history_data, "tests": {}}
        except: pass
    
    total_run_int: int = 0
    passed_run_int: int = 0
    
    tr: int = 0
    pr: int = 0
    current_tests_map_casted = cast(dict, current_tests_map)
    for k_t, v_t in current_tests_map_casted.items():
        if not str(k_t).endswith("__errormsg__"):
            tr = cast(int, tr) + 1
            if str(v_t) == "passed":
                pr = cast(int, pr) + 1
    
    final_tr: int = tr
    final_pr: int = pr

    c_tests = cast(dict, cast(dict, context).get("tests", {}))
    c_tests["total"] = int(final_tr)
    c_tests["passed"] = int(final_pr)
    c_tests["failed"] = int(final_tr) - int(final_pr)
    cast(dict, context)["total_tests_count"] = int(final_tr)
    
    prate: float = 0.0
    if final_tr > 0:
        prate = _round((float(final_pr) / float(final_tr)) * 100.0)
    c_tests["pass_rate"] = prate

    current_run = {
        "time": str(context["generation_time"]),
        "passed": int(final_pr),
        "total": int(final_tr),
        "pass_rate": prate
    }
    
    history_data_casted = cast(dict, history_data)
    history_runs = cast(list, history_data_casted.get("runs", []))
    history_tests = cast(dict, history_data_casted.get("tests", {}))
    
    if not history_runs or str(history_runs[0].get("time")) != current_run["time"]:
        history_runs.insert(0, current_run)
        for nid_loop, outcome_loop in current_tests_map_casted.items():
            nid_key = str(nid_loop)
            if not nid_key.endswith("__errormsg__"):
                nid_hist = cast(list, history_tests.get(nid_key, []))
                nid_hist.insert(0, str(outcome_loop))
                history_tests[nid_key] = _slice_list(nid_hist, 0, 5)
        
        for nid in list(history_tests.keys()):
            if nid not in current_tests_map_casted:
                nid_list_2 = cast(list, history_tests.get(nid, []))
                nid_list_2.insert(0, "none")
                history_tests[nid] = _slice_list(nid_list_2, 0, 5)

    history_data_casted["runs"] = _slice_list(history_runs, 0, 10)
    with open(args.history, 'w') as f:
        json.dump(history_data_casted, f, indent=2)
    context["history"] = history_runs

    # Warnings Processing
    warnings_data = {}
    if os.path.exists(args.test_results):
        try:
            with open(args.test_results, 'r') as f:
                res_json = json.load(f)
                for w in res_json.get("warnings", []):
                    f_path = w.get("filename", "")
                    if f_path:
                        warning_obj = {
                            "message": w.get("message", "No message"),
                            "lineno": w.get("lineno", 0),
                            "category": w.get("category", "Warning"),
                            "nodeid": w.get("nodeid")
                        }
                        if f_path not in warnings_data:
                            warnings_data[f_path] = []
                        warnings_data[f_path].append(warning_obj)
        except: pass

    # Group tests by file
    file_to_tests = {}
    current_res_tests = []
    if os.path.exists(args.test_results):
        try:
            with open(args.test_results, 'r') as f:
                current_res_tests = json.load(f).get("tests", [])
        except: pass
    res_tests_by_nodeid = {t["nodeid"]: t for t in current_res_tests}

    file_to_tests_casted = cast(dict, file_to_tests)
    for nodeid, results in cast(dict, history_tests).items():
        parts = str(nodeid).split("::")
        filepath = parts[0]
        method_name = " :: ".join(_slice_list(cast(list, parts), 1)) if len(parts) > 1 else parts[0]
        
        test_lineno = 0
        if str(nodeid) in res_tests_by_nodeid:
            test_lineno = int(cast(dict, res_tests_by_nodeid[str(nodeid)]).get("lineno", 0))

        if filepath not in file_to_tests_casted:
            file_to_tests_casted[filepath] = []
        
        file_to_tests_casted[filepath].append({
            "nodeid": str(nodeid),
            "name": str(method_name),
            "lineno": test_lineno,
            "results": cast(list, results),
            "method_suffix": str(parts[-1]) if len(parts) > 1 else ""
        })

    grouped_tests = {}
    for filepath, tests in file_to_tests.items():
        sorted_tests = sorted([t for t in tests if t["lineno"] > 0], key=lambda x: x["lineno"])
        file_warnings_all = []
        for w_abs_path, w_list in warnings_data.items():
            if filepath in w_abs_path:
                file_warnings_all = w_list
                break
        
        test_specific_warnings = {t["nodeid"]: [] for t in tests}
        test_specific_warnings_casted = cast(dict, test_specific_warnings)
        unclaimed_warnings = []
        for w in file_warnings_all:
            claimed = False
            w_dict = cast(dict, w)
            w_nodeid = str(w_dict.get("nodeid", ""))
            if w_nodeid and w_nodeid in test_specific_warnings_casted:
                cast(list, test_specific_warnings_casted[w_nodeid]).append(w_dict)
                claimed = True
            if not claimed:
                for t in tests:
                    t_dict = cast(dict, t)
                    if t_dict.get("method_suffix") and (str(t_dict["method_suffix"]) in str(w_dict["message"])):
                        t_nodeid = str(t_dict["nodeid"])
                        cast(list, test_specific_warnings_casted[t_nodeid]).append(w_dict)
                        claimed = True
                        break
            if not claimed and int(w_dict.get("lineno", 0)) > 0 and sorted_tests:
                for i, t in enumerate(sorted_tests):
                    t_dict_inner = cast(dict, t)
                    next_t_lineno = float(cast(dict, sorted_tests[i+1]).get("lineno", float('inf'))) if i+1 < len(sorted_tests) else float('inf')
                    if float(t_dict_inner.get("lineno", 0)) <= float(w_dict["lineno"]) < next_t_lineno:
                        t_nodeid_inner = str(t_dict_inner["nodeid"])
                        cast(list, test_specific_warnings_casted[t_nodeid_inner]).append(w_dict)
                        claimed = True
                        break
            if not claimed:
                unclaimed_warnings.append(w_dict)

        unique_unclaimed = []
        seen_msgs = set()
        for w in unclaimed_warnings:
            if str(w["message"]) not in seen_msgs:
                unique_unclaimed.append(w)
                seen_msgs.add(str(w["message"]))

        grouped_tests[filepath] = {
            "tests": [],
            "file_history": ["none"] * 5,
            "last_failure_ts": 0.0,
            "file_warnings": unique_unclaimed,
            "has_warnings": len(file_warnings_all) > 0
        }
        
        for t_data in tests:
            nodeid = t_data["nodeid"]
            results = t_data["results"]
            padded = _slice_list(cast(list, results) + ["none"] * 5, 0, 5)
            
            for i, res in enumerate(padded):
                hist_node = cast(list, cast(dict, grouped_tests[filepath])["file_history"])
                if res == "failed":
                    hist_node[i] = "failed"
                elif res == "passed" and hist_node[i] != "failed":
                    hist_node[i] = "passed"
            
            for i, res in enumerate(results):
                if res == "failed" and i < len(history_runs):
                    try:
                        run_info = cast(dict, history_runs[i])
                        dt = datetime.strptime(str(run_info["time"]), "%Y-%m-%d %H:%M:%S")
                        ts = float(dt.timestamp())
                        g_node = cast(dict, grouped_tests[filepath])
                        if ts > float(str(g_node.get("last_failure_ts", 0.0))):
                            g_node["last_failure_ts"] = ts
                    except: pass
                    break

            is_rust = project_config.get("project_type") == "rust"
            layer = str(test_layers_map.get(nodeid, "BE" if (filepath.endswith((".py", ".rs")) or is_rust) else "FE"))
            test_name = str(t_data["name"])
            if not test_name.startswith(("BE: ", "FE: ", "HL: ")):
                test_name = f"{layer}: {test_name}"
            
            cast(list, cast(dict, grouped_tests[filepath])["tests"]).append({
                "nodeid": nodeid,
                "name": test_name,
                "history": padded,
                "current": results[0] if results else "none",
                "error_message": str(current_tests_map_casted.get(str(nodeid) + "__errormsg__", "")),
                "warnings": cast(dict, test_specific_warnings_casted).get(str(nodeid), []),
                "layer": layer
            })

    test_groups_data = []
    for filepath, data in grouped_tests.items():
        active_tests = [t for t in data["tests"] if t["current"] != "none"]
        if not active_tests: continue
        
        any_failed = any(t["current"] == "failed" for t in active_tests)
        was_recently_failed = any(any(r == "failed" for r in t["history"][1:]) for t in active_tests)
        is_rust = project_config.get("project_type") == "rust"
        if filepath.endswith((".py", ".rs")) or is_rust:
            layer = "BE"
        else:
            layer = "FE"
            
        test_groups_data.append({
            "filepath": filepath,
            "filename": os.path.basename(filepath),
            "directory": os.path.dirname(filepath),
            "layer": layer,
            "tests": sorted(active_tests, key=lambda x: x["name"]),
            "any_failed": any_failed,
            "has_warnings": data["has_warnings"],
            "file_warnings": data["file_warnings"],
            "was_recently_failed": was_recently_failed,
            "last_failure_ts": data["last_failure_ts"],
            "file_history": data["file_history"]
        })

    def sort_key(g):
        return (0 if g["any_failed"] else 1, 0 if g["has_warnings"] else 1, 0 if g["was_recently_failed"] else 1, -g["last_failure_ts"], g["filepath"])
        
    context["test_groups"] = sorted(test_groups_data, key=sort_key)

    # Breakdown by layer
    b_t: int = 0
    b_p: int = 0
    f_t: int = 0
    f_p: int = 0
    h_t: int = 0
    h_p: int = 0
    
    for g_item in test_groups_data:
        g_t_val = int(len(cast(list, g_item["tests"])))
        g_p_val = int(sum(1 for t_l in cast(list, g_item["tests"]) if str(cast(dict, t_l).get("current")) == "passed"))
        lay_name = str(g_item.get("layer", ""))
        if lay_name == "BE":
            b_t = cast(int, b_t) + g_t_val
            b_p = cast(int, b_p) + g_p_val
        elif lay_name == "FE":
            f_t = cast(int, f_t) + g_t_val
            f_p = cast(int, f_p) + g_p_val
        elif lay_name == "HL":
            h_t = cast(int, h_t) + g_t_val
            h_p = cast(int, h_p) + g_p_val

    c_tests_ptr = cast(dict, cast(dict, context)["tests"])
    c_tests_ptr["be_total"] = b_t
    c_tests_ptr["be_passed"] = b_p
    c_tests_ptr["be_rate"] = _round(float(b_p) / float(b_t) * 100.0) if b_t > 0 else 100.0
    c_tests_ptr["fe_total"] = f_t
    c_tests_ptr["fe_passed"] = f_p
    c_tests_ptr["fe_rate"] = _round(float(f_p) / float(f_t) * 100.0) if f_t > 0 else 100.0
    c_tests_ptr["hl_total"] = h_t
    c_tests_ptr["hl_passed"] = h_p
    c_tests_ptr["hl_rate"] = _round(float(h_p) / float(h_t) * 100.0) if h_t > 0 else 100.0

    # Load Coverage Summary
    total_covered_lines: int = 0
    total_statements: int = 0
    
    backend_file_coverage = {}
    frontend_file_coverage = {}
    if os.path.exists(args.coverage):
        try:
            with open(args.coverage, 'r') as f:
                data = cast(dict, json.load(f))
                context_coverage = cast(dict, context["coverage"])
                context_coverage["backend"] = _round(data.get("backend", {}).get("percentage", 0.0))
                context_coverage["frontend"] = _round(data.get("frontend", {}).get("percentage", 0.0))
                context_coverage["gateway"] = _round(data.get("gateway", {}).get("percentage", 0.0))
        except: pass

    # Fallback/Refinement for Backend Coverage from specific file
    if os.path.exists(args.backend_coverage):
        try:
            with open(args.backend_coverage, 'r') as f:
                data = json.load(f)
                
                # Cobertura global (total de tudo medido) para o denominador do total
                be_summary = data.get("totals", {})
                be_cov_count = int(be_summary.get("covered_lines", 0))
                be_total_count = int(be_summary.get("num_statements", 0))
                
                total_covered_lines = int(total_covered_lines) + int(be_cov_count)
                total_statements = int(total_statements) + int(be_total_count)
                
                # Métrica "Backend" no card = apenas src/backend (exclui src/ops, src/tools)
                # para refletir a saúde da API, não de scripts de infra.
                core_covered_v = 0
                core_total_v = 0
                be_files_v = cast(dict, data.get("files", {}))
                for filepath_v, file_info_v in be_files_v.items():
                    file_info_dict_v = cast(dict, file_info_v)
                    cov_pct_v = float(file_info_dict_v.get("summary", {}).get("percent_covered", 0.0))
                    backend_file_coverage[str(filepath_v)] = cov_pct_v
                    if str(filepath_v).startswith("backend-api/") or str(filepath_v).startswith("src/backend-api/"):
                        backend_file_coverage[str(filepath_v).replace("backend-api/", "").replace("src/backend-api/", "")] = cov_pct_v
                    
                    # Acumula só arquivos da pasta principal do backend (src/backend ou app/)
                    is_core_backend = any(term in str(filepath_v) for term in ["src/backend", "src/ops", "src/tools", "app/", "backend-api"])
                    if is_core_backend:
                        f_sum_v = cast(dict, file_info_dict_v.get("summary", {}))
                        core_covered_v = cast(int, core_covered_v) + int(f_sum_v.get("covered_lines", 0))
                        core_total_v = cast(int, core_total_v) + int(f_sum_v.get("num_statements", 0))
                
                coverage_map_v = cast(dict, context["coverage"])
                if core_total_v > 0:
                    coverage_map_v["backend"] = _round(float(core_covered_v) / float(core_total_v) * 100.0)
                elif be_total_count > 0:
                    coverage_map_v["backend"] = _round(float(be_cov_count) / float(be_total_count) * 100.0)
        except: pass


    # Refinement for Frontend Coverage from specific file (usually summary.json format)
    if os.path.exists(args.frontend_coverage):
        try:
            with open(args.frontend_coverage, 'r') as f:
                data = json.load(f)
                fe_summary = cast(dict, cast(dict, data.get("total", {})).get("lines", {}))
                fe_covered: int = int(fe_summary.get("covered", 0))
                fe_total: int = int(fe_summary.get("total", 0))
                
                total_covered_lines = int(total_covered_lines) + int(fe_covered)
                total_statements = int(total_statements) + int(fe_total)
                
                # Seperate frontend file coverage
                for filepath, file_info in data.items():
                    if filepath == "total": continue
                    rel_path = filepath
                    if 'src/' in rel_path:
                        rel_path = rel_path[str(rel_path).find('src/'):]
                    frontend_file_coverage[str(rel_path)] = float(cast(dict, file_info.get("lines", {})).get("pct", 0.0))
                
                # If frontend coverage in context is still 0, update it
                coverage_map_fe = cast(dict, context["coverage"])
                if coverage_map_fe["frontend"] == 0 and fe_total > 0:
                    coverage_map_fe["frontend"] = _round(float(int(fe_covered) / int(fe_total) * 100.0))
        except: pass

    # Refinement for Gateway Coverage
    if os.path.exists(args.gateway_coverage):
        try:
            with open(args.gateway_coverage, 'r') as f:
                data = json.load(f)
                
                # Suporte para formato Istanbul (total.lines)
                gw_sum = cast(dict, cast(dict, data.get("total", {})).get("lines", {}))
                if gw_sum:
                    gw_covered: int = int(gw_sum.get("covered", 0))
                    gw_total_count: int = int(gw_sum.get("total", 0))
                    total_covered_lines = int(total_covered_lines) + int(gw_covered)
                    total_statements = int(total_statements) + int(gw_total_count)
                    
                    coverage_map_gw = cast(dict, context["coverage"])
                    if (coverage_map_gw.get("gateway") == 0 or coverage_map_gw.get("gateway") == 0.0) and gw_total_count > 0:
                        coverage_map_gw["gateway"] = _round(float(int(gw_covered) / int(gw_total_count) * 100.0))
                
                # Suporte para formato V8/c8 (lista de arquivos ou resumo diferente)
                elif isinstance(data, list) or "result" in data:
                    # Se for lista do V8, o c8 costuma salvar o relatório final text/json em outro lugar
                    # mas vamos tentar extrair se houver um sumário
                    pass
                
                # Register report path for beautification
                if "reports" not in context: context["reports"] = {}
                cast(dict, context["reports"])["gateway"] = "ci-dashboard/tmp/gateway_coverage/index.html"
        except: pass

    # Calculate Total Coverage (Weighted if possible)
    context_coverage_final = cast(dict, context["coverage"])
    if total_statements > 0:
        context_coverage_final["total"] = _round(float(int(total_covered_lines) / int(total_statements) * 100.0))
    else:
        # Fallback to simple average if counts not available
        vals = [float(v) for v in [context_coverage_final.get("backend", 0), context_coverage_final.get("frontend", 0), context_coverage_final.get("gateway", 0)] if float(v) > 0]
        if vals:
             context_coverage_final["total"] = _round(float(sum(vals) / len(vals)))
        else:
             context_coverage_final["total"] = 0.0

    # Load CI Status
    if os.path.exists(args.ci_status):
        try:
            with open(args.ci_status, 'r') as f:
                context["ci_runs"] = json.load(f)
        except: pass

    # Load Env Status
    if os.path.exists(args.env_status):
        try:
            with open(args.env_status, 'r') as f:
                context["env_status"] = json.load(f)
        except: pass

    # Load Risks and Metrics
    if os.path.exists(args.risks):
        try:
            with open(args.risks, 'r') as f:
                all_analysis = json.load(f)
                
                # Context risks (all files with at least one severity risk)
                context["risks"] = sorted([(k, v["risks"]) for k, v in all_analysis.items() if v.get("risks")], key=lambda x: len(x[1]), reverse=True)
                context["risks_total"] = sum(len(v.get("risks", [])) for v in all_analysis.values())
                
                # Filtrar arquivos de teste das métricas de Top (Large e Complex)
                filtered_for_tops = {
                    k: v for k, v in all_analysis.items()
                    if not any(t_ind in k.lower() for t_ind in ["test", ".spec.", "/tests/"])
                }

                # Maior Arquivo
                context["top_large"] = _slice_list(sorted(
                    [{"file": k, "value": int(float(v.get("metrics", {}).get("lines", 0)))} for k, v in filtered_for_tops.items()],
                    key=lambda x: x["value"], reverse=True
                ), 0, 5)
                
                # Mais Complexo
                context["top_complex"] = _slice_list(sorted(
                    [{"file": k, "value": int(float(v.get("metrics", {}).get("complexity", 0)))} for k, v in filtered_for_tops.items()],
                    key=lambda x: x["value"], reverse=True
                ), 0, 5)
                
                # Maior Risco de Quebra: log2(complexity) * (100 - coverage)
                # Fórmula logarítmica suaviza o impacto de alta complexidade,
                # priorizando arquivos com baixa cobertura sobre os meramente complexos.
                import math
                
                def _find_coverage(fp_m, be_cov_m, fe_cov_m):
                    """Busca cobertura tentando múltiplas variações de caminho."""
                    cov_m = be_cov_m.get(str(fp_m), 0.0)
                    if not cov_m and ("backend-api/" in str(fp_m) or "src/backend-api/" in str(fp_m)):
                        cov_m = be_cov_m.get(str(fp_m).replace("backend-api/", "").replace("src/backend-api/", ""), 0.0)
                    if not cov_m:
                        cov_m = fe_cov_m.get(str(fp_m), 0.0)
                    if not cov_m:
                        # Fallback: busca por nome do arquivo em todas as chaves
                        bn_m = os.path.basename(str(fp_m))
                        for fk_m, fv_m in {**be_cov_m, **fe_cov_m}.items():
                            if os.path.basename(str(fk_m)) == bn_m and float(fv_m) > 0:
                                cov_m = float(fv_m)
                                break
                    return float(cov_m)
                
                breaking_risks = []
                for k_m, v_m in filtered_for_tops.items():
                    v_dict_m = cast(dict, v_m)
                    cov_m = _find_coverage(k_m, backend_file_coverage, frontend_file_coverage)
                    compl_m = float(v_dict_m.get("metrics", {}).get("complexity", 0))
                    
                    if compl_m <= 1:
                        continue
                    
                    log_compl_m = math.log2(max(compl_m, 2.0))
                    r_score_m = log_compl_m * (100.0 - cov_m)
                    if r_score_m > 0:
                        breaking_risks.append({"file": str(k_m), "value": int(r_score_m), "coverage": _round(cov_m)})
                
                context["top_breaking"] = _slice_list(sorted(breaking_risks, key=lambda x_m: int(cast(dict, x_m).get("value", 0)), reverse=True), 0, 5)

        except Exception as e: 
            print(f"Error processing risks: {e}")

    # Prepare sections structure
    # Config format: "source_directories": [{ "path": "...", "type": "backend", "name": "..." }]
    source_dirs_conf = project_config.get("source_directories", [])
    
    sections: list = []
    
    # Handle legacy config format (dict) just in case, or default to new.
    if isinstance(source_dirs_conf, dict):
         # Convert legacy to list for internal processing transparency
         for key, paths in cast(dict, source_dirs_conf).items():
            for p in cast(list, paths):
                sections.append({
                    "meta": {"path": str(p), "type": str(key), "name": f"{str(key).capitalize()} ({str(p)})"},
                    "tree": {}
                })
    elif isinstance(source_dirs_conf, list):
        for item in cast(list, source_dirs_conf):
             sections.append({
                 "meta": cast(dict, item),
                 "tree": {}
             })
    else:
        # Default fallback
        sections.append({"meta": {"path": "backend-api", "type": "backend", "name": "Backend"}, "tree": {}})
        sections.append({"meta": {"path": "src", "type": "frontend", "name": "Frontend"}, "tree": {}})
    
    # Map files to coverage URLs (Local HTML reports)
    coverage_url_map = {}
    
    # 1. Backend mapping (uses coverage.py status.json)
    ctx_dict_map = cast(dict, context)
    reports_map = cast(dict, ctx_dict_map.get("reports", {}))
    be_report_dir_name_c = os.path.dirname(str(reports_map.get("backend", "")))
    html_status_path_c = os.path.join(be_report_dir_name_c, "status.json")
    if os.path.exists(html_status_path_c):
        try:
            with open(html_status_path_c, 'r') as f:
                status_data_c = json.load(f)
                files_dict_c = cast(dict, status_data_c.get("files", {}))
                for key_c, val_c in files_dict_c.items():
                    val_dict_c = cast(dict, val_c)
                    idx_meta_c = cast(dict, val_dict_c.get("index", {}))
                    file_path_c = str(idx_meta_c.get("file", ""))
                    file_url_c = str(idx_meta_c.get("url", ""))
                    if file_path_c and file_url_c:
                        # Path relative to ci-dashboard/tmp/dashboard.html
                        coverage_url_map[file_path_c] = str(os.path.join("../../", be_report_dir_name_c, file_url_c))
        except Exception as e:
            print(f"⚠️ Error parsing backend status.json: {e}")
            
    # 2. Frontend mapping (NYC heuristic walk)
    fe_report_dir_for_walk = os.path.dirname(str(reports_map.get("frontend", "")))
    if os.path.exists(fe_report_dir_for_walk):
        for root, dirs, files in os.walk(fe_report_dir_for_walk):
            for file_f in files:
                file_str_f = str(file_f)
                if file_str_f.endswith(".js.html") or file_str_f.endswith(".gs.html"):
                    rel_html = str(os.path.relpath(os.path.join(root, file_str_f), fe_report_dir_for_walk))
                    source_key = rel_html.replace(".html", "")
                    
                    # Store absolute-ish paths relative to dashboard
                    url_f = str(os.path.join("../../", fe_report_dir_for_walk, rel_html))
                    coverage_url_map[source_key] = url_f
                    # NYC often uses paths relative to its root, so try variations
                    if not source_key.startswith("src/"):
                        coverage_url_map["src/" + source_key] = url_f
                    if "octaviano/" in source_key and not source_key.startswith("src/"):
                         coverage_url_map["src/" + source_key] = url_f

    # Get detailed coverage data for merging
    detailed_data = {}
    
    # 1. Backend (Pytest-cov JSON)
    if os.path.exists(args.backend_coverage):
        try:
            with open(args.backend_coverage, 'r') as f:
                data = json.load(f).get("files", {})
                # Normalize keys if needed (already relative in pytest-cov usually)
                for k, v in data.items():
                    detailed_data[k] = v
        except Exception as e:
            print(f"Error loading backend coverage: {e}")

    # 2. Frontend (NYC JSON Summary)
    if hasattr(args, 'frontend_coverage') and os.path.exists(args.frontend_coverage):
        try:
            with open(args.frontend_coverage, 'r') as f:
                data = json.load(f)
                coverage_items = cast(dict, data).items()
                for k_f, v_f in coverage_items:
                    if str(k_f) == "total": continue
                    
                    v_dict_f = cast(dict, v_f)
                    # NYC keys are absolute paths, make relative to project root
                    rel_path_f = str(k_f)
                    root_f = str(project_config.get("project_root", "."))
                    if root_f != "." and root_f in str(k_f): 
                        rel_path_f = os.path.relpath(str(k_f), root_f)
                    else:
                        cwd_f = os.getcwd()
                        if str(k_f).startswith(cwd_f):
                            rel_path_f = os.path.relpath(str(k_f), cwd_f)
                    
                    # Normalize format to match pytest-cov structure
                    line_sum_f = cast(dict, v_dict_f.get("lines", {}))
                    pct_f = float(str(line_sum_f.get("pct", 0.0)))
                    
                    cast(dict, detailed_data)[str(rel_path_f)] = {
                        "summary": {
                            "percent_covered": pct_f,
                            "total": int(float(str(line_sum_f.get("total", 0)))),
                            "covered": int(float(str(line_sum_f.get("covered", 0))))
                        },
                        "classes": {}, 
                        "functions": {} 
                    }
        except Exception as e:
             print(f"Error loading frontend coverage: {e}")

    # 3. Gateway (C8/Node JSON Summary)
    if hasattr(args, 'gateway_coverage') and os.path.exists(args.gateway_coverage):
        try:
            with open(args.gateway_coverage, 'r') as f:
                data = json.load(f)
                coverage_items_gw = cast(dict, data).items()
                for k_gw, v_gw in coverage_items_gw:
                    if str(k_gw) == "total": continue
                    
                    v_dict_gw = cast(dict, v_gw)
                    # C8 keys are absolute paths
                    rel_path_gw = str(k_gw)
                    root_gw = str(project_config.get("project_root", "."))
                    
                    if root_gw != "." and root_gw in str(k_gw):
                        rel_path_gw = os.path.relpath(str(k_gw), root_gw)
                    else:
                        cwd_gw = os.getcwd()
                        if str(k_gw).startswith(cwd_gw):
                            rel_path_gw = os.path.relpath(str(k_gw), cwd_gw)

                    # Normalize format
                    line_sum_gw = cast(dict, v_dict_gw.get("lines", {}))
                    pct_gw = float(str(line_sum_gw.get("pct", 0.0)))
                    
                    cast(dict, detailed_data)[str(rel_path_gw)] = {
                        "summary": {
                            "percent_covered": pct_gw,
                            "total": int(float(str(line_sum_gw.get("total", 0)))),
                            "covered": int(float(str(line_sum_gw.get("covered", 0))))
                        },
                        "classes": {}, 
                        "functions": {} 
                    }
        except Exception as e:
             print(f"Error loading gateway coverage: {e}")

    # Walk the project to find all source files
    walk_paths = [s["meta"]["path"] for s in sections]
    if not walk_paths:
        walk_paths = ["."]

    # Collect exclusion paths (test dirs) verify normalization
    test_exclude_paths = []
    test_dirs_conf = cast(dict, project_config.get("test_directories", {}))
    for cat, paths in test_dirs_conf.items():
        if isinstance(paths, list):
            test_exclude_paths.extend([os.path.normpath(p) for p in paths])

    # Extract repo url
    repo_url = project_config.get("repo_url")

    for root_path in walk_paths:
        if root_path == ".":
             # Legacy full walk behavior
            for root, dirs, files in os.walk("."):
                # Exclude known heavy/irrelevant dirs
                if any(exc in root for exc in [".git", "__pycache__", ".agent", "ci-dashboard", "node_modules", ".venv", "dist", "coverage", "testsprite_tests"]):
                    continue
                
                # Manual exclusion of test dirs for legacy walk
                if any(t_exc in os.path.normpath(root) for t_exc in test_exclude_paths):
                    continue

                for file in files:
                    _process_file_for_tree(root, file, detailed_data, coverage_url_map, sections, repo_url)
        
        else:
            # Directed walk from source dirs
            if not os.path.exists(root_path): continue
            
            for root, dirs, files in os.walk(root_path):
                 # Standard tech exclusions still apply
                if any(exc in root for exc in [".git", "__pycache__", ".agent", "ci-dashboard", "node_modules", ".venv", "dist", "coverage"]):
                    continue
                
                # Check if current root is inside a test directory
                norm_root = os.path.normpath(root)
                if any(norm_root.startswith(t_exc) or t_exc.startswith(norm_root) and len(norm_root) >= len(t_exc)  for t_exc in test_exclude_paths):
                     # Double check: if root STARTS with a test exclusion path
                     if any(norm_root == t_exc or norm_root.startswith(t_exc + os.sep) for t_exc in test_exclude_paths):
                         continue

                for file in files:
                    _process_file_for_tree(root, file, detailed_data, coverage_url_map, sections, repo_url)

    # Helper function to calculate package metrics recursively
    def calc_pkg_metrics(level_m: dict) -> dict:
        total_covered_p = 0
        total_statements_p = 0
        
        for name_p, node_p in level_m.items():
            node_dict_p = cast(dict, node_p)
            if str(node_dict_p.get("type")) == "pkg":
                metrics_p = calc_pkg_metrics(cast(dict, node_dict_p["children"]))
                node_dict_p["total_lines"] = metrics_p["total"]
                node_dict_p["covered_lines"] = metrics_p["covered"]
                if metrics_p["total"] > 0:
                    node_dict_p["percent"] = _round((float(metrics_p["covered"]) / float(metrics_p["total"])) * 100.0)
                else:
                    node_dict_p["percent"] = 0.0
            
            if str(node_dict_p.get("type")) == "file":
                total_statements_p += int(node_dict_p.get("total_lines", 0))
                total_covered_p += (int(node_dict_p.get("total_lines", 0)) - int(node_dict_p.get("missed_lines", 0)))
            else: # pkg
                total_statements_p += int(node_dict_p.get("total_lines", 0))
                total_covered_p += int(node_dict_p.get("covered_lines", 0))
        
        return {"total": total_statements_p, "covered": total_covered_p}


    
    # --- SORTING LOGIC ---
    # User Request: "traga o que tem mais linhas sem cobertura primeiro"
    # We need to recursively sort the tree.
    
    def sort_tree_recursive(level):
        # Flatten dict items to list for sorting
        items = []
        for name, node in level.items():
            # If pkg, recurse first to ensure children are sorted
            if node["type"] == "pkg":
                 # We also need to sum up missed_lines for packages to sort them correctly at this level too
                 # BUT, for now, let's just sort files within packages. 
                 # To fully sort packages by missed lines, we need to propagate the count up.
                 node["missed_lines"] = _calculate_pkg_missed(node["children"])
                 node["children"] = sort_tree_recursive(node["children"])
            
            items.append((name, node))
            
        # Sort Key: 
        # 1. Missed Lines (Descending)
        # 2. Percent (Ascending - lower coverage first)
        def sort_key(item):
            name, node = item
            missed = node.get("missed_lines", 0)
            pct = node.get("percent", 100)
            return (-missed, pct)
            
        # Sort and reconstruct dict (Python 3.7+ keeps insertion order)
        sorted_items = sorted(items, key=sort_key)
        return {k: v for k, v in sorted_items}

    def _calculate_pkg_missed(children):
        total = 0
        for node in children.values():
            if node["type"] == "file":
                total += node.get("missed_lines", 0)
            elif node["type"] == "pkg":
                 total += _calculate_pkg_missed(node["children"])
        return total

    # Calculate metrics and Sort for each section
    for section_s in sections:
        section_tree_s = cast(dict, section_s["tree"])
        if section_tree_s:
            metrics_s = calc_pkg_metrics(section_tree_s)
            section_s["tree"] = sort_tree_recursive(section_tree_s)
            if metrics_s["total"] > 0:
                section_s["meta"]["percent"] = _round((float(metrics_s["covered"]) / float(metrics_s["total"])) * 100.0)
            else:
                section_s["meta"]["percent"] = 0.0

    context["sections"] = sections

    # Render Main Dashboard
    template_path = os.path.join(os.path.dirname(__file__), "..", "templates", "dashboard.html")
    html = render_template(template_path, context)
    with open(args.output, 'w', encoding='utf-8') as f:
        f.write(html)
        
    # Render Tests List Page
    tests_template_path = os.path.join(os.path.dirname(__file__), "..", "templates", "tests_list.html")
    if os.path.exists(tests_template_path):
        tests_html = render_template(tests_template_path, context)
        tests_output = os.path.join(os.path.dirname(args.output), "tests_list.html")
        with open(tests_output, 'w', encoding='utf-8') as f:
            f.write(tests_html)
        print(f"Tests list generated: {os.path.abspath(tests_output)}")

    previous_render_test = os.path.join(os.path.dirname(args.output), "tests_list.html")

    # Render Coverage List Page
    coverage_template_path = os.path.join(os.path.dirname(__file__), "..", "templates", "coverage_list.html")
    if os.path.exists(coverage_template_path):
        cov_html = render_template(coverage_template_path, context)
        cov_output = os.path.join(os.path.dirname(args.output), "coverage_list.html")
        with open(cov_output, 'w', encoding='utf-8') as f:
            f.write(cov_html)
        print(f"Coverage list generated: {os.path.abspath(cov_output)}")

    # Save consolidated JSON for LLM consumption
    llm_json_path = args.output.replace(".html", ".json")
    with open(llm_json_path, 'w', encoding='utf-8') as f:
        json.dump(context, f, indent=2)
    
    # Save data.js for frontend isolation (avoids CORS)
    data_js_path = os.path.join(os.path.dirname(args.output), "data.js")
    with open(data_js_path, 'w', encoding='utf-8') as f:
        f.write(f"window.dashboardData = {json.dumps(context, indent=2)};")
        
    print(f"Dashboard generated: {os.path.abspath(args.output)}")
    print(f"LLM Data generated: {os.path.abspath(llm_json_path)}")
    print(f"JS Data generated (for frontend): {os.path.abspath(data_js_path)}")
    
    # --- POST-PROCESS COVERAGE REPORTS (Premium UI Injection) ---
    print("\n💎 Beautifying Coverage Reports...")
    premium_css_path = os.path.join(os.path.dirname(__file__), "..", "templates", "coverage_premium.css")
    if os.path.exists(premium_css_path):
        with open(premium_css_path, 'r', encoding='utf-8') as f:
            premium_css = f.read()
        
        style_tag = f"\n<style>\n{premium_css}\n</style>\n</head>"
        
        reports_dict = cast(dict, context.get("reports", {}))
        be_dir = os.path.dirname(str(reports_dict.get("backend", "")))
        fe_dir = os.path.dirname(str(reports_dict.get("frontend", "")))
        gw_dir = os.path.dirname(str(reports_dict.get("gateway", "")))
        
        processed_count = 0
        for report_dir in [be_dir, fe_dir, gw_dir]:
            if not report_dir or not os.path.exists(str(report_dir)): continue
            for root, dirs, files in os.walk(str(report_dir)):
                for file in files:
                    file_str = str(file)
                    if file_str.endswith(".html"):
                        fpath = os.path.join(root, file_str)
                        try:
                            with open(fpath, 'r', encoding='utf-8') as f:
                                content = str(f.read())
                            
                            if "</head>" in content and "/* Coverage Premium UI" not in content:
                                new_content = str(content).replace("</head>", str(style_tag))
                                with open(fpath, 'w', encoding='utf-8') as f:
                                    f.write(new_content)
                                processed_count = int(cast(int, processed_count)) + 1
                        except Exception as e:
                            print(f"Error processing {file_str}: {e}")
        
        print(f"✅ {processed_count} reports beautified with Premium UI.")
        
        # --- NEW: Copy Premium CSS for Main Templates ---
        # This allows dashboard.html and lists to reference it externally
        try:
            dest_css = os.path.join(os.path.dirname(args.output), "coverage_premium.css")
            shutil.copy(premium_css_path, dest_css)
            print(f"🔹 Global Premium CSS copied to: {dest_css}")
        except Exception as e:
            print(f"⚠️ Failed to copy global CSS: {e}")
            
    else:
        print("⚠️ Premium CSS not found. Skipping beautification.")
    
    # --- HEALTH CHECKLIST ---
    print("\n" + "="*50)
    print("✅ CHECKLIST DE INTEGRIDADE DO DASHBOARD")
    print("="*50)
    
    # 1. Consistency Check
    tests_summary_c = cast(dict, cast(dict, context).get("tests", {}))
    summary_total_c = int(float(str(tests_summary_c.get("total", 0))))
    list_total_c = int(float(str(context.get("total_tests_count", 0))))
    
    if summary_total_c == list_total_c:
         print(f"🔹 Contagem de Testes: OK ({summary_total_c})")
    else:
         print(f"❌ Contagem de Testes: ERRO (Resumo: {summary_total_c} vs Lista: {list_total_c})")
    
    # 2. Environments Check
    env = cast(dict, context.get("env_status", {}))
    stg = cast(dict, env.get("stg", {}))
    prod = cast(dict, env.get("prod", {}))
    
    if str(stg.get("status", "")) == "UP":
        print(f"🔹 Ambiente STG: OK ({stg.get('version', 'v?')})")
    else:
        print(f"⚠️ Ambiente STG: N/A ou DOWN")
        
    if str(prod.get("status", "")) == "UP":
        print(f"🔹 Ambiente PROD: OK ({prod.get('version', 'v?')})")
    else:
        print(f"⚠️ Ambiente PROD: N/A ou DOWN")
        
    # 3. Coverage Check
    cov_block = cast(dict, context.get("coverage", {}))
    total_cov = float(cov_block.get("total", 0.0))
    be_cov = float(cov_block.get("backend", 0.0))
    fe_cov = float(cov_block.get("frontend", 0.0))
    gw_cov = float(cov_block.get("gateway", 0.0))
    
    if float(total_cov) > 0:
        print(f"🔹 Cobertura Total: OK ({total_cov}%) [BE: {be_cov}% | FE: {fe_cov}% | GW: {gw_cov}%]")
    else:
        print(f"⚠️ Cobertura Total: 0% (Verifique os arquivos de coverage)")
        
    # 4. Result Files Check
    files_checked = []
    if os.path.exists(args.test_results): files_checked.append("BE")
    if os.path.exists(args.frontend_results): files_checked.append("FE")
    if os.path.exists(args.gateway_results): files_checked.append("GW")
    
    # 5. Integrity Check for Specific Paths
    integrity: list = []
    for path_m in ["src/tools", "src/apps-script", "src/ops"]:
        cov_m = backend_file_coverage.get(path_m) or frontend_file_coverage.get(path_m)
        if cov_m is not None:
            integrity.append(f"{path_m}: {cov_m}%")
        else:
            # Fallback check
            integrity.append(f"{path_m}: [Check Coverage Detail]")

    print(f"🔹 Fontes de Dados: {', '.join(files_checked)}")
    print(f"🔹 Integridade: {', '.join(integrity)}")
    
    print("="*50 + "\n")

if __name__ == "__main__":
    main()
