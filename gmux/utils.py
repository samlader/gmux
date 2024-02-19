import os
import subprocess
import time

from jinja2 import Template

from gmux.config import DEFAULT_PR_TEMPLATE_NAME


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
