import json
import os
import re
import sys
import click
from jinja2 import Template
import subprocess
import time
import concurrent.futures


@click.group()
def gmux():
    pass


@gmux.command()
@click.argument("directory_arg", required=False)
@click.option("--directory", required=False)
def init(directory_arg, directory):
    """
    Initialize a new directory for gmux.

    Args:
        directory_arg (str): Directory name provided as an argument.
        directory_name_opt (str): Directory name provided as an option.
    """
    directory_name = directory_arg or directory or "gmux"

    os.makedirs(directory_name, exist_ok=True)

    with open(f"{directory_name}/PR_TEMPLATE.md", "w") as f:
        f.write(
            """## Overview\n\nDescription for {{ repository_name }}\n\n## Changes\n\n-"""
        )

    print(f"\033[92m✨ gmux successfully initialised! ✨\033[0m")
    print(f"Change your directory to `{directory_name}` to begin...")


@gmux.command()
@click.option("--org", required=True)
@click.option("--filter", required=False)
def clone(org, filter):
    """
    Clone repositories from a specified organization or user.

    Args:
        org (str): Organization or user name.
        filter (str): Regex filter for repository names.
    """
    repository_names = json.loads(
        os.popen(f"gh repo list {org} --json name  --limit 9999").read()
    )

    processes = []

    for repository in repository_names:
        if filter and not re.match(filter, repository["name"]):
            continue

        if os.path.isdir(repository["name"]):
            print(f"Skipping {org}/{repository['name']}, already exists")
            continue

        print(f"cloning {org}/{repository['name']}")

        process = subprocess.Popen(
            f'gh repo clone {org}/{repository["name"]} -- --depth=1', shell=True
        )
        processes.append(process)

    successful_repositories = []
    failed_repositories = []

    for process in processes:
        if process.wait() == 0:
            successful_repositories.append(process)
        else:
            failed_repositories.append(process)

    print(f"\033[92mCloned {len(successful_repositories)} repositories\033[0m")

    if failed_repositories:
        print(f"\033[92m{len(processes)} failed\033[0m")


@gmux.command()
@click.option("--filter", required=False)
@click.argument("cmd", nargs=-1, type=click.UNPROCESSED)
def cmd(filter, cmd):
    """
    Run a command in each repository.
    """
    return_codes = []

    def run_cmd(folder):
        print(f"\033[97m{folder}\033[0m {' '.join(cmd)}")
        start_time = time.time()
        result = subprocess.run(" ".join(cmd), shell=True, cwd=folder)
        elapsed_time = time.time() - start_time
        return_codes.append(result.returncode)
        return_code_color = "91" if result.returncode != 0 else "37"
        print(
            f"\033[{return_code_color}mreturn code {result.returncode} (elapsed time: {elapsed_time:.2f} seconds)\033[0m"
        )
        return result

    results = _for_each_repository(filter, run_cmd)

    if set([result.returncode for result in results]) != set([0]):
        sys.exit(1)


@gmux.command()
@click.option("--filter", required=False)
def status(filter):
    """
    Retrieve status for every repository.
    """

    def get_status(folder):
        branch_name = subprocess.check_output(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=folder, text=True
        ).strip()

        commit_ref = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=folder, text=True
        ).strip()

        print(
            f"\033[97m{folder}\033[0m \033[37m{branch_name} ({commit_ref[0:6]})\033[0m"
        )

        subprocess.run(["git", "status", "-s"], cwd=folder)

    _for_each_repository(filter, get_status)


@gmux.command()
@click.option("--filter", required=False)
@click.option("--title", prompt=True)
def pr(filter, title):
    """
    Create a pull request for each repository.

    Args:
        title (str): Title for the Pull Request.
    """
    template_path = "PR_TEMPLATE.md"
    if not os.path.isfile(template_path):
        click.echo("PR template not found. Run 'gmux init' first.")
        return

    with open(template_path, "r") as f:
        template_content = f.read()

    template = Template(template_content)

    def create_pr(folder):
        base_branch = subprocess.check_output(
            "git remote show origin | sed -n '/HEAD branch/s/.*: //p'",
            shell=True,
            cwd=folder,
            text=True,
        ).strip()

        diff_output = subprocess.check_output(
            f"git diff --name-only origin/{base_branch}",
            shell=True,
            cwd=folder,
            text=True,
        )

        diff_files = diff_output.strip().split("\n")

        if not diff_files:
            return

        pr_content = template.render(
            title=title, diff_files=diff_files, repository_name=folder
        )

        subprocess.run(
            f'gh pr create -w --title "{title}" --body "{pr_content}"',
            shell=True,
            cwd=folder,
        )

    _for_each_repository(filter, create_pr)


@gmux.command(
    context_settings=dict(
        ignore_unknown_options=True,
    )
)
@click.argument("git_command", nargs=-1)
@click.option("--filter", required=False)
def git(git_command, filter):
    """
    Run any Git command (with magic variables) for all repositories.
    """

    # TODO: Clean this whole mess up

    def run_git_command(folder):
        try:
            magic_variables = {
                "@default": subprocess.check_output(
                    "git remote show origin | sed -n '/HEAD branch/s/.*: //p'",
                    shell=True,
                    cwd=folder,
                    text=True,
                ).strip(),
                "@current": subprocess.check_output(
                    "git rev-parse --abbrev-ref HEAD",
                    shell=True,
                    cwd=folder,
                    text=True,
                ).strip(),
            }

            cmd = [magic_variables.get(arg, arg) for arg in git_command if arg]

            current_branch = subprocess.run(
                "git rev-parse --abbrev-ref HEAD",
                shell=True,
                cwd=folder,
                capture_output=True,
                text=True,
            ).stdout.strip()

            start_time = time.time()

            result = subprocess.run(
                ["git", *cmd],
                shell=False,
                cwd=folder,
                text=True,
                capture_output=True,
            )

            elapsed_time = time.time() - start_time

            return_code_color = "91" if result.returncode != 0 else "37"

            print(
                f"\033[97m{folder} ({current_branch})\033[0m git {' '.join(cmd)}\n"
                f"{result.stdout}"
                f"\033[{return_code_color}mreturn code {result.returncode} (elapsed time: {elapsed_time:.2f} seconds)\033[0m"
            )

        except Exception as e:
            print(f"Error for {folder}:\n \033[93m{e}\033[0m")

    with concurrent.futures.ThreadPoolExecutor() as executor:
        folders = [
            folder
            for folder in os.listdir(".")
            if os.path.isdir(f"{folder}/.git")
            and (not filter or re.match(filter, folder))
        ]
        executor.map(run_git_command, folders)


def _for_each_repository(filter, function):
    result = []
    for folder in os.listdir("."):
        if not os.path.isdir(f"{folder}/.git"):
            continue
        if filter and not re.match(filter, folder):
            continue
        if os.path.isdir(folder):
            try:
                result.append(function(folder))
            except Exception as e:
                print(f"Error for {folder}:\n \033[93m{e}\033[0m")

    return result


if __name__ == "__main__":
    gmux()
