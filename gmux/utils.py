from concurrent.futures import ThreadPoolExecutor
import os
import subprocess
import time

from jinja2 import Template

from gmux.config import DEFAULT_PR_TEMPLATE_NAME
from gmux.helper import is_git_directory


def run_command(
    command, cwd=None, text=False, capture_output=False, log_metadata=False
):
    start_time = time.time()
    result = subprocess.run(command, cwd=cwd, text=text, capture_output=capture_output)
    elapsed_time = time.time() - start_time

    if log_metadata:
        print(
            f"\033[{'91' if result.returncode != 0 else '37'}mreturn code {result.returncode} (elapsed time: {elapsed_time:.2f} seconds)\033[0m"
        )

    return result


def get_template(template_path=None):
    if not template_path:
        template_path = DEFAULT_PR_TEMPLATE_NAME

    if not os.path.isfile(template_path):
        return

    with open(template_path, "r") as f:
        template_content = f.read()

    return Template(template_content)


def _for_each_repository(function, filter=None, parallel=False, *args, **kwargs):
    folders = [
        folder
        for folder in os.listdir(".")
        if is_git_directory(folder) and (not filter or re.match(filter, folder))
    ]

    if parallel:
        with ThreadPoolExecutor() as executor:
            return executor.map(function, folders, *args, **kwargs)

    result = []

    for folder in folders:
        try:
            result.append(function(folder, *args, **kwargs))
        except Exception as e:
            print(f"Error for {folder}:\n \033[93m{e}\033[0m")

    return result
