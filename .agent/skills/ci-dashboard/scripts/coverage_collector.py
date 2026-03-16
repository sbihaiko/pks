import subprocess
import json
import os
import argparse

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="ci-dashboard/tmp/coverage_summary.json")
    args = parser.parse_args()
    
    coverage_data = {"backend": 0.0, "frontend": 0.0, "gateway": 0.0}

    # Backend (Rust/Tarpaulin or Python/Pytest-cov)
    be_cov_path = "ci-dashboard/tmp/backend_coverage.json"
    tarpaulin_path = "ci-dashboard/tmp/tarpaulin-report.json"
    
    if os.path.exists(be_cov_path):
        try:
             with open(be_cov_path, 'r') as f:
                data = json.load(f)
                if "totals" in data:
                     coverage_data["backend"] = round(data["totals"].get("percent_covered", 0), 2)
        except Exception as e:
            print(f"Error parsing Backend coverage (pytest): {e}")

    # Fallback to Tarpaulin for Rust
    if coverage_data["backend"] == 0.0 and os.path.exists(tarpaulin_path):
        try:
            with open(tarpaulin_path, 'r') as f:
                data = json.load(f)
                if "files" in data:
                    total_covered = sum(f["covered"] for f in data["files"])
                    total_coverable = sum(f["coverable"] for f in data["files"])
                    if total_coverable > 0:
                        coverage_data["backend"] = round((total_covered / total_coverable) * 100, 2)
        except Exception as e:
            print(f"Error parsing Tarpaulin report: {e}")

    # Frontend
    fe_cov_path = "ci-dashboard/tmp/frontend_coverage.json"
    if os.path.exists(fe_cov_path):
        try:
             with open(fe_cov_path, 'r') as f:
                data = json.load(f)
                pct = data.get("total", {}).get("lines", {}).get("pct", 0)
                coverage_data["frontend"] = round(float(pct), 2)
        except Exception as e:
            print(f"Error parsing Frontend coverage: {e}")

    # Gateway Coverage
    gw_cov_path = "ci-dashboard/tmp/gateway_coverage.json"
    if os.path.exists(gw_cov_path):
        try:
             with open(gw_cov_path, 'r') as f:
                data = json.load(f)
                pct_val = data.get("total", {}).get("lines", {}).get("pct", 0)
                coverage_data["gateway"] = round(float(pct_val), 2)
        except Exception as e:
            print(f"Error parsing Gateway coverage: {e}")

    summary = {
        "backend": {
            "percentage": round(coverage_data["backend"], 1),
            "html_report": "coverage/backend_html/index.html"
        },
        "frontend": {
            "percentage": round(coverage_data["frontend"], 1),
            "html_report": "coverage/frontend_html/index.html"
        },
        "gateway": {
            "percentage": round(coverage_data["gateway"], 1),
            "html_report": "ci-dashboard/tmp/gateway_coverage/index.html"
        }
    }
    
    with open(args.output, 'w') as f:
        json.dump(summary, f, indent=2)
    print(f"Coverage summary saved to {args.output}")

if __name__ == "__main__":
    main()
