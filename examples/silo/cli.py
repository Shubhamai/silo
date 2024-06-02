import argparse
import importlib.util


def run_script(script_path):
    spec = importlib.util.spec_from_file_location("script", script_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    if hasattr(module, "main"):
        module.main()
    else:
        pass
        # print("No 'main' function found in the script.")


def main():
    parser = argparse.ArgumentParser(description="Silo CLI")
    subparsers = parser.add_subparsers(dest="command")

    run_parser = subparsers.add_parser("launch", help="Run a Python script")
    run_parser.add_argument("script", type=str, help="Path to the Python script")

    args = parser.parse_args()

    if args.command == "launch":
        run_script(args.script)


if __name__ == "__main__":
    main()
